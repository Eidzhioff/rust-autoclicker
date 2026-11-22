//! Фоновый поток кликера, режим «Слушать» (захват кнопки) и аварийный watcher F12.
//!
//! Кликер и watcher не зависят от живости GUI: остановка идёт через общий
//! `Shutdown`, а F12 завершает процесс даже при зависшем окне (ТЗ §2.3).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::keys;
use crate::settings::{SharedSettings, Shutdown, IDLE_POLL_MS, MIN_DELAY_MS};
use crate::winapi;

/// Таймаут режима «Слушать».
pub const LISTEN_TIMEOUT: Duration = Duration::from_secs(10);
/// Не чаще одного сообщения об ошибке клика в секунду (не спамим канал/GUI).
const ERROR_REPORT_INTERVAL: Duration = Duration::from_secs(1);
/// Столько ошибок SendInput подряд — авто-остановка кликера.
const MAX_CONSECUTIVE_ERRORS: u32 = 20;
/// Период опроса F12 аварийным watcher'ом.
const EMERGENCY_POLL: Duration = Duration::from_millis(30);
/// Grace-период перед жёстким exit при аварийном выходе.
const EMERGENCY_GRACE: Duration = Duration::from_millis(100);

/// События из фоновых потоков в GUI.
#[derive(Debug)]
pub enum UiEvent {
    /// В режиме «Слушать» захвачен VK-код.
    Captured(u16),
    /// Режим «Слушать» завершён без захвата (отмена/таймаут).
    ListenCancelled,
    /// Ошибка симуляции клика (троттлинг — не чаще 1/с).
    ClickError(String),
    /// Кликер сам остановился после серии ошибок.
    AutoStopped(String),
}

#[derive(Default)]
struct ListenInner {
    requested: AtomicBool,
    cancelled: AtomicBool,
    active: AtomicBool,
}

/// Управляемый из GUI запрос на захват следующей нажатой кнопки.
#[derive(Clone, Default)]
pub struct ListenRequest(Arc<ListenInner>);

impl ListenRequest {
    /// Начать ожидание нажатия (сбрасывает прежнюю отмену).
    pub fn start(&self) {
        self.0.cancelled.store(false, Ordering::SeqCst);
        self.0.requested.store(true, Ordering::SeqCst);
    }

    /// Отменить ожидание (кнопка «Отмена», Esc, повторный клик).
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::SeqCst);
    }

    /// Идёт ли захват прямо сейчас (watcher F12 на это время приостанавливается).
    pub fn is_active(&self) -> bool {
        self.0.active.load(Ordering::SeqCst)
    }

    fn take_request(&self) -> bool {
        self.0.requested.swap(false, Ordering::SeqCst)
    }

    fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::SeqCst)
    }

    fn set_active(&self, value: bool) {
        self.0.active.store(value, Ordering::SeqCst);
    }
}

/// Запускает daemon-поток кликера. Живёт всё время работы приложения;
/// активность гейтится `settings.enabled`, старт/стоп не пересоздаёт поток.
pub fn spawn(
    settings: SharedSettings,
    shutdown: Shutdown,
    listen: ListenRequest,
    ui_tx: Sender<UiEvent>,
) -> anyhow::Result<JoinHandle<()>> {
    let handle = thread::Builder::new()
        .name("clicker".into())
        .spawn(move || run(settings, shutdown, listen, ui_tx))?;
    Ok(handle)
}

fn run(settings: SharedSettings, shutdown: Shutdown, listen: ListenRequest, ui_tx: Sender<UiEvent>) {
    winapi::set_high_timer_resolution(true);
    let mut consecutive_errors = 0u32;
    let mut last_error_report: Option<Instant> = None;

    while !shutdown.is_set() {
        if listen.take_request() {
            consecutive_errors = 0;
            capture_key(&shutdown, &listen, &ui_tx);
            continue;
        }

        let snapshot = settings.snapshot();
        if snapshot.enabled && winapi::is_pressed(snapshot.vk) {
            match winapi::send_left_click() {
                Ok(()) => consecutive_errors = 0,
                Err(err) => {
                    consecutive_errors += 1;
                    let due = last_error_report
                        .is_none_or(|t| t.elapsed() >= ERROR_REPORT_INTERVAL);
                    if due {
                        let _ = ui_tx.send(UiEvent::ClickError(err.to_string()));
                        last_error_report = Some(Instant::now());
                    }
                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        settings.update(|s| s.enabled = false);
                        let _ = ui_tx.send(UiEvent::AutoStopped(format!(
                            "Кликер остановлен после {MAX_CONSECUTIVE_ERRORS} ошибок подряд: {err}"
                        )));
                        consecutive_errors = 0;
                    }
                }
            }
            thread::sleep(Duration::from_millis(snapshot.delay_ms.max(MIN_DELAY_MS)));
        } else {
            thread::sleep(Duration::from_millis(IDLE_POLL_MS));
        }
    }

    winapi::set_high_timer_resolution(false);
}

fn capture_key(shutdown: &Shutdown, listen: &ListenRequest, ui_tx: &Sender<UiEvent>) {
    listen.set_active(true);
    let captured = capture_loop(shutdown, listen, Instant::now() + LISTEN_TIMEOUT);
    listen.set_active(false);
    let event = match captured {
        Some(vk) => UiEvent::Captured(vk),
        None => UiEvent::ListenCancelled,
    };
    let _ = ui_tx.send(event);
}

/// Двухфазный захват: сначала дождаться отпускания всех кнопок (иначе поймаем
/// клик по самой кнопке «Слушать»), затем поймать первое нажатие. Esc — отмена.
fn capture_loop(shutdown: &Shutdown, listen: &ListenRequest, deadline: Instant) -> Option<u16> {
    loop {
        if listen.is_cancelled() || shutdown.is_set() || Instant::now() >= deadline {
            return None;
        }
        if !any_pressed() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    loop {
        if listen.is_cancelled() || shutdown.is_set() || Instant::now() >= deadline {
            return None;
        }
        if winapi::is_pressed(keys::VK_ESCAPE) {
            return None;
        }
        for vk in 0x01..=0xFEu16 {
            if vk == keys::VK_ESCAPE {
                continue;
            }
            if winapi::is_pressed(vk) {
                return Some(vk);
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn any_pressed() -> bool {
    (0x01..=0xFEu16).any(winapi::is_pressed)
}

/// Независимый поток аварийного выхода: F12 останавливает всё и завершает
/// процесс, даже если GUI завис. Намеренно «грязный» exit — это фича (ТЗ §2.3).
pub fn spawn_emergency_watcher(shutdown: Shutdown, listen: ListenRequest) -> anyhow::Result<JoinHandle<()>> {
    let handle = thread::Builder::new()
        .name("emergency".into())
        .spawn(move || {
            while !shutdown.is_set() {
                // Во время захвата кнопки F12 можно выбрать как кнопку активации —
                // на это время watcher приостановлен (захват ограничен таймаутом).
                if !listen.is_active() && winapi::is_pressed(keys::EMERGENCY_VK) {
                    log::warn!("F12: экстренный выход");
                    shutdown.set();
                    thread::sleep(EMERGENCY_GRACE);
                    std::process::exit(0);
                }
                thread::sleep(EMERGENCY_POLL);
            }
        })?;
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listen_request_lifecycle() {
        let listen = ListenRequest::default();
        assert!(!listen.take_request());
        listen.start();
        assert!(listen.take_request());
        assert!(!listen.take_request(), "запрос одноразовый");
    }

    #[test]
    fn cancel_flag_semantics() {
        let listen = ListenRequest::default();
        assert!(!listen.is_cancelled());
        listen.cancel();
        assert!(listen.is_cancelled());
        listen.start();
        assert!(!listen.is_cancelled(), "start сбрасывает отмену");
    }

    #[test]
    fn active_flag_semantics() {
        let listen = ListenRequest::default();
        assert!(!listen.is_active());
        listen.set_active(true);
        assert!(listen.is_active());
        listen.set_active(false);
        assert!(!listen.is_active());
    }

    #[test]
    fn capture_aborts_on_cancel() {
        let shutdown = Shutdown::new();
        let listen = ListenRequest::default();
        listen.cancel();
        let result = capture_loop(&shutdown, &listen, Instant::now() + Duration::from_secs(5));
        assert_eq!(result, None);
    }

    #[test]
    fn capture_aborts_on_shutdown() {
        let shutdown = Shutdown::new();
        shutdown.set();
        let listen = ListenRequest::default();
        let result = capture_loop(&shutdown, &listen, Instant::now() + Duration::from_secs(5));
        assert_eq!(result, None);
    }

    #[test]
    fn capture_times_out_when_nothing_pressed() {
        // Заглушки winapi на не-Windows: ничего не нажато → таймаут.
        let shutdown = Shutdown::new();
        let listen = ListenRequest::default();
        let result = capture_loop(&shutdown, &listen, Instant::now() + Duration::from_millis(60));
        assert_eq!(result, None);
    }
}
