use serde::{Deserialize, Serialize};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hotkey {
    pub modifiers: u32,
    pub key: u32,
}

impl Hotkey {
    pub const fn ctrl_alt(key: u8) -> Self {
        Self {
            modifiers: MOD_CONTROL | MOD_ALT,
            key: key as u32,
        }
    }

    pub fn valid(self) -> bool {
        self.key != 0 && self.modifiers & (MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_WIN) != 0
    }

    pub fn label(self) -> String {
        let mut parts = Vec::with_capacity(5);
        if self.modifiers & MOD_CONTROL != 0 {
            parts.push("Ctrl".to_string());
        }
        if self.modifiers & MOD_ALT != 0 {
            parts.push("Alt".to_string());
        }
        if self.modifiers & MOD_SHIFT != 0 {
            parts.push("Shift".to_string());
        }
        if self.modifiers & MOD_WIN != 0 {
            parts.push("Win".to_string());
        }
        parts.push(key_name(self.key));
        parts.join(" + ")
    }
}

impl Default for Hotkey {
    fn default() -> Self {
        Self::ctrl_alt(b'C')
    }
}

fn key_name(key: u32) -> String {
    if (b'A' as u32..=b'Z' as u32).contains(&key) || (b'0' as u32..=b'9' as u32).contains(&key) {
        return char::from_u32(key).unwrap().to_string();
    }
    if (0x70..=0x87).contains(&key) {
        return format!("F{}", key - 0x6f);
    }
    match key {
        0x20 => "Space".into(),
        0x21 => "Page Up".into(),
        0x22 => "Page Down".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x25 => "Left".into(),
        0x26 => "Up".into(),
        0x27 => "Right".into(),
        0x28 => "Down".into(),
        0x2d => "Insert".into(),
        0x2e => "Delete".into(),
        _ => format!("Key 0x{key:02X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::Hotkey;

    #[test]
    fn formats_default_shortcut() {
        assert_eq!(Hotkey::ctrl_alt(b'C').label(), "Ctrl + Alt + C");
        assert!(Hotkey::ctrl_alt(b'C').valid());
    }
}
