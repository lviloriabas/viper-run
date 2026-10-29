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
pub const DEAD_ZONE_MAX: f64 = 0.35;

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
    /// Independent radial dead zones, measured before any response curve.
    pub pad_dead_left: f64,
    pub pad_dead_right: f64,
    /// In COMBAT, X and a held L3 both select the rudder. X + L3 dispenses.
    pub pad_rudder_click: bool,
    /// Gentler small corrections while holding the zoom button.
    pub pad_fine_aim: bool,
    pub fov: f64,
    pub muted: bool,
    pub fullscreen: bool,
    /// Native frame graphs and hardware utilisation.
    pub performance_overlay: bool,
    /// Mission clocks on; off, no mission times out.
    pub clock: bool,
    pub tutorial_completed: u8,
    pub keys: Keymap,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { sensitivity: 2.8, pitch: PitchSource::Mouse, invert_pitch: false, pad_layout: PadLayout::Combat, expo: 0.65, pad_dead_left: 0.10, pad_dead_right: 0.10, pad_rudder_click: false, pad_fine_aim: true, fov: 58.0, muted: false, fullscreen: false, performance_overlay: true, clock: true, tutorial_completed: 0, keys: Keymap::default() }
    }
}

impl Settings {
    pub fn click_rudder(&self) -> bool {
        self.pad_layout == PadLayout::Combat && self.pad_rudder_click
    }

    /// Shape positions without adding input latency or losing full authority.
    pub fn flight_stick(&self, (x, y): (f64, f64), zoom: f64) -> (f64, f64) {
        let e = if self.pad_fine_aim { self.expo + (0.85 - self.expo).max(0.0) * zoom.clamp(0.0, 1.0) } else { self.expo };
        (stick_expo(x, e), stick_expo(y, e))
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
    let has_saved_ptt=entries.iter().any(|(key,_)|key=="key_push_to_talk");
    let mut b = Best::default();
    for (k, v) in entries {
        let num = v.parse::<f64>().ok().filter(|n| n.is_finite());
        match (k.as_str(), num) {
            ("sensitivity", Some(n)) => s.sensitivity = clamp(n, 0.5, 5.0),
            ("fov", Some(n)) => s.fov = clamp(n, FOV_MIN, FOV_MAX),
            ("expo", Some(n)) => s.expo = clamp(n, 0.0, 1.0),
            ("pad_dead_left", Some(n)) => s.pad_dead_left = clamp(n, 0.0, DEAD_ZONE_MAX),
            ("pad_dead_right", Some(n)) => s.pad_dead_right = clamp(n, 0.0, DEAD_ZONE_MAX),
            ("pad_rudder_click", _) => s.pad_rudder_click = v == "true",
            ("pad_fine_aim", _) => s.pad_fine_aim = v != "false",
            ("pitch", _) => s.pitch = if v == "keys" { PitchSource::Keys } else { PitchSource::Mouse },
            // Older versions saved "invert_y", when only the mouse flipped.
            ("invert_pitch", _) | ("invert_y", _) => s.invert_pitch = v == "true",
            ("pad_layout", _) => s.pad_layout = if v == "classic" { PadLayout::Classic } else { PadLayout::Combat },
            ("muted", _) => s.muted = v == "true",
            ("fullscreen", _) => s.fullscreen = v == "true",
            ("performance_overlay", _) => s.performance_overlay = v != "false",
            ("clock", _) => s.clock = v != "false",
            ("tutorial_completed", _) => s.tutorial_completed = v.parse::<u8>().unwrap_or(0) & 31,
            ("best_score", Some(n)) => b.score = n.max(0.0),
            ("best_time", Some(n)) => b.time = n.max(0.0),
            (k, _) if k.starts_with("key_") => s.keys.load(&k[4..], &v),
            _ => {}
        }
    }
    if !has_saved_ptt {
        for (slot,key) in s.keys.get(crate::keymap::Bind::PushToTalk).into_iter().enumerate() {
            if key.is_some() && crate::keymap::BINDS.iter().any(|&bind|bind!=crate::keymap::Bind::PushToTalk && s.keys.get(bind).contains(&key)) {
                s.keys.set(crate::keymap::Bind::PushToTalk,slot,None);
            }
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
        "sensitivity={:.1}\nexpo={:.2}\npitch={}\ninvert_pitch={}\npad_layout={}\nfov={}\nmuted={}\nfullscreen={}\nperformance_overlay={}\nclock={}\nbest_score={}\nbest_time={:.3}\n",
        s.sensitivity, s.expo, if s.pitch == PitchSource::Keys { "keys" } else { "mouse" }, s.invert_pitch, if s.pad_layout == PadLayout::Classic { "classic" } else { "combat" }, s.fov.round(), s.muted, s.fullscreen, s.performance_overlay, s.clock, b.score.round(), b.time
    );
    text + &format!("tutorial_completed={}\npad_dead_left={:.2}\npad_dead_right={:.2}\npad_rudder_click={}\npad_fine_aim={}\n", s.tutorial_completed, s.pad_dead_left, s.pad_dead_right, s.pad_rudder_click, s.pad_fine_aim) + &s.keys.save()
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

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn a_new_voice_binding_never_steals_an_existing_user_key() {
        let (settings,_) = parse(vec![("key_airbrake".into(),"U,".into())]);
        assert_eq!(settings.keys.lookup(miniquad::KeyCode::U),Some(crate::keymap::Bind::Airbrake));
        assert_eq!(settings.keys.get(crate::keymap::Bind::PushToTalk),[None,None]);
        let (fresh,_) = parse(Vec::new());
        assert_eq!(fresh.keys.lookup(miniquad::KeyCode::U),Some(crate::keymap::Bind::PushToTalk));
        let (saved,_) = parse(vec![("key_push_to_talk".into(),"F10,".into()),("tutorial_completed".into(),"31".into())]);
        assert_eq!(saved.keys.lookup(miniquad::KeyCode::F10),Some(crate::keymap::Bind::PushToTalk));
        assert_eq!(saved.tutorial_completed,31);
    }
}

/// Non-secret multiplayer preferences. Room invitation keys are never persisted.
pub fn load_online() -> (String, String) {
    let text = dir().and_then(|d|std::fs::read_to_string(d.join("online.txt")).ok()).unwrap_or_default();
    parse_online(&text)
}
fn parse_online(text: &str) -> (String, String) {
    let mut name = "Pilot".to_string();
    let mut server = String::new();
    for line in text.lines() {
        if let Some((k,v)) = line.split_once('=') {
            if k == "callsign" && crate::net::valid_name(v) { name=v.into(); }
            if k == "server" && v.len() <= 160 && v.bytes().all(|b|b.is_ascii_graphic()) { server=v.into(); }
        }
    }
    (name,server)
}
pub fn save_online(name: &str, server: &str) {
    let Some(d) = dir() else { return; };
    let (name,server)=parse_online(&format!("callsign={name}\nserver={server}\n"));
    if std::fs::create_dir_all(&d).is_ok() { let _=std::fs::write(d.join("online.txt"),format!("callsign={name}\nserver={server}\n")); }
}
#[cfg(test)]
mod online_tests {
    use super::*;
    #[test]
    fn online_preferences_validate_names_and_do_not_restore_room_credentials() {
        assert_eq!(parse_online("callsign=Wing_2\nserver=localhost:42861\ninvite=secret"),("Wing_2".into(),"localhost:42861".into()));
        assert_eq!(parse_online("callsign=invalid/name\nserver=bad address"),("Pilot".into(),String::new()));
    }
}

#[cfg(test)]
mod controller_tests {
    use super::*;
    #[test]
    fn controller_settings_round_trip_and_invalid_values_are_bounded() {
        let settings = Settings { pad_dead_left: 0.03, pad_dead_right: 0.17, expo: 0.65, pad_rudder_click: true, pad_fine_aim: false, ..Settings::default() };
        let entries = encode(&settings, &Best::default()).lines().filter_map(|l| l.split_once('=')).map(|(k,v)| (k.into(),v.into())).collect();
        assert_eq!(parse(entries).0, settings);
        let saved = parse(vec![("expo".into(), "0.5".into()), ("pad_dead_left".into(), "-1".into()), ("pad_dead_right".into(), "2".into())]).0;
        assert_eq!((saved.expo, saved.pad_dead_left, saved.pad_dead_right), (0.5, 0.0, DEAD_ZONE_MAX));
        for bad in ["NaN", "inf", "-inf", "bad"] {
            assert_eq!(parse(vec![("pad_dead_left".into(), bad.into())]).0.pad_dead_left, 0.10);
        }
        assert!(!parse(Vec::new()).0.pad_rudder_click);
    }

    #[test]
    fn fine_aim_reduces_small_inputs_without_losing_authority_or_reversing() {
        let mut settings = Settings::default();
        for x in [-1.0, -0.4, -0.05, 0.0, 0.05, 0.4, 1.0] {
            let normal = settings.flight_stick((x, x), 0.0).0;
            let zoomed = settings.flight_stick((x, x), 1.0).0;
            assert!(zoomed.abs() <= normal.abs());
            assert_eq!(zoomed.signum(), normal.signum());
        }
        assert_eq!(settings.flight_stick((1.0, -1.0), 1.0), (1.0, -1.0));
        settings.pad_fine_aim = false;
        assert_eq!(settings.flight_stick((0.2, 0.3), 1.0), settings.flight_stick((0.2, 0.3), 0.0));
        settings.expo = 1.0;
        settings.pad_fine_aim = true;
        assert_eq!(settings.flight_stick((0.2, 0.3), 1.0), settings.flight_stick((0.2, 0.3), 0.0));
    }
}
