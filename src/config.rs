use std::{env, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::hotkeys::Hotkey;

pub const MIN_SCALE_PERCENT: u32 = 0;
pub const MAX_SCALE_PERCENT: u32 = 500;

fn legacy_scale_version() -> u8 {
    1
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub image_path: Option<PathBuf>,
    pub tint_enabled: bool,
    pub tint: String,
    #[serde(default = "legacy_scale_version")]
    scale_version: u8,
    pub scale: u32,
    pub monitor: String,
    pub center_hotkey: Hotkey,
    pub hide_hotkey: Hotkey,
    pub show_hotkey: Hotkey,
    pub toggle_hotkey: Hotkey,
    pub visible: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            image_path: None,
            tint_enabled: false,
            tint: "#00FF00".into(),
            scale_version: 2,
            scale: 100,
            monitor: String::new(),
            center_hotkey: Hotkey::ctrl_alt(b'C'),
            hide_hotkey: Hotkey::ctrl_alt(b'H'),
            show_hotkey: Hotkey::ctrl_alt(b'S'),
            toggle_hotkey: Hotkey::ctrl_alt(b'X'),
            visible: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = path();
        let Ok(data) = fs::read(&path) else {
            return Self::default();
        };

        match serde_json::from_slice::<Self>(&data) {
            Ok(mut config) => {
                let defaults = Self::default();
                config.normalize_scale();
                if parse_hex(&config.tint).is_none() {
                    config.tint = defaults.tint;
                }
                if !config.center_hotkey.valid() {
                    config.center_hotkey = defaults.center_hotkey;
                }
                if !config.hide_hotkey.valid() {
                    config.hide_hotkey = defaults.hide_hotkey;
                }
                if !config.show_hotkey.valid() {
                    config.show_hotkey = defaults.show_hotkey;
                }
                if !config.toggle_hotkey.valid() {
                    config.toggle_hotkey = defaults.toggle_hotkey;
                }
                config
            }
            Err(_) => {
                let _ = fs::rename(&path, path.with_extension("invalid.json"));
                Self::default()
            }
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(self)?)
    }

    fn normalize_scale(&mut self) {
        if self.scale_version < 2 {
            self.scale = 100 + self.scale.min(100);
            self.scale_version = 2;
        } else {
            self.scale = self.scale.clamp(MIN_SCALE_PERCENT, MAX_SCALE_PERCENT);
        }
    }
}

pub fn path() -> PathBuf {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("LightCrosshair")
        .join("config.json")
}

pub fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    if hex.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ])
}

#[cfg(test)]
mod tests {
    use super::{Config, parse_hex};

    #[test]
    fn parses_only_six_digit_rgb() {
        assert_eq!(parse_hex("#00FF7f"), Some([0, 255, 127]));
        assert_eq!(parse_hex("ffffff"), Some([255, 255, 255]));
        assert_eq!(parse_hex("#123"), None);
        assert_eq!(parse_hex("#GG0000"), None);
    }

    #[test]
    fn migrates_the_old_one_to_two_times_scale_range() {
        let mut config: Config = serde_json::from_str(r#"{"scale": 50}"#).unwrap();
        config.normalize_scale();
        assert_eq!(config.scale, 150);
        assert_eq!(config.scale_version, 2);
    }
}
