//! Procedural audio: a tiny synth on its own thread. Output goes to waveOut on
//! Windows, and to PulseAudio or ALSA (loaded at runtime) on Linux. If no
//! device is available the game simply runs silent.
use std::f32::consts::TAU;
use std::sync::{Arc, Mutex};

pub const RATE: u32 = 44100;

#[derive(Clone, Copy, Default)]
pub struct Params {
    pub master: f32,
    pub eng_gain: f32,
    pub eng_freq: f32,
    pub wind_gain: f32,
    pub wind_freq: f32,
    pub rumble_gain: f32,
    pub rumble_freq: f32,
    pub whine_gain: f32,
    pub whine_freq: f32,
}

#[derive(Clone, Copy)]
pub enum Wave {
    Square,
    Saw,
}

struct Voice {
    wave: Wave,
    freq: f32,
    delay: u32,
    len: u32,
    vol: f32,
    t: u32,
    phase: f32,
}

#[derive(Default, Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    fn set(&mut self, lowpass: bool, freq: f32, q: f32) {
        let w = TAU * (freq / RATE as f32).clamp(0.0005, 0.49);
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q);
        let a0 = 1.0 + alpha;
        if lowpass {
            self.b0 = (1.0 - c) * 0.5 / a0;
            self.b1 = (1.0 - c) / a0;
            self.b2 = self.b0;
        } else {
            self.b0 = alpha / a0;
            self.b1 = 0.0;
            self.b2 = -alpha / a0;
        }
        self.a1 = -2.0 * c / a0;
        self.a2 = (1.0 - alpha) / a0;
    }
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Filtered noise hit; its low pass sweeps from `f0` down to `f1` as it decays.
struct Burst {
    delay: u32,
    t: u32,
    len: u32,
    vol: f32,
    f0: f32,
    f1: f32,
    lp: Biquad,
}

struct Synth {
    target: Params,
    cur: Params,
    seed: u32,
    brown: [f32; 2],
    eng: Biquad,
    wind: Biquad,
    rumble_lp: Biquad,
    tone_lp: Biquad,
    crash_lp: Biquad,
    rumble_phase: f32,
    whine_phase: f32,
    voices: Vec<Voice>,
    crash: Option<u32>,
    bursts: Vec<Burst>,
}

impl Synth {
    fn new() -> Synth {
        Synth {
            target: Params::default(),
            cur: Params::default(),
            seed: 0x1234567,
            brown: [0.0; 2],
            eng: Biquad::default(),
            wind: Biquad::default(),
            rumble_lp: Biquad::default(),
            tone_lp: Biquad::default(),
            crash_lp: Biquad::default(),
            rumble_phase: 0.0,
            whine_phase: 0.0,
            voices: Vec::new(),
            crash: None,
            bursts: Vec::new(),
        }
    }

    fn white(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn render(&mut self, out: &mut [i16]) {
        // Parameters glide like Web Audio's setTargetAtTime (tau about 0.1 s).
        let block = out.len() as f32 / RATE as f32;
        let k = 1.0 - (-block / 0.09f32).exp();
        let km = 1.0 - (-block / 0.03f32).exp();
        let (c, t) = (&mut self.cur, &self.target);
        let glide = |a: &mut f32, b: f32, k: f32| *a += (b - *a) * k;
        glide(&mut c.master, t.master, km);
        glide(&mut c.eng_gain, t.eng_gain, k);
        glide(&mut c.eng_freq, t.eng_freq, k);
        glide(&mut c.wind_gain, t.wind_gain, k);
        glide(&mut c.wind_freq, t.wind_freq, k);
        glide(&mut c.rumble_gain, t.rumble_gain, k);
        glide(&mut c.rumble_freq, t.rumble_freq, k);
        glide(&mut c.whine_gain, t.whine_gain, k);
        glide(&mut c.whine_freq, t.whine_freq, k);
        let p = self.cur;
        self.eng.set(true, p.eng_freq.max(40.0), 0.6);
        self.wind.set(false, p.wind_freq.max(60.0), 0.35);
        self.rumble_lp.set(true, 120.0, 0.7);
        for s in out.iter_mut() {
            let w0 = self.white();
            let w1 = self.white();
            self.brown[0] = self.brown[0] * 0.965 + w0 * 0.035;
            self.brown[1] = self.brown[1] * 0.955 + w1 * 0.045;
            let mut v = self.eng.run(self.brown[0] * 5.5) * p.eng_gain;
            v += self.wind.run(self.brown[1] * 5.5) * p.wind_gain;
            self.rumble_phase = (self.rumble_phase + p.rumble_freq / RATE as f32).fract();
            v += self.rumble_lp.run(self.rumble_phase * 2.0 - 1.0) * p.rumble_gain;
            self.whine_phase = (self.whine_phase + p.whine_freq / RATE as f32).fract();
            v += (self.whine_phase * TAU).sin() * p.whine_gain;

            let mut tones = 0.0;
            for voice in self.voices.iter_mut() {
                if voice.delay > 0 {
                    voice.delay -= 1;
                    continue;
                }
                let tt = voice.t as f32 / RATE as f32;
                let dur = voice.len as f32 / RATE as f32;
                let env = if tt < 0.012 { tt / 0.012 } else { (0.0001f32.ln() * ((tt - 0.012) / (dur - 0.012).max(0.001))).exp() };
                voice.phase = (voice.phase + voice.freq / RATE as f32).fract();
                let osc = match voice.wave {
                    Wave::Square => if voice.phase < 0.5 { 1.0 } else { -1.0 },
                    Wave::Saw => voice.phase * 2.0 - 1.0,
                };
                tones += osc * env * voice.vol;
                voice.t += 1;
            }
            v += self.tone_lp.run(tones);
            self.voices.retain(|vc| vc.t < vc.len);

            let noise = self.white();
            for b in self.bursts.iter_mut() {
                if b.delay > 0 {
                    b.delay -= 1;
                    continue;
                }
                let u = b.t as f32 / b.len as f32;
                if b.t % 64 == 0 {
                    b.lp.set(true, b.f0 * (b.f1 / b.f0).powf(u), 0.8);
                }
                let attack = (b.t as f32 / (RATE as f32 * 0.004)).min(1.0);
                v += b.lp.run(noise * attack * (1.0 - u) * (1.0 - u)) * b.vol;
                b.t += 1;
            }
            self.bursts.retain(|b| b.t < b.len);

            if let Some(n) = self.crash {
                let total = (RATE as f32 * 1.4) as u32;
                let t = n as f32 / total as f32;
                if n % 64 == 0 {
                    let f = 2800.0 * (120.0f32 / 2800.0).powf((n as f32 / (RATE as f32 * 1.1)).min(1.0));
                    self.crash_lp.set(true, f, 0.7);
                }
                let w = self.white();
                v += self.crash_lp.run(w * (1.0 - t).powf(2.2) * 0.8) * 0.5;
                self.crash = if n + 1 < total { Some(n + 1) } else { None };
            }
            *s = ((v * p.master).clamp(-1.0, 1.0) * 32000.0) as i16;
        }
    }
}

#[derive(Clone)]
pub struct Audio {
    synth: Arc<Mutex<Synth>>,
    pub available: bool,
}

impl Audio {
    pub fn start() -> Audio {
        let synth = Arc::new(Mutex::new(Synth::new()));
        if let Ok(mut s) = synth.lock() {
            s.tone_lp.set(true, 6000.0, 0.7);
        }
        let available = backend::spawn(synth.clone());
        Audio { synth, available }
    }

    pub fn set(&self, p: Params) {
        if let Ok(mut s) = self.synth.lock() {
            s.target = p;
        }
    }

    pub fn tone(&self, freq: f32, duration: f32, volume: f32, wave: Wave, delay: f32) {
        if let Ok(mut s) = self.synth.lock() {
            s.voices.push(Voice { wave, freq, delay: (delay * RATE as f32) as u32, len: ((duration + 0.03) * RATE as f32) as u32, vol: volume, t: 0, phase: 0.0 });
        }
    }

    pub fn burst(&self, duration: f32, volume: f32, f0: f32, f1: f32, delay: f32) {
        if let Ok(mut s) = self.synth.lock() {
            let b = Burst { delay: (delay * RATE as f32) as u32, t: 0, len: (duration * RATE as f32) as u32, vol: volume, f0, f1, lp: Biquad::default() };
            s.bursts.push(b);
        }
    }

    /// Impact: the noise sweep, a deep boom and two secondary blasts; a
    /// hissing splash instead on water.
    pub fn crash(&self, water: bool) {
        if let Ok(mut s) = self.synth.lock() {
            s.crash = Some(0);
        }
        if water {
            self.burst(1.4, 1.1, 6000.0, 500.0, 0.0);
            self.burst(0.9, 3.0, 260.0, 40.0, 0.0);
            return;
        }
        self.burst(1.8, 5.0, 320.0, 32.0, 0.0);
        self.burst(0.9, 2.4, 1100.0, 90.0, 0.42);
        self.burst(1.1, 2.0, 900.0, 70.0, 0.95);
    }

    /// Afterburner light-off: a low thump and a short crackle.
    pub fn afterburner(&self) {
        self.burst(0.55, 3.2, 230.0, 48.0, 0.0);
        self.burst(0.35, 0.7, 2600.0, 380.0, 0.02);
    }

    pub fn gate(&self) {
        self.tone(660.0, 0.1, 0.045, Wave::Square, 0.0);
        self.tone(990.0, 0.17, 0.04, Wave::Square, 0.09);
    }

    pub fn warn_pull_up(&self) {
        self.tone(640.0, 0.12, 0.05, Wave::Saw, 0.0);
        self.tone(520.0, 0.12, 0.045, Wave::Saw, 0.15);
    }

    pub fn warn_stall(&self) {
        self.tone(410.0, 0.15, 0.035, Wave::Square, 0.0);
    }
}

fn fill(synth: &Arc<Mutex<Synth>>, buf: &mut [i16]) {
    match synth.lock() {
        Ok(mut s) => s.render(buf),
        Err(_) => buf.fill(0),
    }
}

#[cfg(windows)]
mod backend {
    use super::*;
    use std::ffi::c_void;

    #[repr(C)]
    struct WaveFormatEx {
        format_tag: u16,
        channels: u16,
        samples_per_sec: u32,
        avg_bytes_per_sec: u32,
        block_align: u16,
        bits_per_sample: u16,
        cb_size: u16,
    }
    #[repr(C)]
    struct WaveHdr {
        data: *mut i8,
        buffer_length: u32,
        bytes_recorded: u32,
        user: usize,
        flags: u32,
        loops: u32,
        next: *mut WaveHdr,
        reserved: usize,
    }
    #[link(name = "winmm")]
    extern "system" {
        fn waveOutOpen(h: *mut *mut c_void, device: u32, fmt: *const WaveFormatEx, cb: usize, inst: usize, flags: u32) -> u32;
        fn waveOutPrepareHeader(h: *mut c_void, hdr: *mut WaveHdr, size: u32) -> u32;
        fn waveOutWrite(h: *mut c_void, hdr: *mut WaveHdr, size: u32) -> u32;
    }
    const WHDR_DONE: u32 = 1;
    const FRAMES: usize = 512;
    const BUFFERS: usize = 6;

    pub fn spawn(synth: Arc<Mutex<Synth>>) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || unsafe {
                let fmt = WaveFormatEx { format_tag: 1, channels: 1, samples_per_sec: RATE, avg_bytes_per_sec: RATE * 2, block_align: 2, bits_per_sample: 16, cb_size: 0 };
                let mut h: *mut c_void = std::ptr::null_mut();
                if waveOutOpen(&mut h, 0xFFFF_FFFF, &fmt, 0, 0, 0) != 0 {
                    let _ = tx.send(false);
                    return;
                }
                let _ = tx.send(true);
                let mut bufs: Vec<Vec<i16>> = (0..BUFFERS).map(|_| vec![0i16; FRAMES]).collect();
                let mut hdrs: Vec<WaveHdr> = bufs
                    .iter_mut()
                    .map(|b| WaveHdr { data: b.as_mut_ptr() as *mut i8, buffer_length: (FRAMES * 2) as u32, bytes_recorded: 0, user: 0, flags: 0, loops: 0, next: std::ptr::null_mut(), reserved: 0 })
                    .collect();
                let size = std::mem::size_of::<WaveHdr>() as u32;
                for (i, hdr) in hdrs.iter_mut().enumerate() {
                    fill(&synth, &mut bufs[i]);
                    waveOutPrepareHeader(h, hdr, size);
                    waveOutWrite(h, hdr, size);
                }
                loop {
                    let mut wrote = false;
                    for i in 0..BUFFERS {
                        let done = std::ptr::read_volatile(&hdrs[i].flags) & WHDR_DONE != 0;
                        if done {
                            fill(&synth, &mut bufs[i]);
                            hdrs[i].flags &= !WHDR_DONE;
                            waveOutWrite(h, &mut hdrs[i], size);
                            wrote = true;
                        }
                    }
                    if !wrote {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                }
            })
            .ok();
        rx.recv().unwrap_or(false)
    }
}

#[cfg(target_os = "linux")]
mod backend {
    use super::*;
    use std::ffi::{c_char, c_int, c_void, CString};

    unsafe fn sym<T>(lib: *mut c_void, name: &str) -> Option<T> {
        let n = CString::new(name).ok()?;
        let p = libc::dlsym(lib, n.as_ptr());
        if p.is_null() {
            None
        } else {
            Some(std::mem::transmute_copy(&p))
        }
    }

    unsafe fn open(names: &[&str]) -> *mut c_void {
        for n in names {
            let c = CString::new(*n).unwrap();
            let h = libc::dlopen(c.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
            if !h.is_null() {
                return h;
            }
        }
        std::ptr::null_mut()
    }

    #[repr(C)]
    struct PaSampleSpec {
        format: c_int,
        rate: u32,
        channels: u8,
    }
    #[repr(C)]
    struct PaBufferAttr {
        maxlength: u32,
        tlength: u32,
        prebuf: u32,
        minreq: u32,
        fragsize: u32,
    }
    type PaNew = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char, *const c_char, *const PaSampleSpec, *const c_void, *const PaBufferAttr, *mut c_int) -> *mut c_void;
    type PaWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize, *mut c_int) -> c_int;

    type AlsaOpen = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
    type AlsaSetParams = unsafe extern "C" fn(*mut c_void, c_int, c_int, u32, u32, c_int, u32) -> c_int;
    type AlsaWritei = unsafe extern "C" fn(*mut c_void, *const c_void, libc::c_ulong) -> libc::c_long;
    type AlsaRecover = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;

    const FRAMES: usize = 512;

    pub fn spawn(synth: Arc<Mutex<Synth>>) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || unsafe {
                let mut buf = vec![0i16; FRAMES];
                // PulseAudio (also served by PipeWire and WSLg).
                let pa = open(&["libpulse-simple.so.0", "libpulse-simple.so"]);
                if !pa.is_null() {
                    if let (Some(new), Some(write)) = (sym::<PaNew>(pa, "pa_simple_new"), sym::<PaWrite>(pa, "pa_simple_write")) {
                        let spec = PaSampleSpec { format: 3, rate: RATE, channels: 1 };
                        let attr = PaBufferAttr { maxlength: u32::MAX, tlength: (FRAMES * 2 * 4) as u32, prebuf: u32::MAX, minreq: (FRAMES * 2) as u32, fragsize: u32::MAX };
                        let name = CString::new("Viper Run").unwrap();
                        let stream = CString::new("engine").unwrap();
                        let mut err = 0;
                        let s = new(std::ptr::null(), name.as_ptr(), 1, std::ptr::null(), stream.as_ptr(), &spec, std::ptr::null(), &attr, &mut err);
                        if !s.is_null() {
                            let _ = tx.send(true);
                            loop {
                                fill(&synth, &mut buf);
                                if write(s, buf.as_ptr() as *const c_void, FRAMES * 2, &mut err) < 0 {
                                    return;
                                }
                            }
                        }
                    }
                }
                // Plain ALSA.
                let al = open(&["libasound.so.2", "libasound.so"]);
                if !al.is_null() {
                    if let (Some(op), Some(setp), Some(wr), Some(rec)) = (
                        sym::<AlsaOpen>(al, "snd_pcm_open"),
                        sym::<AlsaSetParams>(al, "snd_pcm_set_params"),
                        sym::<AlsaWritei>(al, "snd_pcm_writei"),
                        sym::<AlsaRecover>(al, "snd_pcm_recover"),
                    ) {
                        let dev = CString::new("default").unwrap();
                        let mut pcm: *mut c_void = std::ptr::null_mut();
                        if op(&mut pcm, dev.as_ptr(), 0, 0) >= 0 && setp(pcm, 2, 3, 1, RATE, 1, 60000) >= 0 {
                            let _ = tx.send(true);
                            loop {
                                fill(&synth, &mut buf);
                                let r = wr(pcm, buf.as_ptr() as *const c_void, FRAMES as libc::c_ulong);
                                if r < 0 && rec(pcm, r as c_int, 1) < 0 {
                                    return;
                                }
                            }
                        }
                    }
                }
                let _ = tx.send(false);
            })
            .ok();
        rx.recv().unwrap_or(false)
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod backend {
    use super::*;
    pub fn spawn(_synth: Arc<Mutex<Synth>>) -> bool {
        false
    }
}
