//! Linux authorization for binding the standard DNS and HTTP ports.
//!
//! The GUI stays in the desktop user's session; only setcap runs through Polkit.
pub fn prepare() -> Result<(), String> {
    use std::{fs, os::unix::process::CommandExt, path::Path, process::Command};
    const ATTEMPTED: &str = "WIFINX_BIND_AUTHORIZED";
    let status = fs::read_to_string("/proc/self/status")
        .map_err(|e| format!("Lecture des droits du processus : {e}"))?;
    if has_bind_capability(&status)? {
        return Ok(());
    }
    if std::env::var_os(ATTEMPTED).is_some() {
        return Err("L'autorisation a été accordée mais les droits sur les ports n'ont pas été appliqués. Copier le binaire sur un système de fichiers autorisant les capacités Linux, puis relancer.".into());
    }
    let setcap = ["/usr/sbin/setcap", "/usr/bin/setcap", "/sbin/setcap"]
        .into_iter()
        .find(|p| Path::new(p).is_file())
        .ok_or(
            "setcap introuvable. Installer libcap (Arch/Garuda) ou libcap2-bin (Debian/Ubuntu).",
        )?;
    let pkexec = ["/usr/bin/pkexec", "/bin/pkexec"]
        .into_iter()
        .find(|p| Path::new(p).is_file())
        .ok_or("pkexec introuvable. Installer Polkit et son agent d'authentification graphique.")?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let result = Command::new(pkexec)
        .args([
            "--disable-internal-agent",
            setcap,
            "cap_net_bind_service=+ep",
        ])
        .arg(&executable)
        .status()
        .map_err(|e| format!("Demande des droits administrateur : {e}"))?;
    authorization_result(result.code())?;
    // A fresh exec is required for the capability to become effective. The
    // desktop UID, session bus and configuration directory remain unchanged.
    let error = Command::new(executable)
        .args(std::env::args_os().skip(1))
        .env(ATTEMPTED, "1")
        .exec();
    Err(format!(
        "Relancement après autorisation impossible : {error}"
    ))
}
fn has_bind_capability(status: &str) -> Result<bool, String> {
    let value = status
        .lines()
        .find_map(|line| line.strip_prefix("CapEff:"))
        .ok_or("Les capacités du processus sont introuvables.")?;
    let bits = u64::from_str_radix(value.trim(), 16)
        .map_err(|_| "Les capacités du processus sont illisibles.")?;
    Ok(bits & (1 << 10) != 0) // CAP_NET_BIND_SERVICE
}
fn authorization_result(code: Option<i32>) -> Result<(), String> {
    match code {
        Some(0) => Ok(()),
        Some(126) => Err("Demande des droits administrateur annulée. Relancer l'application pour réessayer.".into()),
        _ => Err("Autorisation refusée ou impossible. Vérifier Polkit, son agent graphique et la prise en charge des capacités Linux sur le binaire.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_bind_capability_does_not_require_authorization() {
        assert!(has_bind_capability("Name:\twifinx\nCapEff:\t0000000000000400\n").unwrap());
        assert!(has_bind_capability("CapEff:\t000001ffffffffff\n").unwrap());
        assert!(!has_bind_capability("CapEff:\t0000000000000000\n").unwrap());
        assert!(!has_bind_capability("CapEff:\t0000000000000200\n").unwrap());
    }
    #[test]
    fn unreadable_capabilities_are_not_treated_as_authorized() {
        assert!(has_bind_capability("Name:\twifinx\n").is_err());
        assert!(has_bind_capability("CapEff:\tinvalid\n").is_err());
    }
    #[test]
    fn cancelled_or_failed_authentication_is_an_error() {
        assert!(authorization_result(Some(0)).is_ok());
        for code in [Some(126), Some(127), Some(1), None] {
            assert!(authorization_result(code).is_err());
        }
    }
}
