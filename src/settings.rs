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

/// Controller button layout. COMBAT keeps fire, lock and throttle on the
/// triggers and bumpers, so both thumbs stay on the sticks while the helmet
/// looks; CLASSIC is the older layout with the rudder on the bumpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadLayout {
    Combat,
    Classic,
}

/// Cockpit vertical field of view range, degrees.
pub const FOV_MIN: f64 = 40.0;
pub const FOV_MAX: f64 = 95.0;

/// RC-style stick expo, y = (1 - e) x + e x^3: a gentle slope round the
/// centre for small corrections that steepens to full deflection at the rim,
/// so a quick flick still reaches the stops.
pub fn stick_expo(x: f64, e: f64) -> f64 {
    (1.0 - e) * x + e * x * x * x
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub sensitivity: f64,
    pub pitch: PitchSource,
    /// Flips every pitch input: mouse, W / S and the controller.
    pub invert_pitch: bool,
    pub pad_layout: PadLayout,
    /// Controller stick expo, 0 (linear) to 1 (cubic): see `stick_expo`.
    pub expo: f64,
    pub fov: f64,
    pub muted: bool,
    pub fullscreen: bool,
    /// Native frame graphs and hardware utilisation.
    pub performance_overlay: bool,
    /// Mission clocks on; off, no mission times out.
    pub clock: bool,
    pub keys: Keymap,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { sensitivity: 2.8, pitch: PitchSource::Mouse, invert_pitch: false, pad_layout: PadLayout::Combat, expo: 0.5, fov: 58.0, muted: false, fullscreen: false, performance_overlay: true, clock: true, keys: Keymap::default() }
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
    parse(read())
}

fn parse(entries: Vec<(String, String)>) -> (Settings, Best) {
    let mut s = Settings::default();
    let mut b = Best::default();
    for (k, v) in entries {
        let num = v.parse::<f64>().ok().filter(|n| n.is_finite());
        match (k.as_str(), num) {
            ("sensitivity", Some(n)) => s.sensitivity = clamp(n, 0.5, 5.0),
            ("fov", Some(n)) => s.fov = clamp(n, FOV_MIN, FOV_MAX),
            ("expo", Some(n)) => s.expo = clamp(n, 0.0, 1.0),
            ("pitch", _) => s.pitch = if v == "keys" { PitchSource::Keys } else { PitchSource::Mouse },
            // Older versions saved "invert_y", when only the mouse flipped.
            ("invert_pitch", _) | ("invert_y", _) => s.invert_pitch = v == "true",
            ("pad_layout", _) => s.pad_layout = if v == "classic" { PadLayout::Classic } else { PadLayout::Combat },
            ("muted", _) => s.muted = v == "true",
            ("fullscreen", _) => s.fullscreen = v == "true",
            ("performance_overlay", _) => s.performance_overlay = v != "false",
            ("clock", _) => s.clock = v != "false",
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
    let _ = std::fs::write(d.join("settings.txt"), encode(s, b));
}

fn encode(s: &Settings, b: &Best) -> String {
    let text = format!(
        "sensitivity={:.1}\nexpo={:.1}\npitch={}\ninvert_pitch={}\npad_layout={}\nfov={}\nmuted={}\nfullscreen={}\nperformance_overlay={}\nclock={}\nbest_score={}\nbest_time={:.3}\n",
        s.sensitivity, s.expo, if s.pitch == PitchSource::Keys { "keys" } else { "mouse" }, s.invert_pitch, if s.pad_layout == PadLayout::Classic { "classic" } else { "combat" }, s.fov.round(), s.muted, s.fullscreen, s.performance_overlay, s.clock, b.score.round(), b.time
    );
    text + &s.keys.save()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn performance_switch_survives_save_and_older_settings() {
        let old = parse(vec![("fov".into(), "72".into())]).0;
        assert!(old.performance_overlay);
        for enabled in [false, true] {
            let s = Settings { performance_overlay: enabled, ..Settings::default() };
            let entries = encode(&s, &Best::default()).lines()
                .filter_map(|l| l.split_once('='))
                .map(|(k, v)| (k.to_owned(), v.to_owned())).collect();
            assert_eq!(parse(entries).0.performance_overlay, enabled);
        }
    }
}