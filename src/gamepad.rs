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
    pub state: State,
    prev: State,
}

/// Radial dead zone, rescaled so the output rises from zero at its edge.
fn stick((x, y): (f64, f64)) -> (f64, f64) {
    const DEAD: f64 = 0.15;
    let m = (x * x + y * y).sqrt();
    if m <= DEAD {
        return (0.0, 0.0);
    }
    let k = ((m - DEAD) / (1.0 - DEAD)).min(1.0) / m;
    (x * k, y * k)
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
        Gamepad { device: backend::Device::new(), state: State::default(), prev: State::default() }
    }

    /// Reads the controller; call once per frame. Returns the new connection
    /// state when a controller was plugged in or removed.
    pub fn poll(&mut self, now: f64) -> Option<bool> {
        self.prev = self.state;
        self.state = self.device.read(now).map(|r| State { left: stick(r.left), right: stick(r.right), lt: trigger(r.lt), rt: trigger(r.rt), ..r }).unwrap_or_default();
        (self.state.connected != self.prev.connected).then_some(self.state.connected)
    }

    pub fn held(&self, b: u16) -> bool {
        self.state.buttons & b != 0
    }

    /// True on the frame the button goes down.
    pub fn pressed(&self, b: u16) -> bool {
        self.held(b) && self.prev.buttons & b == 0
    }

    /// Menu directions newly pushed this frame.
    pub fn nav(&self) -> u16 {
        dirs(&self.state) & !dirs(&self.prev)
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
    const BUTTONS: [u16; 15] = [A, B, X, Y, LB, RB, BACK, START, 0, 0, RS, LEFT, RIGHT, UP, DOWN];

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
