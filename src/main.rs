#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod app;
mod i18n;
#[cfg(target_os = "linux")]
mod privileges;
mod startup;
mod tray;
fn main() -> std::process::ExitCode {
    let log = match startup::StartupLog::open_default() {
        Ok(log) => log,
        Err(error) => {
            eprintln!("Journal de démarrage indisponible : {error}");
            startup::StartupLog::unavailable()
        }
    };
    let _ = log.record(
        "START",
        &format!(
            "WifiNx Validator {} / {} / {}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
    );
    let panic_log = log.clone();
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let report = panic_log.failure(&format!("Erreur interne : {info}"));
        previous_hook(info);
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        startup::show_error(&report);
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        eprintln!("{report}");
    }));
    match run(log.clone()) {
        Ok(()) => {
            let _ = log.record("EXIT", "Fermeture normale");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            let message = log.failure(&format!("{error:?}"));
            eprintln!("{message}");
            startup::show_error(&message);
            std::process::ExitCode::FAILURE
        }
    }
}
fn run(log: startup::StartupLog) -> eframe::Result {
    #[cfg(target_os = "linux")]
    if let Err(message) = privileges::prepare() {
        use gtk::prelude::*;
        if gtk::init().is_ok() {
            let language = i18n::initial_language();
            let dialog = gtk::MessageDialog::builder()
                .message_type(gtk::MessageType::Error)
                .buttons(gtk::ButtonsType::Close)
                .text(language.text(
                    "Autorisation nécessaire pour WifiNx Validator",
                    "WifiNx Validator needs authorization",
                ))
                .secondary_text(language.message(&message))
                .build();
            dialog.run();
            dialog.close();
        }
        return Err(eframe::Error::AppCreation(
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, message).into(),
        ));
    }
    let options = native_options(Some(log.clone()));
    let _ = log.record(
        "RENDERER",
        if cfg!(target_os = "windows") {
            "wgpu / Direct3D 12"
        } else if cfg!(target_os = "macos") {
            "wgpu / Metal"
        } else {
            "OpenGL"
        },
    );
    eframe::run_native(
        &format!("WifiNx Validator {}", env!("CARGO_PKG_VERSION")),
        options,
        Box::new(move |cc| {
            let _ = log.record("APP", "Création de la fenêtre et de la notification");
            let app = app::App::new(cc);
            let _ = log.record("READY", "Interface créée");
            Ok(Box::new(app))
        }),
    )
}
fn native_options(log: Option<startup::StartupLog>) -> eframe::NativeOptions {
    let options = eframe::NativeOptions {
        #[cfg(target_os = "linux")]
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([1000.0, 650.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    #[cfg(target_os = "windows")]
    {
        let mut options = options;
        options.renderer = eframe::Renderer::Wgpu;
        if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup
        {
            // Avoid OpenGL entirely. DXGI also enumerates the Windows WARP
            // software adapter, which wgpu can select when no GPU is suitable.
            setup.instance_descriptor.backends = eframe::wgpu::Backends::DX12;
            setup.native_adapter_selector = Some(std::sync::Arc::new(move |adapters, surface| {
                let candidates: Vec<_> = adapters
                    .iter()
                    .map(|adapter| {
                        (
                            adapter.get_info().device_type == eframe::wgpu::DeviceType::Cpu,
                            surface.is_none_or(|surface| adapter.is_surface_supported(surface)),
                        )
                    })
                    .collect();
                let index = preferred_adapter(&candidates).ok_or_else(|| {
                    "Aucun adaptateur Direct3D compatible, matériel ou WARP, n'est disponible."
                        .to_owned()
                })?;
                let adapter = adapters[index].clone();
                if let Some(log) = &log {
                    let info = adapter.get_info();
                    let _ = log.record(
                        "ADAPTER",
                        &format!(
                            "{} / {:?} / {:?}",
                            info.name, info.device_type, info.backend
                        ),
                    );
                }
                Ok(adapter)
            }));
        }
        options
    }
    #[cfg(target_os = "macos")]
    {
        let mut options = options;
        options.renderer = eframe::Renderer::Wgpu;
        if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup
        {
            setup.instance_descriptor.backends = eframe::wgpu::Backends::METAL;
        }
        let _ = log;
        options
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = log;
        options
    }
}
#[cfg(all(test, target_os = "windows"))]
mod windows_tests {
    #[test]
    fn windows_uses_direct3d_without_opengl() {
        let options = super::native_options(None);
        assert_eq!(options.renderer, eframe::Renderer::Wgpu);
        let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = options.wgpu_options.wgpu_setup else {
            panic!("Expected a new Direct3D setup");
        };
        assert_eq!(
            setup.instance_descriptor.backends,
            eframe::wgpu::Backends::DX12
        );
    }
}

#[cfg(any(test, target_os = "windows"))]
fn preferred_adapter(candidates: &[(bool, bool)]) -> Option<usize> {
    candidates
        .iter()
        .position(|(software, compatible)| *software && *compatible)
        .or_else(|| candidates.iter().position(|(_, compatible)| *compatible))
}
#[cfg(test)]
mod adapter_tests {
    #[test]
    fn software_adapter_avoids_unreliable_hardware_drivers() {
        assert_eq!(
            super::preferred_adapter(&[(false, true), (true, true)]),
            Some(1)
        );
        assert_eq!(
            super::preferred_adapter(&[(true, false), (false, true)]),
            Some(1)
        );
        assert_eq!(super::preferred_adapter(&[(false, true)]), Some(0));
        assert_eq!(super::preferred_adapter(&[(true, false)]), None);
        assert_eq!(super::preferred_adapter(&[]), None);
    }
}
