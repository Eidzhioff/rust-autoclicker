//! Настройки, разделяемые между GUI и потоком кликера.
//!
//! `Arc<Mutex<Settings>>` по ТЗ; лок удерживается только на копирование снимка
//! (структура — три Copy-поля), так что горячий цикл кликера не блокирует GUI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::keys;

/// Кнопка активации по умолчанию — XButton2, как в Python-прототипе.
pub const DEFAULT_VK: u16 = keys::VK_XBUTTON2;
/// Задержка по умолчанию, мс (ТЗ §2.1).
pub const DEFAULT_DELAY_MS: u64 = 40;
/// Минимальная задержка, мс (ТЗ §2.1).
pub const MIN_DELAY_MS: u64 = 1;
/// Верхняя граница задержки, мс (защита от абсурдных значений в GUI).
pub const MAX_DELAY_MS: u64 = 10_000;
/// Период idle-опроса, когда кнопка не зажата (как в прототипе).
pub const IDLE_POLL_MS: u64 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// VK-код кнопки активации.
    pub vk: u16,
    /// Задержка между кликами, мс.
    pub delay_ms: u64,
    /// Кликер включён (кнопка «Старт/Стоп»).
    pub enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            vk: DEFAULT_VK,
            delay_ms: DEFAULT_DELAY_MS,
            enabled: false,
        }
    }
}

impl Settings {
    /// Приводит значения к допустимым границам (применяется и в GUI, и в кликере).
    pub fn sanitized(mut self) -> Self {
        self.delay_ms = self.delay_ms.clamp(MIN_DELAY_MS, MAX_DELAY_MS);
        if !keys::is_valid_activation(self.vk) {
            self.vk = DEFAULT_VK;
        }
        self
    }
}

/// Разделяемые настройки: GUI пишет, кликер читает снимки.
#[derive(Clone)]
pub struct SharedSettings(Arc<Mutex<Settings>>);

impl SharedSettings {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Settings::default())))
    }

    /// Отравленный мьютекс не должен ронять приложение — восстанавливаем содержимое.
    fn lock(&self) -> MutexGuard<'_, Settings> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Копия текущих настроек (короткий лок).
    pub fn snapshot(&self) -> Settings {
        *self.lock()
    }

    /// Изменение с автоматической санацией значений.
    pub fn update(&self, f: impl FnOnce(&mut Settings)) {
        let mut guard = self.lock();
        f(&mut guard);
        *guard = guard.sanitized();
    }
}

impl Default for SharedSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Общий флаг остановки для всех потоков (graceful shutdown и аварийный выход).
#[derive(Clone, Default)]
pub struct Shutdown(Arc<AtomicBool>);

impl Shutdown {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        let s = Settings::default();
        assert_eq!(s.vk, 0x06);
        assert_eq!(s.delay_ms, 40);
        assert!(!s.enabled);
    }

    #[test]
    fn delay_is_clamped() {
        let shared = SharedSettings::new();
        shared.update(|s| s.delay_ms = 0);
        assert_eq!(shared.snapshot().delay_ms, MIN_DELAY_MS);
        shared.update(|s| s.delay_ms = u64::MAX);
        assert_eq!(shared.snapshot().delay_ms, MAX_DELAY_MS);
        shared.update(|s| s.delay_ms = 120);
        assert_eq!(shared.snapshot().delay_ms, 120);
    }

    #[test]
    fn invalid_vk_falls_back_to_default() {
        let shared = SharedSettings::new();
        shared.update(|s| s.vk = 0x00);
        assert_eq!(shared.snapshot().vk, DEFAULT_VK);
        shared.update(|s| s.vk = 0xFF);
        assert_eq!(shared.snapshot().vk, DEFAULT_VK);
        shared.update(|s| s.vk = 0x41);
        assert_eq!(shared.snapshot().vk, 0x41);
    }

    #[test]
    fn concurrent_updates_keep_invariants() {
        let shared = SharedSettings::new();
        let mut handles = Vec::new();
        for t in 0..4u64 {
            let shared = shared.clone();
            handles.push(std::thread::spawn(move || {
                for i in 0..1000u64 {
                    shared.update(|s| {
                        s.delay_ms = (i + t) % 500;
                        s.enabled = i % 2 == 0;
                    });
                    let snap = shared.snapshot();
                    assert!((MIN_DELAY_MS..=MAX_DELAY_MS).contains(&snap.delay_ms));
                }
            }));
        }
        for h in handles {
            h.join().expect("test thread");
        }
    }

    #[test]
    fn shutdown_flag() {
        let flag = Shutdown::new();
        assert!(!flag.is_set());
        let clone = flag.clone();
        flag.set();
        assert!(clone.is_set());
    }
}
