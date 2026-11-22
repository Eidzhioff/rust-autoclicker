//! System tray: иконка (вшита в exe) и контекстное меню «Показать окно» / «Выход».
//!
//! ВАЖНО (Windows): трей создаётся на главном потоке до `eframe::run_native` —
//! message pump обеспечивает winit. События забираются в `App::logic()` из
//! глобальных каналов `MenuEvent`/`TrayIconEvent`.

use anyhow::{Context as _, Result};
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

const ICON_BYTES: &[u8] = include_bytes!("../assets/icon.ico");

/// Живой трей + id пунктов меню для распознавания событий.
/// `TrayIcon` нельзя дропать раньше времени — иконка исчезнет из трея.
pub struct Tray {
    pub icon: TrayIcon,
    pub show_id: MenuId,
    pub quit_id: MenuId,
}

/// Собирает трей. Вызывать на главном потоке (см. модульный комментарий).
pub fn build() -> Result<Tray> {
    let show = MenuItem::new("Показать окно", true, None);
    let quit = MenuItem::new("Выход", true, None);
    let menu = Menu::new();
    menu.append_items(&[&show, &PredefinedMenuItem::separator(), &quit])
        .context("не удалось собрать меню трея")?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        // ЛКМ по иконке — показать окно (обрабатываем сами), ПКМ — меню.
        .with_menu_on_left_click(false)
        .with_tooltip("Автокликер — остановлен")
        .with_icon(load_icon()?)
        .build()
        .context("не удалось создать иконку трея")?;

    Ok(Tray {
        icon: tray,
        show_id: show.id().clone(),
        quit_id: quit.id().clone(),
    })
}

fn load_icon() -> Result<Icon> {
    let image = image::load_from_memory(ICON_BYTES)
        .context("не удалось декодировать встроенную icon.ico")?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).context("некорректные RGBA-данные иконки")
}

/// Иконка для заголовка окна/панели задач (RGBA из того же встроенного .ico).
pub fn window_icon() -> Option<eframe::egui::IconData> {
    let image = image::load_from_memory(ICON_BYTES).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some(eframe::egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}
