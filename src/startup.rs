use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct StartupLog {
    path: PathBuf,
    file: Option<Arc<Mutex<File>>>,
}
impl StartupLog {
    pub fn open(path: PathBuf) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            path,
            file: Some(Arc::new(Mutex::new(file))),
        })
    }
    fn open_with_fallback(primary: PathBuf, fallback: PathBuf) -> io::Result<Self> {
        Self::open(primary).or_else(|_| Self::open(fallback))
    }
    pub fn unavailable() -> Self {
        Self {
            path: PathBuf::new(),
            file: None,
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn record(&self, stage: &str, message: &str) -> io::Result<()> {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let file = self
            .file
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Journal indisponible"))?;
        let mut file = file.lock().unwrap_or_else(|p| p.into_inner());
        writeln!(file, "[{seconds}] {stage}: {message}")
    }
    pub fn failure(&self, error: &str) -> String {
        if self.file.is_none() {
            return format!(
                "WifiNx Validator n'a pas pu démarrer.\n\n{error}\n\nJournal indisponible : aucun fichier accessible."
            );
        }
        let saved = self.record("ERROR", error);
        let mut message = format!(
            "WifiNx Validator n'a pas pu démarrer.\n\n{error}\n\nJournal : {}",
            self.path().display()
        );
        if let Err(e) = saved {
            message.push_str(&format!("\nÉcriture du journal impossible : {e}"));
        }
        message
    }
    pub fn open_default() -> io::Result<Self> {
        #[cfg(target_os = "windows")]
        let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        #[cfg(not(target_os = "windows"))]
        let base = directories::ProjectDirs::from("org", "WifiNx", "WifiNxValidator")
            .map(|p| p.data_local_dir().to_path_buf());
        let base = base.unwrap_or_else(std::env::temp_dir);
        Self::open_with_fallback(
            base.join("WifiNxValidator").join("startup.log"),
            std::env::temp_dir().join(format!("wifinx-startup-{}.log", std::process::id())),
        )
    }
}
/// Show Windows startup failures without requiring a console or graphics renderer.
#[cfg(target_os = "windows")]
pub fn show_error(message: &str) {
    let language = crate::i18n::initial_language();
    let message = language.message(message);
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            window: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }
    let text: Vec<u16> = message
        .replace('\0', " ")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let title: Vec<u16> = language
        .text("WifiNx Validator — erreur", "WifiNx Validator — error")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: both buffers are NUL-terminated and live for the synchronous call.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x10);
    }
}
#[cfg(target_os = "macos")]
pub fn show_error(message: &str) {
    let language = crate::i18n::initial_language();
    let title = language.text("WifiNx Validator — erreur", "WifiNx Validator — error");
    let message = language.message(message);
    // Arguments are passed separately; the error text is never AppleScript code.
    let script = "on run argv\n display alert (item 1 of argv) message (item 2 of argv) as critical buttons {\"OK\"} default button \"OK\"\nend run";
    match std::process::Command::new("/usr/bin/osascript")
        .args(["-e", script, "--", title, &message])
        .status()
    {
        Ok(status) if status.success() => {}
        _ => eprintln!("{title}: {message}"),
    }
}
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn show_error(message: &str) {
    eprintln!("{}", crate::i18n::initial_language().message(message));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "wifinx-startup-{}-{}.log",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn startup_stages_survive_a_restart() {
        let path = path();
        let log = StartupLog::open(path.clone()).unwrap();
        log.record("START", "version de test").unwrap();
        drop(log);
        let log = StartupLog::open(path.clone()).unwrap();
        log.record("WINDOW", "création de la fenêtre").unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("START") && text.contains("version de test"));
        assert!(text.contains("WINDOW") && text.contains("création de la fenêtre"));
        drop(log);
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn startup_error_is_logged_and_ready_for_a_visible_dialog() {
        let path = path();
        let log = StartupLog::open(path.clone()).unwrap();
        let error = "OpenGL(PainterError: egui_glow requires opengl 2.0+)";
        let report = log.failure(error);
        assert!(report.contains(error));
        assert!(report.contains(&path.display().to_string()));
        assert!(fs::read_to_string(&path).unwrap().contains(error));
        drop(log);
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn blocked_log_directory_uses_a_temporary_log() {
        let blocked = path();
        fs::write(&blocked, "An existing file blocks this directory").unwrap();
        let fallback = path();
        let log =
            StartupLog::open_with_fallback(blocked.join("startup.log"), fallback.clone()).unwrap();
        assert_eq!(log.path(), fallback);
        log.record("START", "fallback").unwrap();
        assert!(fs::read_to_string(&fallback).unwrap().contains("fallback"));
        drop(log);
        fs::remove_file(blocked).unwrap();
        fs::remove_file(fallback).unwrap();
    }
}
