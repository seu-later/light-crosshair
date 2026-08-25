use serde::{Deserialize, Serialize};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};

pub const VK_MOUSE_LEFT: u32 = 0x01;
pub const VK_MOUSE_RIGHT: u32 = 0x02;
pub const VK_MOUSE_MIDDLE: u32 = 0x04;
pub const VK_MOUSE_X1: u32 = 0x05;
pub const VK_MOUSE_X2: u32 = 0x06;

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

    #[cfg(test)]
    pub fn mouse(key: u32) -> Self {
        Self { modifiers: 0, key }
    }

    pub fn is_mouse(self) -> bool {
        matches!(
            self.key,
            VK_MOUSE_LEFT | VK_MOUSE_RIGHT | VK_MOUSE_MIDDLE | VK_MOUSE_X1 | VK_MOUSE_X2
        )
    }

    pub fn valid(self) -> bool {
        self.key != 0
            && (self.is_mouse()
                || self.modifiers & (MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_WIN) != 0)
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
    match key {
        VK_MOUSE_LEFT => return "Left Click".into(),
        VK_MOUSE_RIGHT => return "Right Click".into(),
        VK_MOUSE_MIDDLE => return "Middle Click".into(),
        VK_MOUSE_X1 => return "Mouse 4".into(),
        VK_MOUSE_X2 => return "Mouse 5".into(),
        _ => {}
    }
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
    use super::{Hotkey, VK_MOUSE_MIDDLE, VK_MOUSE_X1};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL};

    #[test]
    fn formats_default_shortcut() {
        assert_eq!(Hotkey::ctrl_alt(b'C').label(), "Ctrl + Alt + C");
        assert!(Hotkey::ctrl_alt(b'C').valid());
    }

    #[test]
    fn mouse_binds_need_no_modifier() {
        let bind = Hotkey::mouse(VK_MOUSE_X1);
        assert!(bind.valid());
        assert!(bind.is_mouse());
        assert_eq!(bind.label(), "Mouse 4");
        assert!(!Hotkey::mouse(VK_MOUSE_MIDDLE).label().contains('+'));
    }

    #[test]
    fn keyboard_binds_still_require_a_modifier() {
        assert!(
            !Hotkey {
                modifiers: 0,
                key: b'H' as u32
            }
            .valid()
        );
        assert!(
            Hotkey {
                modifiers: MOD_CONTROL | MOD_ALT,
                key: b'H' as u32
            }
            .valid()
        );
    }
}
