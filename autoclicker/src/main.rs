//! Точка входа: логгер, общие настройки, потоки, трей, GUI.
//!
//! Release-сборка — без консоли (`windows_subsystem = "windows"`), один .exe
//! со встроенной иконкой. Фатальные ошибки показываются диалогом (консоли нет).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// На не-Windows собирается только логика для юнит-тестов — часть кода не используется.
#![cfg_attr(not(windows), allow(dead_code))]

mod clicker;
mod keys;
mod settings;
mod winapi;

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod tray;

fn main() {
    if let Err(err) = run() {
        log::error!("Фатальная ошибка: {err:#}");
        winapi::fatal_dialog(&format!("Не удалось запустить автокликер:\n\n{err:#}"));
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn run() -> anyhow::Result<()> {
    use anyhow::Context as _;

    init_logging();

    let settings = settings::SharedSettings::new();
    let shutdown = settings::Shutdown::new();
    let listen = clicker::ListenRequest::default();
    let (ui_tx, ui_rx) = std::sync::mpsc::channel();

    let clicker_thread = clicker::spawn(
        settings.clone(),
        shutdown.clone(),
        listen.clone(),
        ui_tx.clone(),
    )
    .context("не удалось запустить поток кликера")?;
    let watcher_thread = clicker::spawn_emergency_watcher(shutdown.clone(), listen.clone())
        .context("не удалось запустить поток аварийного выхода")?;
    drop(ui_tx); // канал закроется, когда завершатся оба потока

    // Трей — на главном потоке до run_native: message pump даст winit (см. tray.rs).
    let tray = tray::build().context("system tray")?;

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Автокликер")
        .with_inner_size([380.0, 300.0])
        .with_resizable(false);
    if let Some(icon) = tray::window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        // Чтобы run_native вернул управление и мы корректно погасили потоки.
        run_and_return: true,
        ..Default::default()
    };

    let (app_settings, app_shutdown, app_listen) =
        (settings.clone(), shutdown.clone(), listen.clone());
    eframe::run_native(
        "Автокликер",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            Ok(Box::new(app::App::new(
                app_settings,
                app_shutdown,
                app_listen,
                ui_rx,
                tray,
            )))
        }),
    )
    // eframe::Error не Send+Sync — переносим в anyhow через Display.
    .map_err(|err| anyhow::anyhow!("GUI: {err}"))?;

    // Окно закрылось через «Выход» в трее — гасим потоки (App уже дропнут, трей убран).
    shutdown.set();
    settings.update(|s| s.enabled = false);
    join_with_timeout(clicker_thread, std::time::Duration::from_millis(600));
    join_with_timeout(watcher_thread, std::time::Duration::from_millis(300));
    log::info!("Завершено корректно");
    Ok(())
}

#[cfg(windows)]
fn init_logging() {
    let default_level = if cfg!(debug_assertions) { "info" } else { "warn" };
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .format_timestamp_secs()
        .try_init();
}

#[cfg(windows)]
fn join_with_timeout(handle: std::thread::JoinHandle<()>, timeout: std::time::Duration) {
    let deadline = std::time::Instant::now() + timeout;
    while !handle.is_finished() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if handle.is_finished() {
        let _ = handle.join();
    } else {
        log::warn!("Поток не успел завершиться за {timeout:?} — процесс выходит");
    }
}

#[cfg(not(windows))]
fn run() -> anyhow::Result<()> {
    println!("Автокликер рассчитан на Windows (Win32 API). Эта сборка — только для юнит-тестов.");
    Ok(())
}
