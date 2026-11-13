use std::net::Ipv4Addr;

pub const HOSTS: [&str; 4] = [
    "ctest.p01.ctest.srv.nintendo.net",
    "ctest-ipv6.p01.ctest.srv.nintendo.net",
    "ctest.cdn.nintendo.net",
    "conntest.nintendowifi.net",
];

pub fn is_local(name: &str) -> bool {
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    HOSTS
        .iter()
        .any(|host| name == *host || name.ends_with(&format!(".{host}")))
}

/// Decode a single question. Compression is bounded to prevent pointer cycles.
pub fn question(packet: &[u8]) -> Result<(String, u16, u16), String> {
    if packet.len() < 12 || packet.len() > 4096 || packet[2] & 0xf8 != 0 || packet[4..6] != [0, 1] {
        return Err("En-tête DNS invalide".into());
    }
    let mut pos = 12;
    let mut end = None;
    let mut labels = Vec::new();
    let mut length = 1;
    for _ in 0..128 {
        let n = *packet.get(pos).ok_or("Nom DNS tronqué")?;
        if n & 0xc0 == 0xc0 {
            let next = *packet.get(pos + 1).ok_or("Pointeur tronqué")?;
            end.get_or_insert(pos + 2);
            pos = ((n as usize & 63) << 8) | next as usize;
            continue;
        }
        if n & 0xc0 != 0 {
            return Err("Label DNS invalide".into());
        }
        pos += 1;
        if n == 0 {
            let e = end.unwrap_or(pos);
            let tail = packet.get(e..e + 4).ok_or("Question tronquée")?;
            return Ok((
                labels.join("."),
                u16::from_be_bytes([tail[0], tail[1]]),
                u16::from_be_bytes([tail[2], tail[3]]),
            ));
        }
        length += n as usize + 1;
        if length > 255 {
            return Err("Nom DNS trop long".into());
        }
        let label = packet.get(pos..pos + n as usize).ok_or("Label tronqué")?;
        // Arbitrary binary labels are legal DNS, but not local hostnames.
        if !label
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
        {
            return Err("Label non pris en charge".into());
        }
        labels.push(String::from_utf8(label.to_vec()).map_err(|e| e.to_string())?);
        pos += n as usize;
    }
    Err("Boucle de compression DNS".into())
}

pub fn respond(packet: &[u8], address: Ipv4Addr) -> Result<Vec<u8>, String> {
    let (name, kind, class) = question(packet)?;
    let local = is_local(&name) && class == 1;
    let answer = local && kind == 1;
    let mut out = vec![
        packet[0],
        packet[1],
        0x80 | (packet[2] & 1) | if local { 4 } else { 0 },
        if local { 0 } else { 5 },
        0,
        1,
        0,
        u8::from(answer),
        0,
        0,
        0,
        0,
    ];
    if !name.is_empty() {
        for label in name.split('.') {
            out.push(label.len() as u8);
            out.extend(label.as_bytes());
        }
    }
    out.push(0);
    out.extend(kind.to_be_bytes());
    out.extend(class.to_be_bytes());
    if answer {
        out.extend([0xc0, 12, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4]);
        out.extend(address.octets());
    }
    Ok(out)
}
