//! Единственное место с `unsafe`: безопасные обёртки над Win32 API (ТЗ §4).
//!
//! На не-Windows платформах модуль подменяется заглушками — это позволяет
//! гонять юнит-тесты логики (keys/settings/clicker) на любой ОС.

#[cfg(windows)]
mod imp {
    use std::mem::size_of;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::GetLastError;
    use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN,
        MOUSEEVENTF_LEFTUP, MOUSE_EVENT_FLAGS, MOUSEINPUT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    /// Зажата ли кнопка прямо сейчас (старший бит `GetAsyncKeyState`, как в прототипе).
    pub fn is_pressed(vk: u16) -> bool {
        // SAFETY: GetAsyncKeyState — read-only опрос состояния, предусловий нет.
        let state = unsafe { GetAsyncKeyState(i32::from(vk)) };
        (state as u16) & 0x8000 != 0
    }

    /// Одиночный клик ЛКМ через `SendInput` (down+up одним вызовом).
    pub fn send_left_click() -> anyhow::Result<()> {
        let inputs = [
            mouse_input(MOUSEEVENTF_LEFTDOWN),
            mouse_input(MOUSEEVENTF_LEFTUP),
        ];
        // SAFETY: срез валиден на время вызова; INPUT — POD-структуры корректного размера.
        let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
        if sent == inputs.len() as u32 {
            Ok(())
        } else {
            // SAFETY: GetLastError — read-only, предусловий нет.
            let code = unsafe { GetLastError() }.0;
            anyhow::bail!(
                "SendInput: отправлено {sent}/{} событий (код ошибки Windows {code})",
                inputs.len()
            );
        }
    }

    fn mouse_input(flags: MOUSE_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    /// Точность системного таймера ~1 мс, иначе `sleep(1)` на Windows спит ~15.6 мс.
    /// Вызывается кликером при старте и откатывается при завершении.
    pub fn set_high_timer_resolution(on: bool) {
        // SAFETY: timeBeginPeriod/timeEndPeriod — парные вызовы с константой 1, предусловий нет.
        let rc = unsafe {
            if on {
                timeBeginPeriod(1)
            } else {
                timeEndPeriod(1)
            }
        };
        if rc != 0 {
            log::warn!("time{}Period(1) вернул код {rc}", if on { "Begin" } else { "End" });
        }
    }

    /// Диалог фатальной ошибки: в release-сборке консоли нет, показать ошибку можно только так.
    pub fn fatal_dialog(message: &str) {
        let text: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
        let caption: Vec<u16> = "Автокликер — ошибка"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: оба буфера null-terminated и живут до конца вызова; HWND=None допустим.
        unsafe {
            MessageBoxW(
                None,
                PCWSTR::from_raw(text.as_ptr()),
                PCWSTR::from_raw(caption.as_ptr()),
                MB_ICONERROR | MB_OK,
            );
        }
    }
}

#[cfg(not(windows))]
mod imp {
    //! Заглушки для юнит-тестов на не-Windows: ничего не нажато, клики «успешны».

    pub fn is_pressed(_vk: u16) -> bool {
        false
    }

    pub fn send_left_click() -> anyhow::Result<()> {
        Ok(())
    }

    pub fn set_high_timer_resolution(_on: bool) {}

    pub fn fatal_dialog(message: &str) {
        eprintln!("{message}");
    }
}

pub use imp::*;
