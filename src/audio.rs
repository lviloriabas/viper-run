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
    pub engines: f32,
    pub burner: f32,
    pub wind_gain: f32,
    pub wind_freq: f32,
    pub rumble_gain: f32,
    pub rumble_freq: f32,
    pub whine_gain: f32,
    pub whine_freq: f32,
    /// Gun firing: 0 or its level, and its rate of fire in rounds a second
    /// (the M61A1's 100 tear, the GSh-30-1's 25 to 30 hammer).
    pub gun: f32,
    pub gun_rate: f32,
    /// Sidewinder seeker tone: a low growl searching, a high tone locked.
    pub seeker_gain: f32,
    pub seeker_freq: f32,
}

#[derive(Clone, Copy)]
pub enum Wave {
    Square,
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

struct Speech { pcm: &'static [u8], rate: u32, at: f64, priority: u8 }
impl Speech {
    fn new(wav: &'static [u8], priority: u8) -> Option<Self> {
        if wav.get(..4)? != b"RIFF" || wav.get(8..12)? != b"WAVE" { return None; }
        let mut pos = 12;
        let mut rate = 0;
        while pos + 8 <= wav.len() {
            let len = u32::from_le_bytes(wav[pos+4..pos+8].try_into().ok()?) as usize;
            let data = wav.get(pos+8..pos+8+len)?;
            if &wav[pos..pos+4] == b"fmt " {
                if data.len() < 16 || data[..2] != [1, 0] || data[2..4] != [1, 0] || data[14..16] != [16, 0] { return None; }
                rate = u32::from_le_bytes(data[4..8].try_into().ok()?);
            }
            if &wav[pos..pos+4] == b"data" && rate > 0 {
                return Some(Self { pcm: data, rate, at: 0.0, priority });
            }
            pos += 8 + len + len % 2;
        }
        None
    }
    fn next(&mut self) -> Option<f32> {
        let i = self.at as usize;
        if i * 2 + 1 >= self.pcm.len() { return None; }
        let read = |i: usize| i16::from_le_bytes([self.pcm[i*2], self.pcm[i*2+1]]) as f32 / 32768.0;
        let a = read(i);
        let b = if (i+1)*2+1 < self.pcm.len() { read(i+1) } else { a };
        let value = a + (b-a) * self.at.fract() as f32;
        self.at += self.rate as f64 / RATE as f64;
        Some(value)
    }
}

/// A small, bounded voice bus, separate from the procedural game master.
/// Network frames are resampled before entering the audio callback.
#[derive(Default)]
struct RemoteVoice {
    pcm: std::collections::VecDeque<f32>,
    target_gain: f32,
    gain: f32,
    envelope: f32,
    last: f32,
}
#[derive(Default)]
struct VoiceBus { peers: std::collections::BTreeMap<u64, RemoteVoice>, activity: f32 }
impl VoiceBus {
    fn enqueue(&mut self, peer: u64, samples: &[i16], rate: u32, gain: f32) -> bool {
        if !(8000..=48000).contains(&rate) || samples.is_empty()
            || samples.len() > rate as usize / 5 || !gain.is_finite() { return false; }
        if !self.peers.contains_key(&peer) && self.peers.len() >= 16 { return false; }
        let stream = self.peers.entry(peer).or_default();
        stream.target_gain = gain.clamp(0.0, 4.0);
        let count = samples.len() * RATE as usize / rate as usize;
        // At most 120 ms per participant. Drop oldest speech after a stall.
        let limit = RATE as usize * 120 / 1000;
        if stream.pcm.len() + count > limit {
            let discard = (stream.pcm.len() + count).saturating_sub(limit).min(stream.pcm.len());
            stream.pcm.drain(..discard); stream.envelope = 0.0;
        }
        let skip = count.saturating_sub(limit);
        for index in skip..count {
            let at = index as f64 * rate as f64 / RATE as f64;
            let lower = at as usize;
            let a = samples[lower.min(samples.len()-1)] as f32 / 32768.0;
            let b = samples[(lower+1).min(samples.len()-1)] as f32 / 32768.0;
            stream.pcm.push_back(a + (b-a) * at.fract() as f32);
        }
        true
    }
    fn sample(&mut self) -> f32 {
        let mut mixed = 0.0;
        for peer in self.peers.values_mut() {
            peer.gain += (peer.target_gain - peer.gain) * 0.003;
            if let Some(sample) = peer.pcm.pop_front() {
                peer.envelope += (1.0 - peer.envelope) * 0.008;
                peer.last = sample;
            } else {
                peer.envelope *= 0.99;
                peer.last *= 0.98;
            }
            mixed += peer.last * peer.gain * peer.envelope;
        }
        // Keep the bus bounded even when every participant speaks at once.
        let smoothing = if mixed.abs() > self.activity { 0.01 } else { 0.0003 };
        self.activity += (mixed.abs() - self.activity) * smoothing;
        mixed.tanh() * 0.30
    }
}

struct Synth {
    target: Params,
    cur: Params,
    seed: u32,
    brown: [f32; 2],
    eng: [Biquad; 2],
    engine_phase: [f32; 2],
    engine_drift: [f32; 2],
    wind: Biquad,
    rumble_lp: Biquad,
    tone_lp: Biquad,
    crash_lp: Biquad,
    gun_lp: Biquad,
    gun_phase: f32,
    gun_env: f32,
    seeker_phase: f32,
    speech: Option<Speech>,
    chat: VoiceBus,
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
            eng: [Biquad::default(); 2],
            engine_phase: [0.0, 0.37],
            engine_drift: [0.0; 2],
            wind: Biquad::default(),
            rumble_lp: Biquad::default(),
            tone_lp: Biquad::default(),
            crash_lp: Biquad::default(),
            gun_lp: Biquad::default(),
            gun_phase: 0.0,
            gun_env: 0.0,
            seeker_phase: 0.0,
            speech: None,
            chat: VoiceBus::default(),
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
        glide(&mut c.burner, t.burner, k);
        c.engines = t.engines;
        glide(&mut c.wind_gain, t.wind_gain, k);
        glide(&mut c.wind_freq, t.wind_freq, k);
        glide(&mut c.rumble_gain, t.rumble_gain, k);
        glide(&mut c.rumble_freq, t.rumble_freq, k);
        glide(&mut c.whine_gain, t.whine_gain, k);
        glide(&mut c.whine_freq, t.whine_freq, k);
        glide(&mut c.gun, t.gun, km);
        c.gun_rate = t.gun_rate;
        glide(&mut c.seeker_gain, t.seeker_gain, km);
        c.seeker_freq = t.seeker_freq;
        let p = self.cur;
        for (i, eng) in self.eng.iter_mut().enumerate() {
            eng.set(true, (p.eng_freq * (1.0 + i as f32 * 0.06)).max(40.0), 0.6);
        }
        self.wind.set(false, p.wind_freq.max(60.0), 0.35);
        self.rumble_lp.set(true, (p.rumble_freq * 2.5).max(50.0), 0.7);
        // The 30 mm's slower, heavier reports sit lower than the 20 mm's.
        self.gun_lp.set(true, if p.gun_rate < 50.0 { 900.0 } else { 1600.0 }, 0.9);
        for s in out.iter_mut() {
            let w0 = self.white();
            let w1 = self.white();
            self.brown[0] = self.brown[0] * 0.965 + w0 * 0.035;
            self.brown[1] = self.brown[1] * 0.955 + w1 * 0.045;
            let noise = [w0, w1];
            let engines = if p.engines > 1.5 { 2 } else { 1 };
            let mut v = 0.0;
            for (i, &n) in noise.iter().enumerate().take(engines) {
                self.engine_drift[i] = self.engine_drift[i] * 0.99997 + n * 0.00003;
                let drift = self.engine_drift[i] * 8.0;
                let turbine = p.whine_freq * (1.0 + i as f32 * 0.017 + drift * 0.006);
                self.engine_phase[i] = (self.engine_phase[i] + turbine / RATE as f32).fract();
                let phase = self.engine_phase[i] * TAU;
                let air = self.eng[i].run(n * 0.55 + self.brown[i] * 3.0);
                let whine = phase.sin() * 0.7 + (phase * 2.01 + drift).sin() * 0.12;
                v += (air * p.eng_gain * (1.0 + drift * 0.08)
                    + whine * p.whine_gain + n * p.burner * p.eng_gain * 0.18) / (engines as f32).sqrt();
            }
            v += self.wind.run(self.brown[1] * 5.5) * p.wind_gain;
            v += self.rumble_lp.run((self.brown[0] + self.brown[1]) * 5.0) * p.rumble_gain;
            // The gun: a crack of noise for each round, which at 100 a second
            // the ear hears as one tearing buzz, and a thump of the breech
            // under it.
            if p.gun > 0.001 {
                self.gun_phase += p.gun_rate.max(1.0) / RATE as f32;
                if self.gun_phase >= 1.0 {
                    self.gun_phase -= 1.0;
                    self.gun_env = 1.0;
                }
                let crack = self.white() * self.gun_env + (self.gun_env - 0.5) * 1.4;
                self.gun_env *= 0.9975;
                v += self.gun_lp.run(crack) * p.gun;
            }
            if p.seeker_gain > 0.0005 {
                self.seeker_phase = (self.seeker_phase + p.seeker_freq / RATE as f32).fract();
                v += (self.seeker_phase * TAU).sin() * p.seeker_gain;
            }

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
            if let Some(voice) = &mut self.speech {
                if let Some(sample) = voice.next() { v = v * 0.48 + sample * 0.48; }
                else { self.speech = None; }
            }
            let chat = self.chat.sample();
            // Crew voice stays audible when game audio is paused. VMS has
            // priority over crew speech, with headroom for both buses.
            let chat = if self.speech.is_some() { chat * 0.55 } else { chat };
            let game = v * p.master * (1.0 - 0.30 * (self.chat.activity * 20.0).min(1.0));
            *s = ((game + chat).clamp(-1.0, 1.0) * 32000.0) as i16;
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

    /// Queue mono PCM for one authenticated participant. At most 16 streams,
    /// 120 ms per stream, 200 ms per call; gain is limited to 0..4.
    pub fn voice_pcm(&self, peer: u64, samples: &[i16], rate: u32, gain: f32) -> bool {
        self.synth.lock().is_ok_and(|mut synth| synth.chat.enqueue(peer, samples, rate, gain))
    }
    /// None clears every participant; Some clears one. Mute/deafen must call
    /// this before submitting new frames, including when the game is paused.
    pub fn voice_clear(&self, peer: Option<u64>) {
        if let Ok(mut synth) = self.synth.lock() {
            if let Some(peer) = peer { synth.chat.peers.remove(&peer); }
            else { synth.chat = VoiceBus::default(); }
        }
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

    /// Sonic boom: the N-wave's two pressure jumps, bow shock then tail
    /// shock a tenth of a second apart, over a rolling low rumble.
    pub fn sonic_boom(&self) {
        self.burst(0.32, 5.0, 1400.0, 80.0, 0.0);
        self.burst(0.38, 4.2, 1100.0, 60.0, 0.11);
        self.burst(1.6, 2.2, 160.0, 30.0, 0.02);
    }

    /// The AGSM's quick breath through the mask: a sharp exhale, then the
    /// snatched inhale before the next strain.
    pub fn breath(&self) {
        self.burst(0.16, 0.5, 2400.0, 900.0, 0.0);
        self.burst(0.32, 0.32, 1500.0, 420.0, 0.14);
    }

    pub fn gate(&self) {
        self.tone(660.0, 0.1, 0.045, Wave::Square, 0.0);
        self.tone(990.0, 0.17, 0.04, Wave::Square, 0.09);
    }

    pub fn warning(&self, kind: crate::sim::Type, cue: crate::warnings::Cue) {
        use crate::warnings::Cue;
        if !kind.russian() && matches!(cue, Cue::LowSpeed | Cue::HighAoa | Cue::Gear) {
            let frequency = if cue == Cue::Gear { 800.0 } else { 300.0 };
            self.tone(frequency, 0.25, 0.10, Wave::Square, 0.0);
            self.tone(frequency, 0.25, 0.10, Wave::Square, 0.38);
            return;
        }
        let wav: &'static [u8] = match (kind.russian(), cue) {
            (false, Cue::PullUp) => include_bytes!("../assets/voices/f16-pullup.wav"),
            (false, Cue::Altitude) => include_bytes!("../assets/voices/f16-altitude.wav"),
            (false, Cue::Fire) => include_bytes!("../assets/voices/f16-warning.wav"),
            (false, _) => include_bytes!("../assets/voices/f16-caution.wav"),
            (true, Cue::PullUp | Cue::Altitude) => include_bytes!("../assets/voices/mig-altitude.wav"),
            (true, Cue::HighAoa) => include_bytes!("../assets/voices/mig-aoa.wav"),
            (true, Cue::LowSpeed | Cue::OverSpeed) => include_bytes!("../assets/voices/mig-speed.wav"),
            (true, Cue::OverG) => include_bytes!("../assets/voices/mig-g.wav"),
            (true, Cue::Gear) => include_bytes!("../assets/voices/mig-gear.wav"),
            (true, Cue::Fire) => include_bytes!("../assets/voices/mig-fire.wav"),
        };
        if let Ok(mut s) = self.synth.lock() {
            if s.speech.as_ref().is_none_or(|v| cue.priority() > v.priority) {
                s.speech = Speech::new(wav, cue.priority());
            }
        }
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

/// A reproducible demonstration: idle, spool up, dry power, afterburner, then VMS.
pub fn demo(path: &str, kind: crate::sim::Type) -> std::io::Result<()> {
    use std::io::Write;
    let mut synth = Synth::new();
    synth.tone_lp.set(true, 6000.0, 0.7);
    let mut samples = Vec::new();
    let blocks = (RATE as usize * 12) / 512;
    for block in 0..blocks {
        let time = block as f32 * 512.0 / RATE as f32;
        let rpm = ((time - 2.0) / 4.0).clamp(0.0, 1.0);
        let burner = if (7.0..10.0).contains(&time) { 1.0 } else { 0.0 };
        synth.target = Params { master: 0.65, engines: if kind == crate::sim::Type::F16 { 1.0 } else { 2.0 },
            eng_gain: 0.06 + rpm * 0.06 + burner * 0.06, eng_freq: 450.0 + rpm * 1900.0,
            whine_gain: 0.002 + rpm * 0.006, whine_freq: 750.0 + rpm * 2250.0,
            rumble_gain: 0.02 + burner * 0.02, rumble_freq: 40.0 + rpm * 30.0, burner,
            ..Default::default() };
        if block == (RATE as usize * 10) / 512 {
            let wav: &'static [u8] = if kind.russian() {
                include_bytes!("../assets/voices/mig-altitude.wav")
            } else { include_bytes!("../assets/voices/f16-pullup.wav") };
            synth.speech = Speech::new(wav, 8);
        }
        let mut buf = [0i16; 512];
        synth.render(&mut buf);
        samples.extend_from_slice(&buf);
    }
    let bytes = samples.len() as u32 * 2;
    let mut f = std::fs::File::create(path)?;
    f.write_all(b"RIFF")?; f.write_all(&(36 + bytes).to_le_bytes())?; f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?; f.write_all(&1u16.to_le_bytes())?; f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&RATE.to_le_bytes())?; f.write_all(&(RATE*2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?; f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?; f.write_all(&bytes.to_le_bytes())?;
    for s in samples { f.write_all(&s.to_le_bytes())?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_voice_is_decoded_and_resampled_without_a_loop() {
        let mut speech = Speech::new(include_bytes!("../assets/voices/f16-pullup.wav"), 8).unwrap();
        let mut count = 0; let mut energy = 0.0;
        while let Some(x) = speech.next() { count += 1; energy += x*x; assert!(x.is_finite() && x.abs() <= 1.0); }
        assert!(count > RATE && count < RATE*3 && energy > 10.0);
        assert!(speech.next().is_none());
    }
    #[test]
    fn all_embedded_warnings_have_clean_boundaries_and_usable_levels() {
        let clips: [(&str, &[u8]); 10] = [
            ("f16-pullup", include_bytes!("../assets/voices/f16-pullup.wav")),
            ("f16-altitude", include_bytes!("../assets/voices/f16-altitude.wav")),
            ("f16-warning", include_bytes!("../assets/voices/f16-warning.wav")),
            ("f16-caution", include_bytes!("../assets/voices/f16-caution.wav")),
            ("mig-altitude", include_bytes!("../assets/voices/mig-altitude.wav")),
            ("mig-aoa", include_bytes!("../assets/voices/mig-aoa.wav")),
            ("mig-speed", include_bytes!("../assets/voices/mig-speed.wav")),
            ("mig-g", include_bytes!("../assets/voices/mig-g.wav")),
            ("mig-gear", include_bytes!("../assets/voices/mig-gear.wav")),
            ("mig-fire", include_bytes!("../assets/voices/mig-fire.wav")),
        ];
        for (name, wav) in clips {
            let mut speech = Speech::new(wav, 8).expect(name);
            assert_eq!(speech.rate, 22050, "{name}: source sample rate");
            let source_frames = speech.pcm.len() / 2;
            let samples: Vec<f32> = std::iter::from_fn(|| speech.next()).collect();
            assert_eq!(samples.len(), source_frames * 2, "{name}: resampling duration");
            assert!(samples.len() > RATE as usize / 2 && samples.len() < RATE as usize * 3,
                "{name}: warning must stay concise");
            assert!(samples.iter().all(|x| x.is_finite() && x.abs() <= 24000.0 / 32768.0),
                "{name}: peak headroom");
            assert_eq!(samples[0], 0.0, "{name}: no click at onset");
            assert_eq!(*samples.last().unwrap(), 0.0, "{name}: no click at end");
            let rms = (samples.iter().map(|x| x*x).sum::<f32>() / samples.len() as f32).sqrt();
            assert!(rms > 0.045 && rms < 0.18, "{name}: audible and consistent level ({rms})");
            let first = samples.iter().position(|x| x.abs() > 0.01).expect(name);
            let last = samples.iter().rposition(|x| x.abs() > 0.01).expect(name);
            assert!(first < RATE as usize / 5, "{name}: excessive leading silence");
            assert!(samples.len() - last < RATE as usize / 4, "{name}: excessive trailing silence");
            assert!(speech.next().is_none(), "{name}: speech must finish");
        }
    }

    #[test]
    fn crew_voice_streams_are_resampled_bounded_and_cleared_without_a_device() {
        let audio = Audio { synth: Arc::new(Mutex::new(Synth::new())), available: false };
        let samples: Vec<i16> = (0..320).map(|i| (i as f32 * 0.2).sin().mul_add(10000.0, 0.0) as i16).collect();
        assert!(!audio.voice_pcm(1, &samples, 0, 1.0));
        assert!(!audio.voice_pcm(1, &samples, 16000, f32::NAN));
        for _ in 0..100 { assert!(audio.voice_pcm(1, &samples, 16000, 1.0)); }
        {
            let synth = audio.synth.lock().unwrap();
            assert_eq!(synth.chat.peers[&1].pcm.len(), RATE as usize * 120 / 1000);
        }
        let mut output = vec![0i16; RATE as usize / 20];
        audio.synth.lock().unwrap().render(&mut output);
        // The game bus is silent/paused; crew voice remains available.
        assert!(output.iter().any(|x| x.abs() > 100));
        assert!(output.iter().all(|x| x.abs() < 32000));
        audio.voice_clear(Some(1));
        assert!(audio.synth.lock().unwrap().chat.peers.is_empty());
        for id in 0..16 { assert!(audio.voice_pcm(id, &samples, 16000, 4.0)); }
        assert!(!audio.voice_pcm(17, &samples, 16000, 1.0));
        audio.synth.lock().unwrap().render(&mut output);
        assert!(output.iter().all(|x| x.abs() < 32000));
        audio.voice_clear(None);
        audio.synth.lock().unwrap().render(&mut output);
        assert!(output.iter().all(|&x| x == 0));
    }

    #[test]
    fn engine_output_is_continuous_nonrepeating_and_not_clipped_for_both_airframes() {
        for engines in [1.0, 2.0] {
            let mut s = Synth::new();
            s.target = Params { master: 0.7, eng_gain: 0.18, eng_freq: 2200.0, engines, burner: 1.0,
                whine_freq: 2800.0, whine_gain: 0.008, rumble_gain: 0.04, rumble_freq: 70.0, ..Default::default() };
            let mut previous = [0i16; 512];
            for i in 0..300 {
                let mut next = [0i16; 512]; s.render(&mut next);
                assert!(next.iter().any(|&x| x.abs() > 10));
                assert!(next.iter().all(|x| x.abs() < 31000));
                if i > 0 { assert_ne!(next, previous, "audio buffer repeated"); }
                previous = next;
            }
        }
    }
}
