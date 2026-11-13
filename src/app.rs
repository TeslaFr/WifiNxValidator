use crate::{
    i18n::{self, Language, Settings},
    tray::{Action, Tray},
};
use eframe::egui::{self, Color32, RichText};
use std::{
    net::Ipv4Addr,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, UNIX_EPOCH},
};
use wifinx_validator::{
    config::{Config, DNS_PORT, HTTP_PORT},
    history::History,
    server::Server,
};

pub struct App {
    config: Config,
    language: Language,
    language_override: Option<Language>,
    listen: String,
    answer: String,
    history: History,
    server: Option<Server>,
    tray: Option<Tray>,
    actions: Receiver<Action>,
    error: Option<String>,
    filter: String,
    quitting: bool,
    path: Option<PathBuf>,
}
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let path = i18n::config_path();
        let (settings, mut error) = match i18n::load_settings() {
            Ok(settings) => (settings, None),
            Err(error) => (Settings::default(), Some(error)),
        };
        let language = settings.language.unwrap_or_else(i18n::system_language);
        let config = settings.network;
        let (tx, actions) = mpsc::channel();
        let tray = match Tray::start(cc.egui_ctx.clone(), tx, language) {
            Ok(t) => Some(t),
            Err(e) => {
                error = Some(e);
                None
            }
        };
        Self {
            listen: config.listen.to_string(),
            answer: config.answer.to_string(),
            config,
            language,
            language_override: settings.language,
            history: History::default(),
            server: None,
            tray,
            actions,
            error,
            filter: String::new(),
            quitting: false,
            path,
        }
    }
    fn change_language(&mut self, language: Language) {
        self.language = language;
        self.language_override = Some(language);
        if let Some(tray) = &self.tray {
            tray.set_language(language);
        }
        if let Err(error) = self.persist() {
            self.error = Some(error);
        }
    }
    fn active(&self) -> bool {
        self.server.as_ref().is_some_and(Server::is_running)
    }
    fn parse_config(&self) -> Result<Config, String> {
        let listen = self
            .listen
            .trim()
            .parse::<Ipv4Addr>()
            .map_err(|_| "Adresse d'écoute IPv4 invalide")?;
        let answer = self
            .answer
            .trim()
            .parse::<Ipv4Addr>()
            .map_err(|_| "IP annoncée IPv4 invalide")?;
        let config = Config {
            listen,
            answer,
            ..Config::default()
        };
        config.validate()?;
        Ok(config)
    }
    fn persist(&self) -> Result<(), String> {
        let path = self
            .path
            .as_ref()
            .ok_or("Répertoire de configuration utilisateur introuvable")?;
        std::fs::create_dir_all(path.parent().ok_or("Répertoire invalide")?)
            .map_err(|e| e.to_string())?;
        let data = serde_json::to_string_pretty(&Settings {
            network: self.config.clone(),
            language: self.language_override,
        })
        .map_err(|e| e.to_string())?;
        std::fs::write(path, data).map_err(|e| format!("Enregistrement des paramètres : {e}"))
    }
    fn toggle(&mut self) {
        if self.active() {
            self.server.take();
            return;
        }
        self.server.take();
        self.error = None;
        match self.parse_config().and_then(|cfg| {
            Server::start(cfg.clone(), self.history.clone()).map(|server| (cfg, server))
        }) {
            Ok((cfg, server)) => {
                self.config = cfg;
                self.server = Some(server);
                if let Err(e) = self.persist() {
                    self.error = Some(e);
                }
            }
            Err(e) => {
                self.error = Some(format!(
                    "Démarrage impossible : {e}. Vérifier les ports, l'adresse et les permissions."
                ))
            }
        }
    }
    fn quit(&mut self, ctx: &egui::Context) {
        self.server.take();
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    fn show(ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
    fn hide(&mut self, ctx: &egui::Context) {
        if self.tray.as_ref().is_some_and(Tray::can_hide) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else {
            self.error=Some("La zone de notification est indisponible. La fenêtre reste accessible ; utilisez le bouton de réduction du système ou Quitter.".into());
        }
    }
}
impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        while let Ok(action) = self.actions.try_recv() {
            match action {
                Action::Show => Self::show(ctx),
                Action::Toggle => {
                    self.toggle();
                    if self.error.is_some() {
                        Self::show(ctx);
                    }
                }
                Action::Quit => {
                    self.quit(ctx);
                    break;
                }
            }
        }
        if self.quitting {
            return;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide(ctx);
        }
        if self.server.as_ref().is_some_and(|s| !s.is_running()) {
            self.server.take();
            self.error = Some(
                "Les services se sont arrêtés. Consulter l'historique avant de redémarrer.".into(),
            );
            Self::show(ctx);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }
    fn ui(&mut self, root: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.quitting {
            return;
        }
        let ctx = root.ctx().clone();
        let language = self.language;
        let header = egui::Panel::top("header").show(root, |ui| {
            ui.add_space(12.0);
            let buttons = ui
                .horizontal(|ui| {
                    ui.heading("WifiNx Validator");
                    ui.label(language.text("Réseau local Nintendo", "Nintendo local network"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let quit = ui.button(language.text("Quitter", "Quit"));
                        if quit.clicked() {
                            self.quit(&ctx);
                        }
                        let hide = ui.button(language.text("Masquer", "Hide"));
                        if hide.clicked() {
                            self.hide(&ctx);
                        }
                        let choose_language = ui
                            .button(language.other().code())
                            .on_hover_text(language.text("Passer en anglais", "Switch to French"));
                        if choose_language.clicked() {
                            self.change_language(language.other());
                            ctx.request_repaint();
                        }
                        [quit.rect, hide.rect, choose_language.rect]
                    })
                    .inner
                })
                .inner;
            ui.add_space(10.0);
            buttons
        });
        // Cover the entire header, including its margins and empty space,
        // while leaving the header buttons outside the native drag hit areas.
        for (index, rect) in header_drag_regions(header.response.rect, header.inner)
            .into_iter()
            .enumerate()
        {
            let drag = root
                .interact(
                    rect,
                    root.scope_id().with(("window_drag", index)),
                    egui::Sense::drag(),
                )
                .on_hover_cursor(egui::CursorIcon::Grab)
                .on_hover_text(language.text(
                    "Faire glisser pour déplacer la fenêtre",
                    "Drag to move the window",
                ));
            if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        }
        egui::CentralPanel::default().show(root,|ui|{
            let active=self.active();
            ui.horizontal(|ui|{
                let color = if active { Color32::from_rgb(72,211,168) } else { Color32::GRAY };
                let (indicator, _) = ui.allocate_exact_size(egui::vec2(12.0, 18.0), egui::Sense::hover());
                ui.painter().circle_filled(indicator.center(), 4.0, color);
                ui.label(RichText::new(if active { language.text("DNS + HTTP actifs", "DNS + HTTP running") } else { language.text("Services arrêtés", "Services stopped") }).color(color).size(18.0));
                if ui.button(if active{language.text("Arrêter", "Stop")}else{language.text("Activer DNS + HTTP", "Start DNS + HTTP")}).clicked(){self.toggle();}
            });
            if let Some(s)=&self.server{ui.label(format!("DNS UDP/TCP: {}   •   HTTP: {}   •   {} {}",s.dns_addr(),s.http_addr(),s.active_connections(), language.text("connexion(s)", "connection(s)")));}
            ui.add_space(8.0);
            ui.add_enabled_ui(!active,|ui|{
                egui::Grid::new("settings").num_columns(4).spacing([12.0,8.0]).show(ui,|ui|{
                    ui.label(language.text("Adresse d'écoute", "Listen address"));ui.add(egui::TextEdit::singleline(&mut self.listen).desired_width(160.0).min_size(egui::vec2(160.0, 0.0)));ui.label(language.text("Port DNS", "DNS port"));ui.label(DNS_PORT.to_string());ui.end_row();
                    ui.label(language.text("IP du PC annoncée", "Advertised PC IP"));ui.add(egui::TextEdit::singleline(&mut self.answer).desired_width(160.0).min_size(egui::vec2(160.0, 0.0)));ui.label(language.text("Port HTTP", "HTTP port"));ui.label(HTTP_PORT.to_string());ui.end_row();
                });
                ui.horizontal(|ui|{
                    if ui.button(language.text("Enregistrer les paramètres", "Save settings")).clicked(){match self.parse_config(){Ok(cfg)=>{self.config=cfg;self.error=self.persist().err();},Err(e)=>self.error=Some(e)}}
                    ui.small(language.text("0.0.0.0 = toutes les interfaces. Annoncer l'IP locale accessible par la console.", "0.0.0.0 = all interfaces. Advertise the local IP reachable by the console."));
                });
            });
            ui.small(language.text("DNS et HTTP locaux uniquement. La configuration réseau du PC et du routeur reste manuelle.", "Local DNS and HTTP only. PC and router network settings are configured manually."));
            if let Some(error)=self.error.clone(){ui.horizontal_wrapped(|ui|{ui.colored_label(Color32::from_rgb(255,181,104),language.message(&error));if ui.small_button(language.text("Effacer le message", "Dismiss")).clicked(){self.error=None;}});}
            ui.separator();
            let entries=self.history.snapshot();
            ui.horizontal(|ui|{ui.heading(language.text("Historique", "History"));ui.label(format!("{} / 5 000",entries.len()));ui.label(language.text("Filtrer", "Filter"));ui.text_edit_singleline(&mut self.filter);if ui.button(language.text("Effacer", "Clear")).clicked(){self.history.clear();}});
            ui.small(language.text("Heures en UTC • historique conservé en mémoire pendant cette session", "UTC timestamps • history is kept in memory for this session"));
            let filter=self.filter.to_lowercase();
            egui::ScrollArea::both().auto_shrink([false,false]).show(ui,|ui|{
                egui::Grid::new("requests").striped(true).spacing([14.0,6.0]).show(ui,|ui|{
                    for title in [language.text("Date / heure UTC", "Date / time UTC"),language.text("Client", "Client"),language.text("Service", "Service"),language.text("Domaine / chemin", "Domain / path"),language.text("Type", "Type"),language.text("Résultat", "Result")]{ui.strong(title);}ui.end_row();
                    for entry in entries.iter().rev().filter(|e|format!("{} {} {} {} {} {} {} {} {}",e.client,e.protocol,e.target,e.kind,e.result,language.message(&e.kind),language.message(&e.result),language.message(&e.client),language.history_target(&e.target)).to_lowercase().contains(&filter)){
                        ui.monospace(timestamp(entry.time));ui.label(language.message(&entry.client));ui.label(&entry.protocol);ui.label(language.history_target(&entry.target));ui.label(language.message(&entry.kind));ui.label(language.message(&entry.result));ui.end_row();
                    }
                });
            });
        });
    }
}
/// Subtract button rectangles from the header to avoid intercepting clicks.
fn header_drag_regions<const N: usize>(
    header: egui::Rect,
    buttons: [egui::Rect; N],
) -> Vec<egui::Rect> {
    let mut regions = vec![header];
    for button in buttons {
        let mut remaining = Vec::new();
        for region in regions {
            let cut = region.intersect(button);
            if !cut.is_positive() {
                remaining.push(region);
                continue;
            }
            let parts = [
                egui::Rect::from_min_max(region.min, egui::pos2(region.right(), cut.top())),
                egui::Rect::from_min_max(egui::pos2(region.left(), cut.bottom()), region.max),
                egui::Rect::from_min_max(
                    egui::pos2(region.left(), cut.top()),
                    egui::pos2(cut.left(), cut.bottom()),
                ),
                egui::Rect::from_min_max(
                    egui::pos2(cut.right(), cut.top()),
                    egui::pos2(region.right(), cut.bottom()),
                ),
            ];
            remaining.extend(parts.into_iter().filter(egui::Rect::is_positive));
        }
        regions = remaining;
    }
    regions
}
// Gregorian date from Unix days, avoiding platform-specific time APIs.
fn timestamp(time: std::time::SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let z = (secs / 86400) as i64 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        secs / 3600 % 24,
        secs / 60 % 60,
        secs % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::App as _;

    fn app() -> (App, mpsc::Sender<Action>) {
        let (tx, actions) = mpsc::channel();
        let config = Config::default();
        (
            App {
                listen: config.listen.to_string(),
                answer: config.answer.to_string(),
                config,
                language: Language::Fr,
                language_override: None,
                history: History::default(),
                server: None,
                tray: None,
                actions,
                error: None,
                filter: String::new(),
                quitting: false,
                path: None,
            },
            tx,
        )
    }

    #[test]
    fn tray_open_is_processed_without_painting_the_hidden_window() {
        let (mut app, tx) = app();
        let ctx = egui::Context::default();
        let mut frame = eframe::Frame::_new_kittest();
        tx.send(Action::Show).unwrap();
        let output = ctx.run_logic(&egui::RawInput::default(), |ctx| app.logic(ctx, &mut frame));
        let commands = output
            .viewport_commands
            .get(&egui::ViewportId::ROOT)
            .unwrap();
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::Visible(true)))
        );
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::Focus))
        );
    }

    #[test]
    fn missing_tray_keeps_window_accessible() {
        let (mut app, _) = app();
        let ctx = egui::Context::default();
        let output = ctx.run_logic(&egui::RawInput::default(), |ctx| app.hide(ctx));
        assert!(app.error.is_some());
        assert!(
            !output
                .viewport_commands
                .values()
                .flatten()
                .any(|c| matches!(c, egui::ViewportCommand::Visible(false)))
        );
    }

    #[test]
    fn tray_quit_is_processed_without_a_ui_pass() {
        let (mut app, tx) = app();
        let ctx = egui::Context::default();
        let mut frame = eframe::Frame::_new_kittest();
        tx.send(Action::Quit).unwrap();
        let output = ctx.run_logic(&egui::RawInput::default(), |ctx| app.logic(ctx, &mut frame));
        assert!(app.quitting);
        assert!(
            output
                .viewport_commands
                .values()
                .flatten()
                .any(|c| matches!(c, egui::ViewportCommand::Close))
        );
    }

    #[test]
    fn history_timestamps_include_utc_date() {
        assert_eq!(timestamp(UNIX_EPOCH), "1970-01-01 00:00:00");
        assert_eq!(
            timestamp(UNIX_EPOCH + Duration::from_secs(1709251199)),
            "2024-02-29 23:59:59"
        );
    }
    #[test]
    fn full_ipv4_addresses_fit_in_both_fields() {
        const IP: &str = "255.255.255.254";
        fn check(shape: &egui::epaint::Shape, clip: egui::Rect, found: &mut usize) {
            match shape {
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        check(shape, clip, found);
                    }
                }
                egui::epaint::Shape::Text(text) if text.galley.job.text == IP => {
                    *found += 1;
                    assert!(
                        clip.width() >= text.galley.size().x,
                        "IP clipped: text {} px, field {} px",
                        text.galley.size().x,
                        clip.width()
                    );
                }
                _ => {}
            }
        }
        for width in [720.0, 1000.0] {
            let (mut app, _) = app();
            app.listen = IP.into();
            app.answer = IP.into();
            let ctx = egui::Context::default();
            let mut frame = eframe::Frame::_new_kittest();
            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 650.0),
                    )),
                    ..Default::default()
                };
                let mut output = ctx.run_ui(input, |root| app.ui(root, &mut frame));
                output.textures_delta.clear();
                let mut found = 0;
                for shape in output.shapes {
                    check(&shape.shape, shape.clip_rect, &mut found);
                }
                assert_eq!(found, 2, "Both IP fields must be rendered");
            }
        }
    }
    #[test]
    fn saved_port_overrides_are_ignored_and_not_written() {
        let old =
            r#"{"listen":"192.168.1.27","answer":"192.168.1.27","dns_port":5353,"http_port":8080}"#;
        let config: Config = serde_json::from_str(old).unwrap();
        assert_eq!(config.dns_port, 53);
        assert_eq!(config.http_port, 80);
        let saved = serde_json::to_value(&config).unwrap();
        assert!(saved.get("dns_port").is_none());
        assert!(saved.get("http_port").is_none());
    }

    #[test]
    fn application_always_starts_with_standard_ports() {
        let (mut app, _) = app();
        app.config.dns_port = 5353;
        app.config.http_port = 8080;
        let config = app.parse_config().unwrap();
        assert_eq!(config.dns_port, 53);
        assert_eq!(config.http_port, 80);
    }
    fn pointer_frame(
        app: &mut App,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 650.0),
            )),
            events,
            ..Default::default()
        };
        let mut frame = eframe::Frame::_new_kittest();
        let mut output = ctx.run_ui(input, |root| app.ui(root, &mut frame));
        output.textures_delta.clear();
        output
    }
    fn has_window_drag(output: &egui::FullOutput) -> bool {
        output
            .viewport_output
            .values()
            .flat_map(|v| &v.commands)
            .any(|c| matches!(c, egui::ViewportCommand::StartDrag))
    }
    fn pointer_button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }
    #[test]
    fn empty_header_and_its_padding_can_drag_the_window() {
        for y in [6.0, 22.0, 42.0] {
            let (mut app, _) = app();
            let ctx = egui::Context::default();
            for _ in 0..2 {
                pointer_frame(&mut app, &ctx, vec![]);
            }
            let pos = egui::pos2(500.0, y);
            let down = pointer_frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), pointer_button(pos, true)],
            );
            let moved = pointer_frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos + egui::vec2(30.0, 0.0))],
            );
            assert!(
                has_window_drag(&down) || has_window_drag(&moved),
                "No window drag from empty header at y={y}"
            );
        }
    }
    #[test]
    fn header_buttons_stay_clickable_without_dragging_the_window() {
        fn find_text(shape: &egui::epaint::Shape, text: &str) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.job.text == text => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(|s| find_text(s, text)),
                _ => None,
            }
        }
        for button in ["Masquer", "Quitter"] {
            let (mut app, _) = app();
            let ctx = egui::Context::default();
            pointer_frame(&mut app, &ctx, vec![]);
            let output = pointer_frame(&mut app, &ctx, vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|s| find_text(&s.shape, button))
                .unwrap();
            let down = pointer_frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), pointer_button(pos, true)],
            );
            let released = pointer_frame(&mut app, &ctx, vec![pointer_button(pos, false)]);
            assert!(!has_window_drag(&down) && !has_window_drag(&released));
            if button == "Quitter" {
                assert!(app.quitting);
            } else {
                assert!(app.error.is_some());
                assert!(!app.quitting);
            }
        }
    }
    #[test]
    fn language_switch_preserves_running_services_and_saves_preference() {
        let (mut app, _) = app();
        let path = std::env::temp_dir().join(format!(
            "wifinx-language-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        app.path = Some(path.clone());
        let network = Config {
            listen: Ipv4Addr::LOCALHOST,
            dns_port: 0,
            http_port: 0,
            ..Config::default()
        };
        app.server = Some(Server::start(network, app.history.clone()).unwrap());
        let dns = app.server.as_ref().unwrap().dns_addr();
        app.change_language(Language::En);
        assert_eq!(app.language, Language::En);
        assert!(app.server.as_ref().unwrap().is_running());
        assert_eq!(app.server.as_ref().unwrap().dns_addr(), dns);
        let saved: Settings =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved.language, Some(Language::En));
        drop(app);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn language_button_is_left_of_hide_and_changes_the_whole_ui() {
        fn text_pos(shape: &egui::epaint::Shape, wanted: &str) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.job.text == wanted => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(|s| text_pos(s, wanted)),
                _ => None,
            }
        }
        let (mut app, _) = app();
        let ctx = egui::Context::default();
        let path = std::env::temp_dir().join(format!(
            "wifinx-ui-language-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        app.path = Some(path.clone());
        pointer_frame(&mut app, &ctx, vec![]);
        let out = pointer_frame(&mut app, &ctx, vec![]);
        let find =
            |out: &egui::FullOutput, text| out.shapes.iter().find_map(|s| text_pos(&s.shape, text));
        let toggle = find(&out, "EN").unwrap();
        assert!(toggle.x < find(&out, "Masquer").unwrap().x);
        let down = pointer_frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(toggle),
                pointer_button(toggle, true),
            ],
        );
        let up = pointer_frame(&mut app, &ctx, vec![pointer_button(toggle, false)]);
        assert!(!has_window_drag(&down) && !has_window_drag(&up));
        assert_eq!(app.language, Language::En);
        let out = pointer_frame(&mut app, &ctx, vec![]);
        for label in [
            "Hide",
            "Quit",
            "History",
            "Listen address",
            "Advertised PC IP",
            "Save settings",
            "FR",
        ] {
            assert!(
                find(&out, label).is_some(),
                "Missing English label: {label}"
            );
        }
        assert!(find(&out, "Masquer").is_none());
        let toggle = find(&out, "FR").unwrap();
        assert!(toggle.x < find(&out, "Hide").unwrap().x);
        pointer_frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(toggle),
                pointer_button(toggle, true),
            ],
        );
        pointer_frame(&mut app, &ctx, vec![pointer_button(toggle, false)]);
        assert_eq!(app.language, Language::Fr);
        let saved: Settings =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved.language, Some(Language::Fr));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn english_history_filter_matches_the_displayed_invalid_packet() {
        fn contains(shape: &egui::epaint::Shape) -> bool {
            match shape {
                egui::epaint::Shape::Text(t) => t.galley.job.text == "Invalid packet",
                egui::epaint::Shape::Vec(shapes) => shapes.iter().any(contains),
                _ => false,
            }
        }
        let (mut app, _) = app();
        let ctx = egui::Context::default();
        app.language = Language::En;
        app.filter = "Invalid packet".into();
        app.history.push(wifinx_validator::history::Entry::new(
            "127.0.0.1",
            "DNS UDP",
            "Paquet invalide",
            "?",
            "En-tête DNS invalide",
        ));
        pointer_frame(&mut app, &ctx, vec![]);
        let output = pointer_frame(&mut app, &ctx, vec![]);
        // The search field itself also contains the query; the matching table
        // target must produce a second copy for the filtered request row.
        let count = output.shapes.iter().filter(|s| contains(&s.shape)).count();
        assert!(
            count >= 2,
            "The visible English target must match the history filter"
        );
    }
}
