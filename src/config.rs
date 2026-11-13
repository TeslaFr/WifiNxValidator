use std::net::Ipv4Addr;
pub const DNS_PORT: u16 = 53;
pub const HTTP_PORT: u16 = 80;
#[cfg(feature = "desktop")]
fn default_dns_port() -> u16 {
    DNS_PORT
}
#[cfg(feature = "desktop")]
fn default_http_port() -> u16 {
    HTTP_PORT
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "desktop", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "desktop", serde(default))]
pub struct Config {
    pub listen: Ipv4Addr,
    pub answer: Ipv4Addr,
    #[cfg_attr(feature = "desktop", serde(skip, default = "default_dns_port"))]
    pub dns_port: u16,
    #[cfg_attr(feature = "desktop", serde(skip, default = "default_http_port"))]
    pub http_port: u16,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            listen: Ipv4Addr::UNSPECIFIED,
            answer: Ipv4Addr::new(192, 168, 50, 2),
            dns_port: DNS_PORT,
            http_port: HTTP_PORT,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.answer.is_unspecified() || self.answer.is_multicast() || self.answer.is_broadcast()
        {
            return Err("L'IP annoncée doit être une adresse IPv4 unicast du PC accessible depuis la console.".into());
        }
        if self.listen.is_multicast() || self.listen.is_broadcast() {
            return Err("Adresse d'écoute invalide.".into());
        }
        Ok(())
    }
}
