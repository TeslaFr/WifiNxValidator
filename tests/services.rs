use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream, UdpSocket},
    time::{Duration, Instant},
};
use wifinx_validator::{
    config::Config,
    history::{Entry, History},
    server::Server,
};
fn config() -> Config {
    Config {
        listen: Ipv4Addr::LOCALHOST,
        answer: Ipv4Addr::new(192, 168, 50, 2),
        dns_port: 0,
        http_port: 0,
    }
}
fn query() -> Vec<u8> {
    let mut p = vec![0, 42, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in "ctest.cdn.nintendo.net".split('.') {
        p.push(label.len() as u8);
        p.extend(label.as_bytes());
    }
    p.extend([0, 0, 1, 0, 1]);
    p
}
#[test]
fn actual_udp_tcp_http_and_restart() {
    let h = History::default();
    let mut server = Server::start(config(), h.clone()).unwrap();
    let dns = server.dns_addr();
    let http = server.http_addr();
    let udp = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    udp.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    udp.send_to(&query(), dns).unwrap();
    let mut b = [0; 1024];
    let n = udp.recv(&mut b).unwrap();
    assert_eq!(&b[n - 4..n], &[192, 168, 50, 2]);
    let mut tcp = TcpStream::connect(dns).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let q = query();
    tcp.write_all(&(q.len() as u16).to_be_bytes()).unwrap();
    tcp.write_all(&q).unwrap();
    let mut size = [0; 2];
    tcp.read_exact(&mut size).unwrap();
    let mut reply = vec![0; u16::from_be_bytes(size) as usize];
    tcp.read_exact(&mut reply).unwrap();
    assert_eq!(&reply[reply.len() - 4..], &[192, 168, 50, 2]);
    for method in ["GET", "HEAD"] {
        let mut c = TcpStream::connect(http).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        write!(
            c,
            "{method} / HTTP/1.1\r\nHost: ctest.cdn.nintendo.net\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        c.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.ends_with(if method == "GET" {
            "\r\n\r\nok"
        } else {
            "\r\n\r\n"
        }));
    }
    let idle_until = Instant::now() + Duration::from_secs(1);
    while server.active_connections() != 1 && Instant::now() < idle_until {
        std::thread::yield_now();
    }
    assert_eq!(server.active_connections(), 1);
    let mut slow = TcpStream::connect(http).unwrap();
    slow.write_all(b"GET / HTTP/1.1\r\nHost: ctest.cdn.nintendo.net\r\n")
        .unwrap();
    // The answered DNS stream remains active; wait until HTTP is also accepted.
    let accepted_until = Instant::now() + Duration::from_secs(1);
    while server.active_connections() < 2 && Instant::now() < accepted_until {
        std::thread::yield_now();
    }
    assert_eq!(server.active_connections(), 2);

    let start = Instant::now();
    server.stop();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(!server.is_running());
    assert!(h.snapshot().len() >= 4);
    let mut cfg = config();
    cfg.dns_port = dns.port();
    cfg.http_port = http.port();
    let mut again = Server::start(cfg, h).unwrap();
    again.stop();
}
#[test]
fn failed_start_releases_dns() {
    let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    for _ in 0..10 {
        let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = probe.local_addr().unwrap().port();
        let Ok(udp_probe) = UdpSocket::bind((Ipv4Addr::LOCALHOST, port)) else {
            continue;
        };
        drop(udp_probe);
        drop(probe);
        let mut c = config();
        c.dns_port = port;
        c.http_port = occupied.local_addr().unwrap().port();
        let error = match Server::start(c, History::default()) {
            Ok(_) => panic!("HTTP conflict was ignored"),
            Err(e) => e,
        };
        if error.starts_with("DNS") {
            continue;
        } // Another process won the reservation race.
        assert!(error.starts_with("HTTP :"), "{error}");
        assert!(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok());
        assert!(UdpSocket::bind((Ipv4Addr::LOCALHOST, port)).is_ok());
        return;
    }
    panic!("Could not reserve a DNS port for the HTTP conflict test");
}
#[test]
fn bounded_history_and_config() {
    let h = History::default();
    for i in 0..5002 {
        h.push(Entry::new("client", "DNS", &i.to_string(), "A", "OK"));
    }
    let s = h.snapshot();
    assert_eq!(s.len(), 5000);
    assert_eq!(s[0].target, "2");
    h.clear();
    assert!(h.snapshot().is_empty());
    let mut c = config();
    c.answer = Ipv4Addr::UNSPECIFIED;
    assert!(c.validate().is_err());
}
