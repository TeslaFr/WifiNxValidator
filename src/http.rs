use crate::dns::HOSTS;

pub fn respond(method: &str, host: &str, path: &str) -> Vec<u8> {
    let (name, valid_port) = match host.split_once(':') {
        Some((name, port)) => (
            name,
            !port.is_empty()
                && port.bytes().all(|b| b.is_ascii_digit())
                && port.parse::<u16>().is_ok_and(|n| n > 0),
        ),
        None => (host, true),
    };
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    let (status, body) = if !matches!(method, "GET" | "HEAD") {
        ("405 Method Not Allowed", "method not allowed")
    } else if valid_port && HOSTS.contains(&name.as_str()) && path == "/" {
        ("200 OK", "ok")
    } else {
        ("404 Not Found", "not found")
    };
    let mut result = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if status == "200 OK" {
        result.push_str("X-Organization: Nintendo\r\n");
    }
    if status.starts_with("405") {
        result.push_str("Allow: GET, HEAD\r\n");
    }
    result.push_str("\r\n");
    if method != "HEAD" {
        result.push_str(body);
    }
    result.into_bytes()
}
