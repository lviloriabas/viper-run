//! Remappable flight keys. Each action holds a primary and an alternate key;
//! the controls drawer rebinds them and settings.rs saves them by name. Menu
//! keys (Esc, Enter, arrows in menus), P, K, M, F11 and Alt stay fixed.
use miniquad::KeyCode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bind {
    PitchUp,
    PitchDown,
    RollLeft,
    RollRight,
    YawLeft,
    YawRight,
    ThrottleUp,
    ThrottleDown,
    Afterburner,
    Trigger,
    Weapon,
    Lock,
    Countermeasures,
    Airbrake,
    Gear,
    Flaps,
    WheelBrake,
    View,
    Center,
    LeftMfd,
    RightMfd,
}

pub const BINDS: [Bind; 21] = [
    Bind::PitchUp,
    Bind::PitchDown,
    Bind::RollLeft,
    Bind::RollRight,
    Bind::YawLeft,
    Bind::YawRight,
    Bind::ThrottleUp,
    Bind::ThrottleDown,
    Bind::Afterburner,
    Bind::Trigger,
    Bind::Weapon,
    Bind::Lock,
    Bind::Countermeasures,
    Bind::Airbrake,
    Bind::Gear,
    Bind::Flaps,
    Bind::WheelBrake,
    Bind::View,
    Bind::Center,
    Bind::LeftMfd,
    Bind::RightMfd,
];

impl Bind {
    pub fn index(self) -> usize {
        BINDS.iter().position(|&b| b == self).unwrap_or(0)
    }

    /// Label in the controls drawer.
    pub fn label(self) -> &'static str {
        match self {
            Bind::PitchUp => "PITCH UP (PULL)",
            Bind::PitchDown => "PITCH DOWN (PUSH)",
            Bind::RollLeft => "ROLL LEFT",
            Bind::RollRight => "ROLL RIGHT",
            Bind::YawLeft => "RUDDER LEFT",
            Bind::YawRight => "RUDDER RIGHT",
            Bind::ThrottleUp => "THROTTLE UP",
            Bind::ThrottleDown => "THROTTLE DOWN",
            Bind::Afterburner => "AFTERBURNER",
            Bind::Trigger => "FIRE / PICKLE",
            Bind::Weapon => "CYCLE WEAPON",
            Bind::Lock => "LOCK TARGET",
            Bind::Countermeasures => "CHAFF / FLARE",
            Bind::Airbrake => "AIRBRAKE",
            Bind::Gear => "LANDING GEAR",
            Bind::Flaps => "FLAPS",
            Bind::WheelBrake => "WHEEL BRAKES",
            Bind::View => "COCKPIT / CHASE",
            Bind::Center => "CENTER VIEW",
            Bind::LeftMfd => "LEFT MFD PAGE",
            Bind::RightMfd => "RIGHT MFD PAGE",
        }
    }

    /// Name in the settings file.
    pub fn id(self) -> &'static str {
        match self {
            Bind::PitchUp => "pitch_up",
            Bind::PitchDown => "pitch_down",
            Bind::RollLeft => "roll_left",
            Bind::RollRight => "roll_right",
            Bind::YawLeft => "yaw_left",
            Bind::YawRight => "yaw_right",
            Bind::ThrottleUp => "throttle_up",
            Bind::ThrottleDown => "throttle_down",
            Bind::Afterburner => "afterburner",
            Bind::Trigger => "trigger",
            Bind::Weapon => "weapon",
            Bind::Lock => "lock",
            Bind::Countermeasures => "countermeasures",
            Bind::Airbrake => "airbrake",
            Bind::Gear => "gear",
            Bind::Flaps => "flaps",
            Bind::WheelBrake => "wheel_brake",
            Bind::View => "view",
            Bind::Center => "center",
            Bind::LeftMfd => "left_mfd",
            Bind::RightMfd => "right_mfd",
        }
    }

    fn defaults(self) -> [Option<KeyCode>; 2] {
        use KeyCode::*;
        let (a, b) = match self {
            Bind::PitchUp => (S, Some(Down)),
            Bind::PitchDown => (W, Some(Up)),
            Bind::RollLeft => (A, Some(Left)),
            Bind::RollRight => (D, Some(Right)),
            Bind::YawLeft => (Q, None),
            Bind::YawRight => (E, None),
            Bind::ThrottleUp => (LeftShift, Some(RightShift)),
            Bind::ThrottleDown => (LeftControl, Some(RightControl)),
            Bind::Afterburner => (Space, None),
            Bind::Trigger => (F, None),
            Bind::Weapon => (X, None),
            Bind::Lock => (T, None),
            Bind::Countermeasures => (Z, None),
            Bind::Airbrake => (H, None),
            Bind::Gear => (G, None),
            Bind::Flaps => (L, None),
            Bind::WheelBrake => (B, None),
            Bind::View => (V, None),
            Bind::Center => (C, None),
            Bind::LeftMfd => (F1, None),
            Bind::RightMfd => (F2, None),
        };
        [Some(a), b]
    }
}

/// Keys that keep their job whatever the bindings say.
pub const RESERVED: [KeyCode; 9] = [KeyCode::Escape, KeyCode::Enter, KeyCode::KpEnter, KeyCode::P, KeyCode::K, KeyCode::M, KeyCode::F11, KeyCode::LeftAlt, KeyCode::RightAlt];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keymap {
    pub keys: [[Option<KeyCode>; 2]; BINDS.len()],
}

impl Default for Keymap {
    fn default() -> Keymap {
        let mut keys = [[None; 2]; BINDS.len()];
        for (i, b) in BINDS.iter().enumerate() {
            keys[i] = b.defaults();
        }
        Keymap { keys }
    }
}

impl Keymap {
    pub fn get(&self, b: Bind) -> [Option<KeyCode>; 2] {
        self.keys[b.index()]
    }

    /// The action a key is bound to.
    pub fn lookup(&self, key: KeyCode) -> Option<Bind> {
        BINDS.iter().copied().find(|&b| self.get(b).contains(&Some(key)))
    }

    /// Either key of the action is down.
    pub fn held(&self, b: Bind, down: &std::collections::HashSet<KeyCode>) -> bool {
        self.get(b).iter().flatten().any(|k| down.contains(k))
    }

    /// Puts `key` in a slot, taking it off any other action first so one
    /// key never flies two things. `None` clears the slot.
    pub fn set(&mut self, b: Bind, slot: usize, key: Option<KeyCode>) {
        if let Some(k) = key {
            for pair in self.keys.iter_mut() {
                for s in pair.iter_mut() {
                    if *s == Some(k) {
                        *s = None;
                    }
                }
            }
        }
        self.keys[b.index()][slot.min(1)] = key;
    }

    /// Settings lines, one per action.
    pub fn save(&self) -> String {
        let mut s = String::new();
        for &b in BINDS.iter() {
            let keys: Vec<String> = self.get(b).iter().map(|k| k.map(key_id).unwrap_or_default()).collect();
            s.push_str(&format!("key_{}={}\n", b.id(), keys.join(",")));
        }
        s
    }

    /// Reads one settings entry; unknown names are ignored.
    pub fn load(&mut self, id: &str, value: &str) {
        // Older versions called the airbrake the speedbrake.
        let id = if id == "speedbrake" { "airbrake" } else { id };
        let Some(b) = BINDS.iter().copied().find(|b| b.id() == id) else { return };
        let mut it = value.split(',').map(|s| parse_key(s.trim()));
        self.keys[b.index()] = [it.next().flatten(), it.next().flatten()];
    }
}

/// Every key the bindings accept, with its settings name.
const KEYS: &[KeyCode] = {
    use KeyCode::*;
    &[
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Space, Tab, Backspace, Insert, Delete, Home, End, PageUp, PageDown, Up, Down, Left, Right, LeftShift, RightShift, LeftControl, RightControl, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F12, Kp0, Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7, Kp8, Kp9, KpDecimal, KpDivide, KpMultiply, KpSubtract, KpAdd, Minus, Equal, LeftBracket, RightBracket, Semicolon, Apostrophe, Comma, Period, Slash, Backslash, GraveAccent, CapsLock,
    ]
};

pub fn bindable(k: KeyCode) -> bool {
    KEYS.contains(&k) && !RESERVED.contains(&k)
}

fn key_id(k: KeyCode) -> String {
    format!("{k:?}")
}

fn parse_key(s: &str) -> Option<KeyCode> {
    KEYS.iter().copied().find(|&k| key_id(k) == s)
}

/// Short upper-case name for the screen.
pub fn key_name(k: KeyCode) -> String {
    use KeyCode::*;
    match k {
        LeftShift => "L SHIFT".into(),
        RightShift => "R SHIFT".into(),
        LeftControl => "L CTRL".into(),
        RightControl => "R CTRL".into(),
        PageUp => "PG UP".into(),
        PageDown => "PG DN".into(),
        Backspace => "BKSP".into(),
        GraveAccent => "`".into(),
        Minus => "-".into(),
        Equal => "=".into(),
        LeftBracket => "[".into(),
        RightBracket => "]".into(),
        Semicolon => ";".into(),
        Apostrophe => "'".into(),
        Comma => ",".into(),
        Period => ".".into(),
        Slash => "/".into(),
        Backslash => "\\".into(),
        _ => {
            let s = key_id(k).to_uppercase();
            if let Some(d) = s.strip_prefix("KEY") {
                d.to_string()
            } else if let Some(d) = s.strip_prefix("KP") {
                format!("NUM {d}")
            } else {
                s
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_round_trip_through_the_settings_text() {
        let mut m = Keymap::default();
        m.set(Bind::Gear, 0, Some(KeyCode::Kp5));
        m.set(Bind::Trigger, 1, Some(KeyCode::Space));
        let mut back = Keymap::default();
        for line in m.save().lines() {
            let (k, v) = line.split_once('=').unwrap();
            back.load(k.strip_prefix("key_").unwrap(), v);
        }
        assert_eq!(back, m);
        // Space moved to the trigger, so the afterburner lost it.
        assert_eq!(back.get(Bind::Afterburner), [None, None]);
        assert_eq!(back.lookup(KeyCode::Space), Some(Bind::Trigger));
    }

    #[test]
    fn defaults_bind_no_key_twice_and_no_reserved_key() {
        let m = Keymap::default();
        let all: Vec<KeyCode> = m.keys.iter().flatten().flatten().copied().collect();
        for (i, k) in all.iter().enumerate() {
            assert!(bindable(*k), "{k:?} reserved");
            assert!(!all[i + 1..].contains(k), "{k:?} bound twice");
        }
    }
}
