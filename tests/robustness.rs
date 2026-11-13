use std::net::Ipv4Addr;
use wifinx_validator::{dns, http};
#[test]
fn compressed_questions_are_reencoded_without_dangling_pointers() {
    let mut p = vec![0, 7, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 192, 18, 0, 1, 0, 1];
    for label in "ctest.cdn.nintendo.net".split('.') {
        p.push(label.len() as u8);
        p.extend(label.as_bytes());
    }
    p.push(0);
    let r = dns::respond(&p, Ipv4Addr::LOCALHOST).unwrap();
    assert_eq!(
        dns::question(&{
            let mut q = r.clone();
            q[2] = 1;
            q
        })
        .unwrap()
        .0,
        "ctest.cdn.nintendo.net"
    );
    assert_eq!(&r[r.len() - 4..], &[127, 0, 0, 1]);
}
#[test]
fn arbitrary_packets_never_panic() {
    let mut seed = 42u64;
    for len in 0..600 {
        for _ in 0..8 {
            let mut p = Vec::new();
            for _ in 0..len {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                p.push((seed >> 32) as u8);
            }
            let _ = dns::respond(&p, Ipv4Addr::LOCALHOST);
        }
    }
}
#[test]
fn invalid_compression_and_response_packets_are_rejected() {
    let p = vec![0, 7, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 192, 255, 0, 1, 0, 1];
    assert!(dns::respond(&p, Ipv4Addr::LOCALHOST).is_err());
    let mut p = vec![0, 7, 129, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1];
    assert!(dns::respond(&p, Ipv4Addr::LOCALHOST).is_err());
    p[2] = 1;
    p[5] = 2;
    assert!(dns::respond(&p, Ipv4Addr::LOCALHOST).is_err());
}
#[test]
fn http_exact_hosts_and_head_error_lengths() {
    assert!(http::respond("GET", "sub.ctest.cdn.nintendo.net", "/").starts_with(b"HTTP/1.1 404"));
    assert!(http::respond("GET", "ctest.cdn.nintendo.net", "/?a=1").starts_with(b"HTTP/1.1 404"));
    let r = String::from_utf8(http::respond("HEAD", "invalid", "/")).unwrap();
    assert!(r.contains("Content-Length: 9\r\n"));
    assert!(r.ends_with("\r\n\r\n"));
}

#[test]
fn invalid_http_authorities_do_not_match_local_hosts() {
    for authority in [
        "ctest.cdn.nintendo.net:garbage",
        "ctest.cdn.nintendo.net:80:evil",
        "ctest.cdn.nintendo.net:",
        "ctest.cdn.nintendo.net:0",
        "ctest.cdn.nintendo.net:65536",
        "ctest.cdn.nintendo.net:+80",
    ] {
        assert!(
            http::respond("GET", authority, "/").starts_with(b"HTTP/1.1 404"),
            "{authority}"
        );
    }
}

#[test]
fn notification_requires_registered_host_not_just_a_watcher() {
    use wifinx_validator::notification::host_registered;
    assert!(host_registered("(<true>,)\n"));
    assert!(host_registered("( <true>, )"));
    for reply in [
        "(true,)",
        "(<false>,)",
        "",
        "error: true is not a valid reply",
    ] {
        assert!(!host_registered(reply), "{reply}");
    }
}
