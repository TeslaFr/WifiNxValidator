use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wifinx_validator::config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Fr,
}
impl Language {
    pub fn from_locale(locale: &str) -> Self {
        match locale.trim().split(['-', '_', '.']).next() {
            Some(code) if code.eq_ignore_ascii_case("fr") => Self::Fr,
            _ => Self::En,
        }
    }
    #[cfg(any(test, target_os = "linux", target_os = "macos"))]
    pub fn from_sources(all: Option<&str>, messages: Option<&str>, lang: Option<&str>) -> Self {
        [all, messages, lang]
            .into_iter()
            .flatten()
            .find(|s| !s.trim().is_empty())
            .map(Self::from_locale)
            .unwrap_or(Self::En)
    }
    pub fn text<'a>(self, french: &'a str, english: &'a str) -> &'a str {
        if self == Self::Fr { french } else { english }
    }
    pub fn other(self) -> Self {
        if self == Self::Fr { Self::En } else { Self::Fr }
    }
    pub fn code(self) -> &'static str {
        self.text("FR", "EN")
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(flatten)]
    pub network: Config,
    #[serde(
        default,
        deserialize_with = "read_language",
        skip_serializing_if = "Option::is_none"
    )]
    pub language: Option<Language>,
}
pub fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "WifiNx", "WifiNxValidator")
        .map(|d| d.config_dir().join("settings.json"))
}

fn read_language<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Language>, D::Error> {
    Ok(
        Option::<String>::deserialize(deserializer)?.and_then(|value| match value.as_str() {
            "en" => Some(Language::En),
            "fr" => Some(Language::Fr),
            _ => None,
        }),
    )
}
pub fn system_language() -> Language {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetUserDefaultUILanguage() -> u16;
        }
        // SAFETY: this Windows query takes no pointers and returns a LANGID.
        let id = unsafe { GetUserDefaultUILanguage() };
        Language::from_locale(if id & 0x03ff == 0x000c { "fr" } else { "en" })
    }
    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "macos")]
        if let Some(language) = macos_language() {
            return language;
        }
        let all = std::env::var("LC_ALL").ok();
        let messages = std::env::var("LC_MESSAGES").ok();
        let lang = std::env::var("LANG").ok();
        Language::from_sources(all.as_deref(), messages.as_deref(), lang.as_deref())
    }
}
#[cfg(target_os = "macos")]
fn macos_language() -> Option<Language> {
    use std::ffi::{CStr, c_char, c_void};
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFLocaleCopyPreferredLanguages() -> *const c_void;
        fn CFArrayGetCount(array: *const c_void) -> isize;
        fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
        fn CFStringGetCString(
            string: *const c_void,
            buffer: *mut c_char,
            size: isize,
            encoding: u32,
        ) -> u8;
        fn CFRelease(object: *const c_void);
    }
    // SAFETY: the Copy function returns an owned CFArray of CFStrings. We only
    // borrow its first string, check conversion success and release the array
    // exactly once before returning. UTF-8 encoding is kCFStringEncodingUTF8.
    unsafe {
        let languages = CFLocaleCopyPreferredLanguages();
        if languages.is_null() {
            return None;
        }
        let result = if CFArrayGetCount(languages) > 0 {
            let first = CFArrayGetValueAtIndex(languages, 0);
            let mut buffer = [0u8; 256];
            if !first.is_null()
                && CFStringGetCString(
                    first,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as isize,
                    0x08000100,
                ) != 0
            {
                CStr::from_ptr(buffer.as_ptr().cast())
                    .to_str()
                    .ok()
                    .map(Language::from_locale)
            } else {
                None
            }
        } else {
            None
        };
        CFRelease(languages);
        result
    }
}
pub fn load_settings() -> Result<Settings, String> {
    let Some(path) = config_path() else {
        return Ok(Settings::default());
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
        Err(e) => return Err(format!("Lecture des paramètres : {e}")),
    };
    serde_json::from_str(&text).map_err(|e| format!("Paramètres illisibles : {e}"))
}
pub fn initial_language() -> Language {
    load_settings()
        .ok()
        .and_then(|s| s.language)
        .unwrap_or_else(system_language)
}
impl Language {
    pub fn history_target(self, target: &str) -> &str {
        if target == "Paquet invalide" {
            self.text("Paquet invalide", "Invalid packet")
        } else {
            target
        }
    }
    // Keep internal diagnostic strings stable; translate them when displayed so
    // switching language also updates an error or history entry already shown.
    pub fn message(self, source: &str) -> String {
        if self == Self::Fr {
            return source.to_owned();
        }
        let mut result = source.to_owned();
        for (french, english) in ERROR_TEXT {
            if result.contains(french) {
                result = result.replace(french, english);
            }
        }
        match result.as_str() {
            "serveur" => "server".into(),
            "ERREUR" => "ERROR".into(),
            "REFUS" => "REFUSED".into(),
            _ => result,
        }
    }
}
const ERROR_TEXT: &[(&str, &str)] = &[
    (
        "La zone de notification est indisponible. La fenêtre reste accessible ; utilisez le bouton de réduction du système ou Quitter.",
        "The system tray is unavailable. The window remains accessible; use the taskbar or Quit.",
    ),
    (
        "Zone de notification indisponible : activer AppIndicator/StatusNotifier sur le bureau et installer gdbus.",
        "System tray unavailable: enable AppIndicator/StatusNotifier on your desktop and install gdbus.",
    ),
    (
        "Les services se sont arrêtés. Consulter l'historique avant de redémarrer.",
        "The services have stopped. Check the history before restarting.",
    ),
    (
        "L'IP annoncée doit être une adresse IPv4 unicast du PC accessible depuis la console.",
        "The advertised IP must be a unicast IPv4 address of this PC reachable from the console.",
    ),
    (
        "L'autorisation a été accordée mais les droits sur les ports n'ont pas été appliqués. Copier le binaire sur un système de fichiers autorisant les capacités Linux, puis relancer.",
        "Authorization succeeded but the port capability was not applied. Copy the executable to a file system supporting Linux capabilities, then restart.",
    ),
    (
        "Demande des droits administrateur annulée. Relancer l'application pour réessayer.",
        "Administrator authorization was cancelled. Restart the application to try again.",
    ),
    (
        "Autorisation refusée ou impossible. Vérifier Polkit, son agent graphique et la prise en charge des capacités Linux sur le binaire.",
        "Authorization was denied or failed. Check Polkit, its desktop authentication agent and file capability support.",
    ),
    (
        "setcap introuvable. Installer libcap (Arch/Garuda) ou libcap2-bin (Debian/Ubuntu).",
        "setcap was not found. Install libcap (Arch/Garuda) or libcap2-bin (Debian/Ubuntu).",
    ),
    (
        "pkexec introuvable. Installer Polkit et son agent d'authentification graphique.",
        "pkexec was not found. Install Polkit and its desktop authentication agent.",
    ),
    (
        "Répertoire de configuration utilisateur introuvable",
        "User configuration directory not found",
    ),
    ("L'autorisation", "Authorization"),
    (
        "Adresse d'écoute IPv4 invalide",
        "Invalid IPv4 listen address",
    ),
    ("Adresse d'écoute invalide.", "Invalid listen address."),
    (
        "IP annoncée IPv4 invalide",
        "Invalid advertised IPv4 address",
    ),
    ("Paramètres illisibles", "Could not parse settings"),
    ("Lecture des paramètres", "Could not read settings"),
    ("Enregistrement des paramètres", "Could not save settings"),
    ("Répertoire invalide", "Invalid directory"),
    ("Démarrage impossible", "Could not start services"),
    (
        "Vérifier les ports, l'adresse et les permissions.",
        "Check the ports, listen address and permissions.",
    ),
    (
        "Relancement après autorisation impossible",
        "Could not restart after authorization",
    ),
    (
        "Lecture des droits du processus",
        "Could not read process capabilities",
    ),
    (
        "Les capacités du processus sont introuvables.",
        "Process capabilities could not be found.",
    ),
    (
        "Les capacités du processus sont illisibles.",
        "Process capabilities could not be read.",
    ),
    (
        "Demande des droits administrateur",
        "Administrator authorization",
    ),
    (
        "Autorisation nécessaire pour WifiNx Validator",
        "WifiNx Validator needs authorization",
    ),
    (
        "WifiNx Validator n'a pas pu démarrer.",
        "WifiNx Validator could not start.",
    ),
    (
        "Journal indisponible : aucun fichier accessible.",
        "Log unavailable: no writable file.",
    ),
    ("Écriture du journal impossible", "Could not write the log"),
    ("Erreur interne", "Internal error"),
    ("Journal :", "Log:"),
    (
        "Journal de démarrage indisponible",
        "Startup log unavailable",
    ),
    (
        "Aucun adaptateur Direct3D compatible, matériel ou WARP, n'est disponible.",
        "No compatible Direct3D hardware or WARP adapter is available.",
    ),
    ("En-tête DNS invalide", "Invalid DNS header"),
    ("Nom DNS tronqué", "Truncated DNS name"),
    ("Pointeur tronqué", "Truncated pointer"),
    ("Label DNS invalide", "Invalid DNS label"),
    ("Question tronquée", "Truncated question"),
    ("Nom DNS trop long", "DNS name too long"),
    ("Label tronqué", "Truncated label"),
    ("Label non pris en charge", "Unsupported label"),
    ("Boucle de compression DNS", "DNS compression loop"),
    ("Paquet invalide", "Invalid packet"),
    (
        "Requête annulée ou délai dépassé",
        "Request cancelled or timed out",
    ),
    ("En-têtes HTTP trop longs", "HTTP headers too large"),
    ("Un en-tête Host est requis", "A Host header is required"),
    ("Limite de connexions", "Connection limit"),
    ("Notification :", "System tray:"),
];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_locales_select_french_or_english() {
        for locale in ["fr", "fr_FR.UTF-8", "fr-CA", "FR_be", " fr_CH "] {
            assert_eq!(Language::from_locale(locale), Language::Fr);
        }
        for locale in ["en-US", "en_GB", "de_DE", "C", "C.UTF-8", ""] {
            assert_eq!(Language::from_locale(locale), Language::En);
        }
    }
    #[test]
    fn locale_environment_priority_matches_the_desktop() {
        assert_eq!(
            Language::from_sources(Some("fr_FR"), Some("en_US"), Some("en_US")),
            Language::Fr
        );
        assert_eq!(
            Language::from_sources(Some(""), Some("fr_CA"), Some("en_US")),
            Language::Fr
        );
        assert_eq!(
            Language::from_sources(None, None, Some("fr_FR.UTF-8")),
            Language::Fr
        );
        assert_eq!(
            Language::from_sources(Some("C"), Some("fr_FR"), Some("fr_FR")),
            Language::En
        );
    }
    #[test]
    fn old_settings_keep_network_and_default_to_system_language() {
        let settings: Settings = serde_json::from_str(
            r#"{"listen":"192.168.1.27","answer":"192.168.1.27","dns_port":5353,"http_port":8080}"#,
        )
        .unwrap();
        assert_eq!(settings.language, None);
        assert_eq!(settings.network.listen.to_string(), "192.168.1.27");
        assert_eq!(
            (settings.network.dns_port, settings.network.http_port),
            (53, 80)
        );
    }
    #[test]
    fn explicit_language_is_saved_without_configurable_ports() {
        let settings = Settings {
            language: Some(Language::En),
            ..Settings::default()
        };
        let value = serde_json::to_value(&settings).unwrap();
        assert_eq!(value["language"], "en");
        assert!(value.get("dns_port").is_none());
        let restored: Settings = serde_json::from_value(value).unwrap();
        assert_eq!(restored.language, Some(Language::En));
        assert_eq!(restored.network.answer, settings.network.answer);
    }
    #[test]
    fn unsupported_saved_language_preserves_the_network_settings() {
        let settings: Settings = serde_json::from_str(
            r#"{"language":"de","listen":"192.168.1.27","answer":"192.168.1.27"}"#,
        )
        .unwrap();
        assert_eq!(settings.language, None);
        assert_eq!(settings.network.answer.to_string(), "192.168.1.27");
    }
    #[test]
    fn existing_errors_can_change_language_without_losing_details() {
        let original = "Démarrage impossible : DNS UDP : Address already in use (os error 98). Vérifier les ports, l'adresse et les permissions.";
        let english = Language::En.message(original);
        assert!(english.contains("Could not start services"));
        assert!(english.contains("DNS UDP : Address already in use (os error 98)"));
        assert!(!english.contains("Démarrage"));
        assert_eq!(Language::Fr.message(original), original);
        assert_eq!(
            Language::En.message("ctest.cdn.nintendo.net"),
            "ctest.cdn.nintendo.net"
        );
    }
}
