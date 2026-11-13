use std::net::Ipv4Addr;
use wifinx_validator::{dns, http};
fn packet(name: &str, kind: u16) -> Vec<u8> {
    let mut p = vec![0, 42, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in name.trim_end_matches('.').split('.') {
        p.push(label.len() as u8);
        p.extend(label.as_bytes());
    }
    p.push(0);
    p.extend(kind.to_be_bytes());
    p.extend([0, 1]);
    p
}
#[test]
fn local_dns() {
    for n in [
        "ctest.p01.ctest.srv.nintendo.net",
        "ctest-ipv6.p01.ctest.srv.nintendo.net",
        "ctest.cdn.nintendo.net",
        "conntest.nintendowifi.net",
        "CTEST.CDN.NINTENDO.NET.",
        "sub.ctest.cdn.nintendo.net",
    ] {
        let r = dns::respond(&packet(n, 1), Ipv4Addr::new(192, 168, 1, 5)).unwrap();
        assert_eq!(&r[..2], &[0, 42]);
        assert_eq!(&r[6..8], &[0, 1]);
        assert_eq!(&r[r.len() - 4..], &[192, 168, 1, 5]);
    }
}
#[test]
fn no_external_dns() {
    for n in ["example.org", "evilctest.cdn.nintendo.net"] {
        let r = dns::respond(&packet(n, 1), Ipv4Addr::LOCALHOST).unwrap();
        assert_eq!(r[3] & 15, 5);
        assert_eq!(&r[6..8], &[0, 0]);
    }
}
#[test]
fn no_ipv6_record() {
    let r = dns::respond(&packet("ctest.cdn.nintendo.net", 28), Ipv4Addr::LOCALHOST).unwrap();
    assert_eq!(r[3] & 15, 0);
    assert_eq!(&r[6..8], &[0, 0]);
}
#[test]
fn malformed_dns() {
    for p in [
        vec![],
        vec![0; 3],
        vec![255; 100],
        vec![0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 192, 12, 0, 1, 0, 1],
    ] {
        assert!(dns::respond(&p, Ipv4Addr::LOCALHOST).is_err());
    }
}
#[test]
fn http_contract() {
    for host in [
        "ctest.p01.ctest.srv.nintendo.net",
        "ctest-ipv6.p01.ctest.srv.nintendo.net",
        "ctest.cdn.nintendo.net",
        "conntest.nintendowifi.net",
        "CTEST.CDN.NINTENDO.NET.:80",
    ] {
        let r = String::from_utf8(http::respond("GET", host, "/")).unwrap();
        assert!(r.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(r.contains("X-Organization: Nintendo\r\n"));
        assert!(r.ends_with("\r\n\r\nok"));
        let head = String::from_utf8(http::respond("HEAD", host, "/")).unwrap();
        assert!(head.contains("Content-Length: 2\r\n"));
        assert!(head.ends_with("\r\n\r\n"));
    }
    for (method, host, path, code) in [
        ("GET", "example.org", "/", 404),
        ("GET", "ctest.cdn.nintendo.net", "/bad", 404),
        ("POST", "ctest.cdn.nintendo.net", "/", 405),
    ] {
        assert!(
            String::from_utf8(http::respond(method, host, path))
                .unwrap()
                .starts_with(&format!("HTTP/1.1 {code}"))
        );
    }
}
