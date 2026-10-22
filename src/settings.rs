//! Settings and best score, stored as plain `key=value` lines in the user's
//! config directory. Missing or unreadable files fall back to defaults.
use crate::keymap::Keymap;
use crate::math::clamp;
use std::path::PathBuf;

/// Which input flies the pitch axis. Only one is live at a time, so a
/// resting mouse offset can never fight a held key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PitchSource {
    Mouse,
    Keys,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub sensitivity: f64,
    pub pitch: PitchSource,
    /// Flips every pitch input: mouse, W / S and the controller.
    pub invert_pitch: bool,
    pub fov: f64,
    pub muted: bool,
    pub fullscreen: bool,
    pub keys: Keymap,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { sensitivity: 2.8, pitch: PitchSource::Mouse, invert_pitch: false, fov: 58.0, muted: false, fullscreen: false, keys: Keymap::default() }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Best {
    pub score: f64,
    pub time: f64,
}

fn dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map(|b| b.join("viper-run"))
}

fn read() -> Vec<(String, String)> {
    let Some(d) = dir() else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(d.join("settings.txt")) else { return Vec::new() };
    text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect()
}

pub fn load() -> (Settings, Best) {
    let mut s = Settings::default();
    let mut b = Best::default();
    for (k, v) in read() {
        let num = v.parse::<f64>().ok().filter(|n| n.is_finite());
        match (k.as_str(), num) {
            ("sensitivity", Some(n)) => s.sensitivity = clamp(n, 0.5, 5.0),
            ("fov", Some(n)) => s.fov = clamp(n, 45.0, 72.0),
            ("pitch", _) => s.pitch = if v == "keys" { PitchSource::Keys } else { PitchSource::Mouse },
            // Older versions saved "invert_y", when only the mouse flipped.
            ("invert_pitch", _) | ("invert_y", _) => s.invert_pitch = v == "true",
            ("muted", _) => s.muted = v == "true",
            ("fullscreen", _) => s.fullscreen = v == "true",
            ("best_score", Some(n)) => b.score = n.max(0.0),
            ("best_time", Some(n)) => b.time = n.max(0.0),
            (k, _) if k.starts_with("key_") => s.keys.load(&k[4..], &v),
            _ => {}
        }
    }
    (s, b)
}

pub fn save(s: &Settings, b: &Best) {
    let Some(d) = dir() else { return };
    if std::fs::create_dir_all(&d).is_err() {
        return;
    }
    let text = format!(
        "sensitivity={:.1}\npitch={}\ninvert_pitch={}\nfov={}\nmuted={}\nfullscreen={}\nbest_score={}\nbest_time={:.3}\n",
        s.sensitivity, if s.pitch == PitchSource::Keys { "keys" } else { "mouse" }, s.invert_pitch, s.fov.round(), s.muted, s.fullscreen, b.score.round(), b.time
    );
    let text = text + &s.keys.save();
    let _ = std::fs::write(d.join("settings.txt"), text);
}
