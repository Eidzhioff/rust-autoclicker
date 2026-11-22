//! Virtual-Key коды и человекочитаемые имена.
//! Модуль не зависит от WinAPI, поэтому покрыт юнит-тестами на любой ОС.

/// Левая кнопка мыши.
pub const VK_LBUTTON: u16 = 0x01;
/// Боковая кнопка мыши 2 («Вперёд») — кнопка активации по умолчанию (как в прототипе).
pub const VK_XBUTTON2: u16 = 0x06;
/// Esc — отмена режима «Слушать».
pub const VK_ESCAPE: u16 = 0x1B;
/// F12 — экстренный выход (зарезервирован).
pub const EMERGENCY_VK: u16 = 0x7B;

/// Пригоден ли VK-код как кнопка активации (диапазон опроса `GetAsyncKeyState`).
pub fn is_valid_activation(vk: u16) -> bool {
    (0x01..=0xFE).contains(&vk)
}

/// Человекочитаемое имя кнопки; для неизвестных кодов — «VK 0xNN».
pub fn name_for(vk: u16) -> String {
    let name: String = match vk {
        0x01 => "ЛКМ".into(),
        0x02 => "ПКМ".into(),
        0x04 => "СКМ".into(),
        0x05 => "XButton1 (Назад)".into(),
        0x06 => "XButton2 (Вперёд)".into(),
        0x08 => "Backspace".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x10 => "Shift".into(),
        0x11 => "Ctrl".into(),
        0x12 => "Alt".into(),
        0x13 => "Pause".into(),
        0x14 => "CapsLock".into(),
        0x1B => "Esc".into(),
        0x20 => "Пробел".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x25 => "Стрелка влево".into(),
        0x26 => "Стрелка вверх".into(),
        0x27 => "Стрелка вправо".into(),
        0x28 => "Стрелка вниз".into(),
        0x2C => "PrintScreen".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x30..=0x39 => char::from(b'0' + (vk - 0x30) as u8).to_string(),
        0x41..=0x5A => char::from(b'A' + (vk - 0x41) as u8).to_string(),
        0x5B => "Win (левая)".into(),
        0x5C => "Win (правая)".into(),
        0x5D => "Menu (контекстная)".into(),
        0x60..=0x69 => format!("Num {}", vk - 0x60),
        0x6A => "Num *".into(),
        0x6B => "Num +".into(),
        0x6C => "Num (разделитель)".into(),
        0x6D => "Num -".into(),
        0x6E => "Num .".into(),
        0x6F => "Num /".into(),
        0x70..=0x87 => format!("F{}", vk - 0x70 + 1),
        0x90 => "NumLock".into(),
        0x91 => "ScrollLock".into(),
        0xA0 => "Shift (левый)".into(),
        0xA1 => "Shift (правый)".into(),
        0xA2 => "Ctrl (левый)".into(),
        0xA3 => "Ctrl (правый)".into(),
        0xA4 => "Alt (левый)".into(),
        0xA5 => "Alt (правый)".into(),
        0xA6 => "Browser Back".into(),
        0xA7 => "Browser Forward".into(),
        0xAD => "Mute".into(),
        0xAE => "Volume -".into(),
        0xAF => "Volume +".into(),
        0xB0 => "Media Next".into(),
        0xB1 => "Media Prev".into(),
        0xB2 => "Media Stop".into(),
        0xB3 => "Media Play/Pause".into(),
        0xBA => "; :".into(),
        0xBB => "= +".into(),
        0xBC => ", <".into(),
        0xBD => "- _".into(),
        0xBE => ". >".into(),
        0xBF => "/ ?".into(),
        0xC0 => "` ~".into(),
        0xDB => "[ {".into(),
        0xDC => "\\ |".into(),
        0xDD => "] }".into(),
        0xDE => "' \"".into(),
        _ => format!("VK 0x{vk:02X}"),
    };
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_buttons_have_names() {
        assert_eq!(name_for(0x01), "ЛКМ");
        assert_eq!(name_for(0x02), "ПКМ");
        assert_eq!(name_for(0x04), "СКМ");
        assert!(name_for(0x05).contains("XButton1"));
        assert!(name_for(0x06).contains("XButton2"));
    }

    #[test]
    fn function_keys() {
        assert_eq!(name_for(0x70), "F1");
        assert_eq!(name_for(0x7B), "F12");
        assert_eq!(name_for(0x87), "F24");
        assert_eq!(EMERGENCY_VK, 0x7B);
    }

    #[test]
    fn letters_and_digits() {
        assert_eq!(name_for(0x41), "A");
        assert_eq!(name_for(0x5A), "Z");
        assert_eq!(name_for(0x30), "0");
        assert_eq!(name_for(0x39), "9");
        assert_eq!(name_for(0x60), "Num 0");
    }

    #[test]
    fn common_keys() {
        assert_eq!(name_for(0x20), "Пробел");
        assert_eq!(name_for(0x0D), "Enter");
        assert_eq!(name_for(0x1B), "Esc");
    }

    #[test]
    fn unknown_fallback() {
        assert_eq!(name_for(0x00), "VK 0x00");
        assert_eq!(name_for(0xFF), "VK 0xFF");
        assert_eq!(name_for(0x07), "VK 0x07"); // undefined VK
    }

    #[test]
    fn activation_validity() {
        assert!(!is_valid_activation(0x00));
        assert!(is_valid_activation(0x01));
        assert!(is_valid_activation(0xFE));
        assert!(!is_valid_activation(0xFF));
    }
}
