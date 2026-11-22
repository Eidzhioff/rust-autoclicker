//! GUI (egui/eframe).
//!
//! Вся не-UI логика — в `logic()`: eframe 0.37 вызывает её и при скрытом окне
//! (~каждые 100 мс), поэтому события трея и кликера обрабатываются даже в фоне.
//! Отрисовка — в `ui()`.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use eframe::egui;
use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};

use crate::clicker::{ListenRequest, UiEvent};
use crate::keys;
use crate::settings::{SharedSettings, Shutdown, MAX_DELAY_MS, MIN_DELAY_MS};
use crate::tray::Tray;

/// Период принудительного опроса каналов (GUI не должен «засыпать» в трее).
const POLL_INTERVAL: Duration = Duration::from_millis(100);
/// Верхняя граница быстрого слайдера; точное значение до 10000 — через DragValue.
const SLIDER_MAX_MS: u64 = 200;

pub struct App {
    settings: SharedSettings,
    shutdown: Shutdown,
    listen: ListenRequest,
    ui_rx: Receiver<UiEvent>,
    tray: Tray,
    /// GUI-зеркало режима «Слушать» (сбрасывается событием из кликера).
    listening: bool,
    /// Начат выход через «Выход» в трее — крестик больше не перехватываем.
    exiting: bool,
    last_error: Option<String>,
}

impl App {
    pub fn new(
        settings: SharedSettings,
        shutdown: Shutdown,
        listen: ListenRequest,
        ui_rx: Receiver<UiEvent>,
        tray: Tray,
    ) -> Self {
        Self {
            settings,
            shutdown,
            listen,
            ui_rx,
            tray,
            listening: false,
            exiting: false,
            last_error: None,
        }
    }

    fn drain_ui_events(&mut self) {
        loop {
            match self.ui_rx.try_recv() {
                Ok(UiEvent::Captured(vk)) => {
                    self.listening = false;
                    self.settings.update(|s| s.vk = vk);
                    log::info!("Кнопка активации: {} (0x{vk:02X})", keys::name_for(vk));
                }
                Ok(UiEvent::ListenCancelled) => self.listening = false,
                Ok(UiEvent::ClickError(err)) => {
                    log::error!("{err}");
                    self.last_error = Some(err);
                }
                Ok(UiEvent::AutoStopped(err)) => {
                    log::error!("{err}");
                    self.last_error = Some(err);
                    self.sync_tooltip();
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    fn drain_tray_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id() == &self.tray.show_id {
                self.show_window(ctx);
            } else if event.id() == &self.tray.quit_id {
                self.request_exit();
            }
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
                | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => self.show_window(ctx),
                _ => {}
            }
        }
    }

    fn show_window(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn request_exit(&mut self) {
        log::info!("Выход по команде из трея");
        self.exiting = true;
        self.shutdown.set();
        self.settings.update(|s| s.enabled = false);
    }

    fn sync_tooltip(&self) {
        let state = if self.settings.snapshot().enabled {
            "активен"
        } else {
            "остановлен"
        };
        if let Err(err) = self.tray.icon.set_tooltip(Some(format!("Автокликер — {state}"))) {
            log::warn!("Не удалось обновить tooltip трея: {err}");
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_ui_events();
        self.drain_tray_events(ctx);

        if self.exiting {
            // «Выход» в трее: закрываем окно по-настоящему (без перехвата).
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if ctx.input(|i| i.viewport().close_requested()) {
            // Крестик = свернуть в трей, процесс продолжает работать (ТЗ §2.2).
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            log::info!("Окно скрыто в трей");
        }

        // Гарантированный следующий проход даже без ввода (опрос каналов).
        ctx.request_repaint_after(POLL_INTERVAL);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(6.0);
            let before = self.settings.snapshot();
            let mut current = before;

            // --- Кнопка активации ---
            ui.horizontal(|ui| {
                ui.label("Кнопка активации:");
                ui.strong(keys::name_for(current.vk));
            });
            ui.add_space(2.0);
            if self.listening {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Нажмите клавишу или кнопку мыши…");
                    if ui.button("Отмена").clicked() {
                        self.listen.cancel();
                    }
                });
                ui.small("Esc — отмена · таймаут 10 секунд");
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.listen.cancel();
                }
            } else if ui.button("Слушать").clicked() {
                self.listening = true;
                self.listen.start();
            }
            if current.vk == keys::EMERGENCY_VK {
                ui.colored_label(
                    egui::Color32::from_rgb(230, 180, 60),
                    "F12 занят аварийным выходом — выберите другую кнопку",
                );
            }
            if current.vk == keys::VK_LBUTTON {
                ui.colored_label(
                    egui::Color32::from_rgb(230, 180, 60),
                    "ЛКМ как активация: удерживайте её вне этого окна",
                );
            }

            ui.separator();

            // --- Задержка ---
            ui.horizontal(|ui| {
                ui.label("Задержка, мс:");
                ui.add(
                    egui::DragValue::new(&mut current.delay_ms)
                        .range(MIN_DELAY_MS..=MAX_DELAY_MS)
                        .speed(1.0),
                );
                ui.small(format!("({MIN_DELAY_MS}–{MAX_DELAY_MS})"));
            });
            ui.add(
                egui::Slider::new(&mut current.delay_ms, MIN_DELAY_MS..=SLIDER_MAX_MS)
                    .text("мс"),
            );

            ui.separator();

            // --- Старт/Стоп ---
            let (label, fill) = if current.enabled {
                ("Стоп", egui::Color32::from_rgb(178, 58, 58))
            } else {
                ("Старт", egui::Color32::from_rgb(46, 125, 70))
            };
            let button = egui::Button::new(
                egui::RichText::new(label)
                    .size(18.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .fill(fill)
            .min_size(egui::vec2(ui.available_width(), 34.0));
            if ui.add(button).clicked() {
                current.enabled = !current.enabled;
                self.last_error = None;
            }

            // Применяем изменения сразу, без кнопки «Применить».
            if current != before {
                self.settings.update(|s| *s = current);
                self.sync_tooltip();
            }

            // --- Статус ---
            ui.add_space(4.0);
            if let Some(err) = &self.last_error {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(err)
                            .small()
                            .color(egui::Color32::from_rgb(235, 110, 110)),
                    )
                    .wrap(),
                );
            } else if current.enabled {
                ui.colored_label(
                    egui::Color32::from_rgb(120, 200, 140),
                    format!("Активен: кликаю ЛКМ, пока зажата «{}»", keys::name_for(current.vk)),
                );
            } else {
                ui.label("Остановлен");
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.small("F12 — экстренный выход · крестик сворачивает в трей");
            });
        });
    }
}
