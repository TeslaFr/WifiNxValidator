use crate::{
    config::Config,
    dns,
    history::{Entry, History},
    http,
};
use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
const WAIT: Duration = Duration::from_millis(50);
const DEADLINE: Duration = Duration::from_secs(3);
const MAX_CLIENTS: usize = 64;

pub struct Server {
    stop: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    clients: Arc<AtomicUsize>,
    task: Option<JoinHandle<()>>,
    dns: SocketAddr,
    http: SocketAddr,
}
impl Server {
    pub fn start(config: Config, history: History) -> Result<Self, String> {
        config.validate()?;
        let udp = UdpSocket::bind((config.listen, config.dns_port))
            .map_err(|e| format!("DNS UDP : {e}"))?;
        let dns = udp.local_addr().map_err(|e| e.to_string())?;
        let tcp = TcpListener::bind(dns).map_err(|e| format!("DNS TCP : {e}"))?;
        let web = TcpListener::bind((config.listen, config.http_port))
            .map_err(|e| format!("HTTP : {e}"))?;
        let http = web.local_addr().map_err(|e| e.to_string())?;
        udp.set_nonblocking(true).map_err(|e| e.to_string())?;
        tcp.set_nonblocking(true).map_err(|e| e.to_string())?;
        web.set_nonblocking(true).map_err(|e| e.to_string())?;
        let stop = Arc::new(AtomicBool::new(false));
        let running = Arc::new(AtomicBool::new(true));
        let clients = Arc::new(AtomicUsize::new(0));
        let client_count = clients.clone();
        let cancel = stop.clone();
        let alive = running.clone();
        let task = thread::Builder::new()
            .name("wifinx-listeners".into())
            .spawn(move || {
                let mut workers: Vec<JoinHandle<()>> = Vec::new();
                let mut buf = [0; 4096];
                'serve: while !cancel.load(Ordering::Relaxed) {
                    // Bound each batch so UDP traffic cannot starve TCP or shutdown.
                    for _ in 0..64 {
                        match udp.recv_from(&mut buf) {
                            Ok((n, peer)) => {
                                let packet = &buf[..n];
                                let response = dns::respond(packet, config.answer);
                                let result = match &response {
                                    Ok(r) => match udp.send_to(r, peer) {
                                        Ok(_) => dns_result(r),
                                        Err(e) => e.to_string(),
                                    },
                                    Err(e) => e.clone(),
                                };
                                log_dns(&history, peer, "DNS UDP", packet, &result);
                            }
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(e) => {
                                history.push(Entry::new(
                                    "serveur",
                                    "DNS UDP",
                                    "",
                                    "ERREUR",
                                    &e.to_string(),
                                ));
                                break 'serve;
                            }
                        }
                    }
                    for (listener, is_http) in [(&tcp, false), (&web, true)] {
                        for _ in 0..16 {
                            match listener.accept() {
                                Ok((stream, peer)) => {
                                    if workers.len() >= MAX_CLIENTS {
                                        history.push(Entry::new(
                                            &peer.to_string(),
                                            "TCP",
                                            "",
                                            "REFUS",
                                            "Limite de connexions",
                                        ));
                                        continue;
                                    }
                                    let stop = cancel.clone();
                                    let h = history.clone();
                                    let address = config.answer;
                                    let client_count = client_count.clone();
                                    match thread::Builder::new().name("wifinx-client".into()).spawn(
                                        move || {
                                            client_count.fetch_add(1, Ordering::Relaxed);
                                            let _active = ActiveClient(client_count);
                                            if let Err(e) =
                                                handle(stream, peer, is_http, address, &stop, &h)
                                            {
                                                h.push(Entry::new(
                                                    &peer.to_string(),
                                                    if is_http { "HTTP" } else { "DNS TCP" },
                                                    "",
                                                    "ERREUR",
                                                    &e.to_string(),
                                                ));
                                            }
                                        },
                                    ) {
                                        Ok(t) => workers.push(t),
                                        Err(e) => history.push(Entry::new(
                                            "serveur",
                                            "TCP",
                                            "",
                                            "ERREUR",
                                            &e.to_string(),
                                        )),
                                    }
                                }
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                                Err(e) => {
                                    history.push(Entry::new(
                                        "serveur",
                                        "TCP",
                                        "",
                                        "ERREUR",
                                        &e.to_string(),
                                    ));
                                    break 'serve;
                                }
                            }
                        }
                    }
                    let mut i = 0;
                    while i < workers.len() {
                        if workers[i].is_finished() {
                            let _ = workers.swap_remove(i).join();
                        } else {
                            i += 1;
                        }
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                cancel.store(true, Ordering::Relaxed);
                for worker in workers {
                    let _ = worker.join();
                }
                alive.store(false, Ordering::Relaxed);
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            stop,
            running,
            clients,
            task: Some(task),
            dns,
            http,
        })
    }
    pub fn dns_addr(&self) -> SocketAddr {
        self.dns
    }
    pub fn http_addr(&self) -> SocketAddr {
        self.http
    }
    pub fn active_connections(&self) -> usize {
        self.clients.load(Ordering::Relaxed)
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.task.take() {
            let _ = t.join();
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}
fn dns_result(r: &[u8]) -> String {
    if r[3] & 15 == 5 {
        "REFUSED".into()
    } else if r[7] == 0 {
        "NODATA".into()
    } else {
        "A local".into()
    }
}
fn log_dns(h: &History, peer: SocketAddr, protocol: &str, p: &[u8], result: &str) {
    let (name, kind) = dns::question(p)
        .map(|(n, k, _)| {
            (
                n,
                match k {
                    1 => "A".into(),
                    28 => "AAAA".into(),
                    _ => k.to_string(),
                },
            )
        })
        .unwrap_or(("Paquet invalide".into(), "?".into()));
    h.push(Entry::new(
        &peer.to_string(),
        protocol,
        &name,
        &kind,
        result,
    ));
}
fn interrupted(stop: &AtomicBool, until: Instant) -> io::Result<()> {
    if stop.load(Ordering::Relaxed) || Instant::now() >= until {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Requête annulée ou délai dépassé",
        ))
    } else {
        Ok(())
    }
}
fn read_exact(
    stream: &mut TcpStream,
    mut buf: &mut [u8],
    stop: &AtomicBool,
    until: Instant,
) -> io::Result<()> {
    while !buf.is_empty() {
        interrupted(stop, until)?;
        match stream.read(buf) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => buf = &mut buf[n..],
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
fn write_all(
    stream: &mut TcpStream,
    mut buf: &[u8],
    stop: &AtomicBool,
    until: Instant,
) -> io::Result<()> {
    while !buf.is_empty() {
        interrupted(stop, until)?;
        match stream.write(buf) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => buf = &buf[n..],
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
fn handle(
    mut stream: TcpStream,
    peer: SocketAddr,
    is_http: bool,
    address: std::net::Ipv4Addr,
    stop: &AtomicBool,
    h: &History,
) -> io::Result<()> {
    stream.set_read_timeout(Some(WAIT))?;
    stream.set_write_timeout(Some(WAIT))?;
    let until = Instant::now() + DEADLINE;
    if is_http {
        let mut data = Vec::new();
        let mut byte = [0];
        while !data.ends_with(b"\r\n\r\n") {
            if data.len() >= 8192 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "En-têtes HTTP trop longs",
                ));
            }
            read_exact(&mut stream, &mut byte, stop, until)?;
            data.push(byte[0]);
        }
        let text = std::str::from_utf8(&data).map_err(|_| io::ErrorKind::InvalidData)?;
        let mut lines = text.split("\r\n");
        let parts: Vec<_> = lines.next().unwrap_or("").split_whitespace().collect();
        if parts.len() != 3 || !matches!(parts[2], "HTTP/1.1" | "HTTP/1.0") {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let hosts: Vec<_> = lines
            .filter_map(|l| l.split_once(':'))
            .filter(|(k, _)| k.eq_ignore_ascii_case("host"))
            .map(|(_, v)| v.trim())
            .collect();
        if hosts.len() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Un en-tête Host est requis",
            ));
        }
        let response = http::respond(parts[0], hosts[0], parts[1]);
        let status = std::str::from_utf8(&response)
            .unwrap_or("")
            .lines()
            .next()
            .unwrap_or("");
        write_all(&mut stream, &response, stop, until)?;
        h.push(Entry::new(
            &peer.to_string(),
            "HTTP",
            &format!("{}{}", hosts[0], parts[1]),
            parts[0],
            status,
        ));
    } else {
        // Support multiple framed queries on one DNS TCP connection.
        loop {
            let mut size = [0; 2];
            read_exact(&mut stream, &mut size, stop, until)?;
            let len = u16::from_be_bytes(size) as usize;
            if !(12..=4096).contains(&len) {
                return Err(io::ErrorKind::InvalidData.into());
            }
            let mut packet = vec![0; len];
            read_exact(&mut stream, &mut packet, stop, until)?;
            let response = dns::respond(&packet, address)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            write_all(
                &mut stream,
                &(response.len() as u16).to_be_bytes(),
                stop,
                until,
            )?;
            write_all(&mut stream, &response, stop, until)?;
            log_dns(h, peer, "DNS TCP", &packet, &dns_result(&response));
        }
    }
    Ok(())
}

struct ActiveClient(Arc<AtomicUsize>);
impl Drop for ActiveClient {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}
