//! Xbox controller input: XInput (loaded at run time) on Windows and the
//! kernel joystick interface (/dev/input/js*) on Linux. With no controller
//! every reading is zero, so callers never need to check.

// Button bits, numbered as XInput reports them.
pub const UP: u16 = 0x0001;
pub const DOWN: u16 = 0x0002;
pub const LEFT: u16 = 0x0004;
pub const RIGHT: u16 = 0x0008;
pub const START: u16 = 0x0010;
pub const BACK: u16 = 0x0020;
pub const LS: u16 = 0x0040;
pub const RS: u16 = 0x0080;
pub const LB: u16 = 0x0100;
pub const RB: u16 = 0x0200;
pub const A: u16 = 0x1000;
pub const B: u16 = 0x2000;
pub const X: u16 = 0x4000;
pub const Y: u16 = 0x8000;

/// One controller reading. Sticks run -1..1 with right and up positive and
/// the dead zone removed; triggers run 0..1.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct State {
    pub connected: bool,
    pub buttons: u16,
    pub left: (f64, f64),
    pub right: (f64, f64),
    pub lt: f64,
    pub rt: f64,
}

pub struct Gamepad {
    device: backend::Device,
    /// Unfiltered readings for drift measurement and the live setup monitor.
    pub raw: State,
    pub state: State,
    prev: State,
    /// When each button went down, by bit, and the previous and current poll times.
    down_at: [f64; 16],
    then: f64,
    now: f64,
    nav_held: u16,
    nav_next: f64,
    nav_event: u16,
    calibration: Option<Calibration>,
    pub calibration_status: &'static str,
}

/// Radial dead zone, rescaled so the output rises linearly from zero at its
/// edge and reaches full deflection a little short of the rim, where worn
/// sticks stop. The FLCS adds the only response curve.
pub fn stick((x, y): (f64, f64), dead: f64) -> (f64, f64) {
    const FULL: f64 = 0.95;
    let dead = dead.clamp(0.0, crate::settings::DEAD_ZONE_MAX);
    let m = (x * x + y * y).sqrt();
    if m <= dead {
        return (0.0, 0.0);
    }
    let output = ((m - dead) / (FULL - dead)).min(1.0);
    (x / m * output, y / m * output)
}

/// A brief hands-off measurement. A settling interval lets the player release
/// the menu stick; a range check rejects deliberate motion instead of hiding it.
struct Calibration {
    started: f64,
    last: f64,
    samples: usize,
    peak: [f64; 2],
    low: [f64; 4],
    high: [f64; 4],
}

impl Calibration {
    const SETTLE: f64 = 1.5;
    const MEASURE: f64 = 3.0;

    fn new(now: f64) -> Self {
        Self { started: now, last: now, samples: 0, peak: [0.0; 2], low: [f64::INFINITY; 4], high: [f64::NEG_INFINITY; 4] }
    }

    fn sample(&mut self, raw: State, now: f64) -> Option<Result<(f64, f64), &'static str>> {
        if !raw.connected { return Some(Err("CONTROLLER LOST. RECONNECT AND RETRY.")); }
        if now - self.last > 0.25 || now < self.last { return Some(Err("MEASUREMENT INTERRUPTED. PLEASE RETRY.")); }
        self.last = now;
        if now - self.started < Self::SETTLE { return None; }
        let axes = [raw.left.0, raw.left.1, raw.right.0, raw.right.1];
        for (i, v) in axes.into_iter().enumerate() {
            self.low[i] = self.low[i].min(v);
            self.high[i] = self.high[i].max(v);
        }
        for (i, (x, y)) in [raw.left, raw.right].into_iter().enumerate() {
            self.peak[i] = self.peak[i].max(x.hypot(y));
        }
        if self.peak.iter().any(|&p| p > 0.32) || (0..4).any(|i| self.high[i] - self.low[i] > 0.08) {
            return Some(Err("STICKS MOVED OR DRIFT TOO HIGH. RETRY AT REST."));
        }
        self.samples += 1;
        if now - self.started < Self::SETTLE + Self::MEASURE { return None; }
        if self.samples < 60 { return Some(Err("TOO FEW SAMPLES. PLEASE RETRY.")); }
        // Round up to whole percent, with 1.5% headroom above all observed drift.
        let dead = |p: f64| ((p + 0.015) * 100.0).ceil() / 100.0;
        Some(Ok((dead(self.peak[0]), dead(self.peak[1]))))
    }
}

fn trigger(t: f64) -> f64 {
    ((t - 0.12) / 0.88).clamp(0.0, 1.0)
}

/// Menu directions from the d-pad, or the left stick's dominant axis.
fn dirs(s: &State) -> u16 {
    let (x, y) = s.left;
    let mut d = s.buttons & (UP | DOWN | LEFT | RIGHT);
    if y.abs() >= x.abs() {
        d |= if y > 0.6 { UP } else if y < -0.6 { DOWN } else { 0 };
    } else {
        d |= if x > 0.6 { RIGHT } else if x < -0.6 { LEFT } else { 0 };
    }
    d
}

impl Gamepad {
    pub fn new() -> Gamepad {
        Gamepad { device: backend::Device::new(), raw: State::default(), state: State::default(), prev: State::default(), down_at: [0.0; 16], then: 0.0, now: 0.0, nav_held: 0, nav_next: 0.0, nav_event: 0, calibration: None, calibration_status: "RELEASE BOTH STICKS BEFORE CALIBRATING." }
    }

    /// Reads the controller; call once per frame. Returns the new connection
    /// state when a controller was plugged in or removed.
    pub fn poll(&mut self, now: f64, dead_left: f64, dead_right: f64) -> Option<bool> {
        self.prev = self.state;
        self.raw = self.device.read(now).unwrap_or_default();
        let r = self.raw;
        self.state = if r.connected { State { left: stick(r.left, dead_left), right: stick(r.right, dead_right), lt: trigger(r.lt), rt: trigger(r.rt), ..r } } else { State::default() };
        (self.then, self.now) = (self.now, now);
        for i in 0..16 {
            if self.state.buttons & !self.prev.buttons & (1 << i) != 0 {
                self.down_at[i] = now;
            }
        }
        self.update_navigation(now);
        (self.state.connected != self.prev.connected).then_some(self.state.connected)
    }

    pub fn calibrating(&self) -> bool { self.calibration.is_some() }

    pub fn start_calibration(&mut self, now: f64) {
        if self.raw.connected {
            self.calibration = Some(Calibration::new(now));
        } else {
            self.calibration_status = "CONNECT A CONTROLLER FIRST.";
        }
    }

    pub fn cancel_calibration(&mut self) {
        if self.calibration.take().is_some() { self.calibration_status = "CALIBRATION CANCELLED. SETTINGS KEPT."; }
    }

    pub fn calibration_message(&self, now: f64) -> String {
        if let Some(c) = &self.calibration {
            let elapsed = now - c.started;
            if elapsed < Calibration::SETTLE { "LET GO OF BOTH STICKS. KEEP THEM STILL.".into() }
            else { format!("MEASURING DRIFT: {:.0} SECONDS LEFT. HANDS OFF.", (Calibration::SETTLE + Calibration::MEASURE - elapsed).max(0.0).ceil()) }
        } else { self.calibration_status.into() }
    }

    pub fn calibration_tick(&mut self, now: f64) -> Option<(f64, f64)> {
        let result = self.calibration.as_mut()?.sample(self.raw, now)?;
        self.calibration = None;
        match result {
            Ok(dead) => { self.calibration_status = "CALIBRATED AND SAVED. TEST SMALL MOVEMENTS BELOW."; Some(dead) }
            Err(message) => { self.calibration_status = message; None }
        }
    }

    pub fn rudder(&self, settings: &crate::settings::Settings) -> bool {
        self.held(X) || settings.click_rudder() && self.held(LS) && !self.held(RS)
    }

    pub fn countermeasures_held(&self, settings: &crate::settings::Settings) -> bool {
        self.held(LS) && !self.held(RS) && (!settings.click_rudder() || self.held(X))
    }

    pub fn countermeasures_pressed(&self, settings: &crate::settings::Settings) -> bool {
        self.countermeasures_held(settings) && (self.pressed(LS) || settings.click_rudder() && self.pressed(X))
    }

    pub fn held(&self, b: u16) -> bool {
        self.state.buttons & b != 0
    }

    /// True on the frame the button goes down.
    pub fn pressed(&self, b: u16) -> bool {
        self.held(b) && self.prev.buttons & b == 0
    }

    /// True on the frame the button comes up.
    pub fn released(&self, b: u16) -> bool {
        !self.held(b) && self.prev.buttons & b != 0
    }

    /// True on the frame a held button has been down for `secs`.
    pub fn long_press(&self, b: u16, secs: f64) -> bool {
        let t = self.down_at[b.trailing_zeros() as usize] + secs;
        self.held(b) && self.then < t && self.now >= t
    }

    /// The controller is being used this frame: a button went down, or a
    /// stick or trigger is well off its rest.
    pub fn active(&self) -> bool {
        let s = &self.state;
        let m = |(x, y): (f64, f64)| x.abs().max(y.abs());
        s.connected && (s.buttons & !self.prev.buttons != 0
            || m((s.left.0 - self.prev.left.0, s.left.1 - self.prev.left.1)) > 0.18
            || m((s.right.0 - self.prev.right.0, s.right.1 - self.prev.right.1)) > 0.18
            || s.lt > 0.3 && self.prev.lt <= 0.3 || s.rt > 0.3 && self.prev.rt <= 0.3)
    }

    /// Menu directions newly pushed this frame.
    pub fn nav(&self) -> u16 { self.nav_event }

    fn update_navigation(&mut self, now: f64) {
        let mut held = if self.state.connected { dirs(&self.state) } else { 0 };
        let (x, y) = self.state.left;
        if held == 0 && self.nav_held != 0 {
            let keep = match self.nav_held { UP => y > 0.35, DOWN => y < -0.35,
                LEFT => x < -0.35, RIGHT => x > 0.35, _ => false };
            if keep { held = self.nav_held; }
        }
        self.nav_event = 0;
        if held != self.nav_held {
            self.nav_event = held;
            self.nav_next = now + 0.34;
        } else if held != 0 && now >= self.nav_next {
            self.nav_event = held;
            self.nav_next = now + 0.12;
        }
        self.nav_held = held;
    }
}

#[cfg(windows)]
mod backend {
    use super::State;
    use std::ffi::{c_char, c_void};

    #[repr(C)]
    #[derive(Default)]
    struct XInputState {
        packet: u32,
        buttons: u16,
        lt: u8,
        rt: u8,
        lx: i16,
        ly: i16,
        rx: i16,
        ry: i16,
    }
    type GetState = unsafe extern "system" fn(u32, *mut XInputState) -> u32;
    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    }

    pub struct Device {
        get: Option<GetState>,
        slot: Option<u32>,
        probe: u32,
        next_probe: f64,
    }

    impl Device {
        pub fn new() -> Device {
            let get = [&b"xinput1_4.dll\0"[..], b"xinput1_3.dll\0", b"xinput9_1_0.dll\0"].iter().find_map(|name| unsafe {
                let lib = LoadLibraryA(name.as_ptr() as *const c_char);
                let f = if lib.is_null() { lib } else { GetProcAddress(lib, b"XInputGetState\0".as_ptr() as *const c_char) };
                (!f.is_null()).then(|| std::mem::transmute::<*mut c_void, GetState>(f))
            });
            Device { get, slot: None, probe: 0, next_probe: 0.0 }
        }

        fn query(&self, slot: u32) -> Option<XInputState> {
            let get = self.get?;
            let mut s = XInputState::default();
            (unsafe { get(slot, &mut s) } == 0).then_some(s)
        }

        /// Reads the connected slot every frame. With none connected, one
        /// slot is probed every half second, because XInput is slow to
        /// report a missing controller.
        pub fn read(&mut self, now: f64) -> Option<State> {
            if self.slot.is_none() && now >= self.next_probe {
                self.next_probe = now + 0.5;
                if self.query(self.probe).is_some() {
                    self.slot = Some(self.probe);
                }
                self.probe = (self.probe + 1) % 4;
            }
            let s = self.query(self.slot?);
            let Some(s) = s else {
                self.slot = None;
                return None;
            };
            let ax = |v: i16| (v as f64 / 32767.0).max(-1.0);
            Some(State { connected: true, buttons: s.buttons, left: (ax(s.lx), ax(s.ly)), right: (ax(s.rx), ax(s.ry)), lt: s.lt as f64 / 255.0, rt: s.rt as f64 / 255.0 })
        }
    }
}

#[cfg(target_os = "linux")]
mod backend {
    use super::*;
    use std::ffi::CString;

    #[repr(C)]
    struct JsEvent {
        time: u32,
        value: i16,
        kind: u8,
        number: u8,
    }
    const JS_EVENT_BUTTON: u8 = 0x01;
    const JS_EVENT_AXIS: u8 = 0x02;
    const JS_EVENT_INIT: u8 = 0x80;
    const JSIOCGAXES: u64 = 0x8001_6a11;
    const JSIOCGBUTTONS: u64 = 0x8001_6a12;
    /// xpad button order: A B X Y LB RB BACK START GUIDE LS RS, then the
    /// d-pad on wireless receivers that report it as buttons.
    const BUTTONS: [u16; 15] = [A, B, X, Y, LB, RB, BACK, START, 0, LS, RS, LEFT, RIGHT, UP, DOWN];

    pub struct Device {
        fd: i32,
        axes: [i16; 8],
        buttons: u16,
        next_probe: f64,
    }

    impl Device {
        pub fn new() -> Device {
            Device { fd: -1, axes: [0; 8], buttons: 0, next_probe: 0.0 }
        }

        fn open(&mut self) {
            for i in 0..8 {
                let path = CString::new(format!("/dev/input/js{i}")).unwrap();
                unsafe {
                    let fd = libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC);
                    if fd < 0 {
                        continue;
                    }
                    // Skip accelerometers and other devices that are not gamepads.
                    let (mut axes, mut buttons) = (0u8, 0u8);
                    if libc::ioctl(fd, JSIOCGAXES as _, &mut axes) == 0 && libc::ioctl(fd, JSIOCGBUTTONS as _, &mut buttons) == 0 && axes >= 6 && buttons >= 10 {
                        *self = Device { fd, axes: [0; 8], buttons: 0, next_probe: self.next_probe };
                        return;
                    }
                    libc::close(fd);
                }
            }
        }

        /// Drains pending events. With no device open the js nodes are
        /// rescanned every two seconds.
        pub fn read(&mut self, now: f64) -> Option<State> {
            if self.fd < 0 {
                if now < self.next_probe {
                    return None;
                }
                self.next_probe = now + 2.0;
                self.open();
                if self.fd < 0 {
                    return None;
                }
            }
            let mut ev = JsEvent { time: 0, value: 0, kind: 0, number: 0 };
            let size = std::mem::size_of::<JsEvent>();
            loop {
                let n = unsafe { libc::read(self.fd, &mut ev as *mut JsEvent as *mut libc::c_void, size) };
                if n != size as isize {
                    if n < 0 && matches!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EAGAIN) | Some(libc::EINTR)) {
                        break;
                    }
                    // Unplugged.
                    unsafe { libc::close(self.fd) };
                    self.fd = -1;
                    return None;
                }
                let i = ev.number as usize;
                match ev.kind & !JS_EVENT_INIT {
                    JS_EVENT_AXIS if i < self.axes.len() => self.axes[i] = ev.value,
                    JS_EVENT_BUTTON if i < BUTTONS.len() => {
                        if ev.value != 0 {
                            self.buttons |= BUTTONS[i];
                        } else {
                            self.buttons &= !BUTTONS[i];
                        }
                    }
                    _ => {}
                }
            }
            // xpad axes: LX LY LT RX RY RT HATX HATY, Y down positive and the
            // triggers spanning the full range from released to pulled.
            let ax = |i: usize| (self.axes[i] as f64 / 32767.0).clamp(-1.0, 1.0);
            let hat = |i: usize, neg: u16, pos: u16| if self.axes[i] < -16384 { neg } else if self.axes[i] > 16384 { pos } else { 0 };
            Some(State {
                connected: true,
                buttons: self.buttons | hat(6, LEFT, RIGHT) | hat(7, UP, DOWN),
                left: (ax(0), -ax(1)),
                right: (ax(3), -ax(4)),
                lt: (ax(2) + 1.0) / 2.0,
                rt: (ax(5) + 1.0) / 2.0,
            })
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod backend {
    use super::State;

    pub struct Device;

    impl Device {
        pub fn new() -> Device {
            Device
        }

        pub fn read(&mut self, _now: f64) -> Option<State> {
            None
        }
    }
}

#[cfg(test)]
mod menu_tests {
    use super::*;
    #[test]
    fn joystick_repeats_with_hysteresis_and_stops_at_rest() {
        let mut p = Gamepad::new(); p.state.connected = true;
        p.state.left = (0.0, 0.75); p.update_navigation(0.0); assert_eq!(p.nav(), UP);
        p.state.left = (0.0, 0.50); p.update_navigation(0.1); assert_eq!(p.nav(), 0);
        p.update_navigation(0.35); assert_eq!(p.nav(), UP);
        p.update_navigation(0.40); assert_eq!(p.nav(), 0);
        p.update_navigation(0.48); assert_eq!(p.nav(), UP);
        p.state.left = (0.9, 0.1); p.update_navigation(0.49); assert_eq!(p.nav(), RIGHT);
        p.state.left = (0.0, 0.0); p.update_navigation(0.5); assert_eq!(p.nav(), 0);
        p.update_navigation(2.0); assert_eq!(p.nav(), 0);
    }
    #[test]
    fn held_stick_does_not_steal_keyboard_prompts() {
        let mut p = Gamepad::new();
        p.state = State { connected: true, left: (0.0, 0.8), ..Default::default() };
        assert!(p.active()); p.prev = p.state; assert!(!p.active());
        p.state.buttons = A; assert!(p.active());
    }
}

#[cfg(test)]
mod tuning_tests {
    use super::*;
    use crate::settings::{PadLayout, Settings};

    #[test]
    fn radial_dead_zone_is_continuous_and_preserves_direction_and_full_travel() {
        for dead in [0.0, 0.02, 0.1, 0.35] {
            assert_eq!(stick((0.0, 0.0), dead), (0.0, 0.0));
            assert_eq!(stick((dead, 0.0), dead), (0.0, 0.0));
            assert!(stick((dead + 1e-8, 0.0), dead).0 < 1e-6);
            assert_eq!(stick((0.95, 0.0), dead), (1.0, 0.0));
            assert_eq!(stick((-1.0, 0.0), dead), (-1.0, 0.0));
            let diagonal = stick((0.5, -0.5), dead);
            assert!((diagonal.0 + diagonal.1).abs() < 1e-10);
            let rim = stick((1.0, 1.0), dead);
            assert!((rim.0.hypot(rim.1) - 1.0).abs() < 1e-10);
        }
        assert_eq!(stick((0.06, 0.0), 0.1), (0.0, 0.0));
        assert!(stick((0.06, 0.0), 0.02).0 > 0.0);
    }

    #[test]
    fn calibration_uses_raw_independent_peaks_and_safety_margin() {
        let mut c = Calibration::new(0.0);
        let mut result = None;
        for i in 1..=280 {
            let t = i as f64 / 60.0;
            let raw = State { connected: true, left: (0.02 + (t * 2.0).sin() * 0.004, -0.01), right: (0.10, 0.04), ..Default::default() };
            if let Some(r) = c.sample(raw, t) { result = Some(r.unwrap()); break; }
        }
        let (left, right) = result.unwrap();
        assert_eq!((left, right), (0.05, 0.13));
        assert_eq!(stick((0.024, -0.01), left), (0.0, 0.0));
        assert_eq!(stick((0.10, 0.04), right), (0.0, 0.0));
        assert!(stick((0.055, 0.0), left).0 > 0.0);
    }

    #[test]
    fn calibration_rejects_movement_disconnects_sparse_samples_and_interruptions() {
        let rest = State { connected: true, ..Default::default() };
        let mut c = Calibration::new(0.0);
        assert!(c.sample(rest, 0.5).unwrap().is_err());
        let mut c = Calibration::new(0.0);
        assert!(c.sample(State::default(), 0.01).unwrap().is_err());
        let mut c = Calibration::new(0.0);
        let mut result = None;
        for i in 1..=30 { if let Some(r) = c.sample(rest, i as f64 * 0.2) { result = Some(r); break; } }
        assert!(result.unwrap().is_err());
        for step in [0.10, 0.5] {
            let mut c = Calibration::new(0.0);
            for i in 1..=100 { let _ = c.sample(rest, i as f64 / 60.0); }
            assert!(c.sample(State { left: (step, 0.0), ..rest }, 101.0 / 60.0).unwrap().is_err());
        }
    }

    #[test]
    fn cancelled_or_failed_calibration_never_returns_settings() {
        let mut pad = Gamepad::new();
        pad.start_calibration(0.0);
        assert!(!pad.calibrating());
        pad.raw.connected = true;
        pad.start_calibration(0.0);
        assert!(pad.calibrating());
        pad.cancel_calibration();
        assert!(!pad.calibrating());
        assert_eq!(pad.calibration_tick(5.0), None);
        pad.start_calibration(6.0);
        pad.raw.connected = false;
        assert_eq!(pad.calibration_tick(6.01), None);
        assert!(!pad.calibrating());
    }

    #[test]
    fn optional_l3_rudder_keeps_x_and_countermeasures_chord_distinct() {
        let mut pad = Gamepad::new();
        let mut st = Settings::default();
        for enabled in [false, true] {
            st.pad_rudder_click = enabled;
            pad.state.buttons = X;
            assert!(pad.rudder(&st));
            pad.state.buttons = LS;
            assert_eq!(pad.rudder(&st), enabled);
            assert_eq!(pad.countermeasures_pressed(&st), !enabled);
            pad.state.buttons = X | LS;
            assert!(pad.countermeasures_held(&st));
            assert!(pad.countermeasures_pressed(&st));
            pad.prev.buttons = LS;
            assert_eq!(pad.countermeasures_pressed(&st), enabled);
            pad.prev = State::default();
            pad.state.buttons = LS | RS;
            assert!(!pad.rudder(&st));
            assert!(!pad.countermeasures_held(&st));
        }
        st.pad_layout = PadLayout::Classic;
        pad.state.buttons = LS;
        assert!(!pad.rudder(&st));
        assert!(pad.countermeasures_pressed(&st));
    }
}
