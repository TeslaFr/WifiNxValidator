use crate::i18n::Language;
use eframe::egui;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

#[derive(Clone, Copy)]
pub enum Action {
    Show,
    Toggle,
    Quit,
}
pub struct Tray {
    pub available: Arc<AtomicBool>,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    _icon: TrayIcon,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    items: [MenuItem; 3],
    #[cfg(target_os = "linux")]
    stop: Arc<AtomicBool>,
    #[cfg(target_os = "linux")]
    languages: Sender<Language>,
    #[cfg(target_os = "linux")]
    task: Option<std::thread::JoinHandle<()>>,
}
fn create(
    ctx: egui::Context,
    tx: Sender<Action>,
    language: Language,
) -> Result<(TrayIcon, [MenuItem; 3]), String> {
    let menu = Menu::new();
    let show = MenuItem::new(
        language.text("Ouvrir WifiNx Validator", "Open WifiNx Validator"),
        true,
        None,
    );
    let toggle = MenuItem::new(
        language.text("Activer / arrêter DNS + HTTP", "Start / stop DNS + HTTP"),
        true,
        None,
    );
    let quit = MenuItem::new(
        language.text("Quitter complètement", "Quit completely"),
        true,
        None,
    );
    menu.append_items(&[&show, &toggle, &quit])
        .map_err(|e| e.to_string())?;
    let show_id = show.id().clone();
    let toggle_id = toggle.id().clone();
    let quit_id = quit.id().clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let action = if event.id == show_id {
            Action::Show
        } else if event.id == toggle_id {
            Action::Toggle
        } else if event.id == quit_id {
            Action::Quit
        } else {
            return;
        };
        let _ = tx.send(action);
        // Wake the native loop, including while the main window is hidden.
        ctx.request_repaint();
    }));
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let i = (y * 32 + x) * 4;
            let bright = (7..25).contains(&x) && (7..25).contains(&y);
            pixels[i..i + 4].copy_from_slice(if bright {
                &[72, 211, 168, 255]
            } else {
                &[24, 34, 51, 255]
            });
        }
    }
    let icon = Icon::from_rgba(pixels, 32, 32).map_err(|e| e.to_string())?;
    let icon = TrayIconBuilder::new()
        .with_tooltip("WifiNx Validator — DNS + HTTP")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .map_err(|e| e.to_string())?;
    Ok((icon, [show, toggle, quit]))
}
fn update_items(items: &[MenuItem; 3], language: Language) {
    items[0].set_text(language.text("Ouvrir WifiNx Validator", "Open WifiNx Validator"));
    items[1].set_text(language.text("Activer / arrêter DNS + HTTP", "Start / stop DNS + HTTP"));
    items[2].set_text(language.text("Quitter complètement", "Quit completely"));
}
impl Tray {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn start(
        ctx: egui::Context,
        tx: Sender<Action>,
        language: Language,
    ) -> Result<Self, String> {
        let (icon, items) = create(ctx, tx, language)?;
        Ok(Self {
            _icon: icon,
            items,
            available: Arc::new(AtomicBool::new(true)),
        })
    }
    #[cfg(target_os = "linux")]
    pub fn start(
        ctx: egui::Context,
        tx: Sender<Action>,
        language: Language,
    ) -> Result<Self, String> {
        use std::{sync::mpsc, thread, time::Duration};
        let available = Arc::new(AtomicBool::new(false));
        let ready = available.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let (status_tx, status_rx) = mpsc::sync_channel(1);
        let (languages, language_updates) = mpsc::channel();
        let task=thread::Builder::new().name("wifinx-tray".into()).spawn(move || {
            if let Err(e)=gtk::init(){let _=status_tx.send(Err(e.to_string()));return;}
            if !watcher_available(){let _=status_tx.send(Err("Zone de notification indisponible : activer AppIndicator/StatusNotifier sur le bureau et installer gdbus.".into()));return;}
            let (icon, items)=match create(ctx.clone(),tx,language){Ok(i)=>i,Err(e)=>{let _=status_tx.send(Err(e));return;}};
            ready.store(true,Ordering::Relaxed);
            if status_tx.send(Ok(())).is_err(){return;}
            let mut ticks=0;
            gtk::glib::timeout_add_local(Duration::from_millis(200),move || {
                if stopped.load(Ordering::Relaxed){gtk::main_quit();return gtk::glib::ControlFlow::Break;}
                if let Some(language) = language_updates.try_iter().last() { update_items(&items, language); }
                ticks+=1;
                if ticks%10==0 {
                    let supported=watcher_available();
                    if ready.swap(supported,Ordering::Relaxed)&&!supported {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));ctx.request_repaint();
                    }
                }
                gtk::glib::ControlFlow::Continue
            });
            gtk::main();drop(icon);
        }).map_err(|e|e.to_string())?;
        match status_rx.recv_timeout(Duration::from_secs(4)) {
            Ok(Ok(())) => Ok(Self {
                available,
                stop,
                languages,
                task: Some(task),
            }),
            Ok(Err(e)) => {
                stop.store(true, Ordering::Relaxed);
                let _ = task.join();
                Err(e)
            }
            Err(e) => {
                stop.store(true, Ordering::Relaxed);
                Err(format!("Notification : {e}"))
            }
        }
    }
    pub fn set_language(&self, language: Language) {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        update_items(&self.items, language);
        #[cfg(target_os = "linux")]
        if self.languages.send(language).is_err() {
            self.available.store(false, Ordering::Relaxed);
        }
    }
    pub fn can_hide(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }
}
#[cfg(target_os = "linux")]
impl Drop for Tray {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            let _ = task.join();
        }
    }
}

#[cfg(target_os = "linux")]
fn watcher_available() -> bool {
    use std::{
        io::Read,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let mut child = match Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.kde.StatusNotifierWatcher",
            "--object-path",
            "/StatusNotifierWatcher",
            "--method",
            "org.freedesktop.DBus.Properties.Get",
            "org.kde.StatusNotifierWatcher",
            "IsStatusNotifierHostRegistered",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return false;
                }
                let mut text = String::new();
                if let Some(out) = child.stdout.take() {
                    let _ = out.take(128).read_to_string(&mut text);
                }
                return wifinx_validator::notification::host_registered(&text);
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}
