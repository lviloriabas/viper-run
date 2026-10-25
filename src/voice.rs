//! Opt-in multiplayer voice. No device is opened by new(), tests, or the decoder.
//! Packets contain independently decodable IMA ADPCM, 16 kHz mono, 20 ms.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const SAMPLE_RATE: u32 = 16_000;
pub const FRAME_SAMPLES: usize = 320;
pub const PACKET_BYTES: usize = 164;
pub const MAX_PEERS: usize = 16;
const FRAME_TIME: Duration = Duration::from_millis(20);
const MAX_QUEUE: usize = 8;
const JITTER_LIMIT: u32 = 12;
const JITTER_DELAY: Duration = Duration::from_millis(60);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VoiceMode {
    #[default]
    Disabled,
    PushToTalk,
    VoiceActivation,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceSettings {
    pub mode: VoiceMode,
    pub input_gain: f32,
    pub output_gain: f32,
    /// RMS after input gain, in the range 0.001..0.5.
    pub activation_threshold: f32,
    pub hold_ms: u32,
    pub muted: bool,
    pub deafened: bool,
}
impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            mode: VoiceMode::Disabled,
            input_gain: 1.0,
            output_gain: 1.0,
            activation_threshold: 0.02,
            hold_ms: 240,
            muted: false,
            deafened: false,
        }
    }
}
fn finite(value: f32, fallback: f32, low: f32, high: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        fallback
    }
}
impl VoiceSettings {
    fn sanitise(mut self) -> Self {
        self.input_gain = finite(self.input_gain, 1.0, 0.0, 4.0);
        self.output_gain = finite(self.output_gain, 1.0, 0.0, 2.0);
        self.activation_threshold = finite(self.activation_threshold, 0.02, 0.001, 0.5);
        self.hold_ms = self.hold_ms.min(1000);
        self
    }
}

#[derive(Clone, Debug)]
pub struct VoicePacket {
    pub seq: u32,
    pub data: Vec<u8>,
}
struct Captured {
    at: Instant,
    pcm: [i16; FRAME_SAMPLES],
}
struct Playback {
    peer: u64,
    pcm: [i16; FRAME_SAMPLES],
    gain: f32,
    at: Instant,
}
#[derive(Default)]
struct CaptureState {
    ready: bool,
    error: Option<String>,
    frames: VecDeque<Captured>,
}
struct Capture {
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<CaptureState>>,
    worker: Option<JoinHandle<()>>,
}
impl Capture {
    fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let state = Arc::new(Mutex::new(CaptureState::default()));
        let (signal, output) = (stop.clone(), state.clone());
        let worker = std::thread::Builder::new()
            .name("voice-capture".into())
            .spawn(move || {
                let result = capture_backend::run(&signal, &output);
                if let Ok(mut s) = output.lock() {
                    s.ready = false;
                    if !signal.load(Ordering::Acquire) {
                        if let Err(error) = result {
                            s.error = Some(error);
                        }
                    }
                }
            });
        let worker = match worker {
            Ok(worker) => Some(worker),
            Err(error) => {
                if let Ok(mut s) = state.lock() {
                    s.error = Some(error.to_string());
                }
                None
            }
        };
        Self {
            stop,
            state,
            worker,
        }
    }
    fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(mut state) = self.state.lock() {
            state.frames.clear();
        }
    }
    fn finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
        // Device APIs execute on their owning worker. Never block the UI on
        // driver shutdown; the worker observes cancellation and releases it.
        if let Some(worker) = self.worker.take() {
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
    }
}
fn captured(state: &Mutex<CaptureState>, stop: &AtomicBool, pcm: [i16; FRAME_SAMPLES]) {
    if stop.load(Ordering::Acquire) {
        return;
    }
    if let Ok(mut state) = state.lock() {
        if stop.load(Ordering::Acquire) {
            return;
        }
        if state.frames.len() == MAX_QUEUE {
            state.frames.pop_front();
        }
        state.frames.push_back(Captured {
            at: Instant::now(),
            pcm,
        });
    }
}
fn ready(state: &Mutex<CaptureState>) {
    if let Ok(mut state) = state.lock() {
        state.ready = true;
    }
}
fn newer(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) > 0
}
fn level(pcm: &[i16]) -> f32 {
    (pcm.iter()
        .map(|&x| (x as f32 / 32768.0).powi(2))
        .sum::<f32>()
        / pcm.len().max(1) as f32)
        .sqrt()
}

struct Peer {
    muted: bool,
    gain: f32,
    frames: BTreeMap<u32, [i16; FRAME_SAMPLES]>,
    expected: Option<u32>,
    delivered: Option<u32>,
    due: Option<Instant>,
    last_frame: Option<[i16; FRAME_SAMPLES]>,
    missing: u32,
    talking_until: Option<Instant>,
}
impl Default for Peer {
    fn default() -> Self {
        Self {
            muted: false,
            gain: 1.0,
            frames: BTreeMap::new(),
            expected: None,
            delivered: None,
            due: None,
            last_frame: None,
            missing: 0,
            talking_until: None,
        }
    }
}
impl Peer {
    fn clear(&mut self) {
        self.frames.clear();
        self.expected = None;
        self.due = None;
        self.last_frame = None;
        self.missing = 0;
        self.talking_until = None;
    }
    fn receive(&mut self, seq: u32, pcm: [i16; FRAME_SAMPLES], now: Instant) -> bool {
        if self.delivered.is_some_and(|last| !newer(seq, last)) || self.frames.contains_key(&seq) {
            return false;
        }
        if self.muted {
            self.delivered = Some(seq);
            return false;
        }
        match self.expected {
            None => {
                self.expected = Some(seq);
                self.due = Some(now + JITTER_DELAY);
            }
            Some(expected) => {
                let ahead = seq.wrapping_sub(expected);
                if newer(expected, seq) {
                    // Before playback starts a slightly earlier UDP packet
                    // may still extend the initial jitter window.
                    if self.delivered.is_none() && expected.wrapping_sub(seq) < JITTER_LIMIT {
                        self.expected = Some(seq);
                    } else {
                        return false;
                    }
                } else if ahead >= JITTER_LIMIT {
                    if ahead > 100 {
                        return false;
                    }
                    // Resume at fresh speech rather than replay a stale backlog.
                    self.clear();
                    self.expected = Some(seq);
                    self.due = Some(now + JITTER_DELAY);
                }
            }
        }
        if self.frames.len() >= JITTER_LIMIT as usize {
            return false;
        }
        self.frames.insert(seq, pcm);
        if level(&pcm) > 0.004 {
            self.talking_until = Some(now + Duration::from_millis(200));
        }
        true
    }
    fn playout(&mut self, now: Instant) -> Vec<[i16; FRAME_SAMPLES]> {
        let mut result = Vec::new();
        let Some(mut due) = self.due else {
            return result;
        };
        if now.saturating_duration_since(due) > Duration::from_millis(200) {
            // A suspended UI must not turn the jitter buffer into a recording.
            let fresh = self.expected.and_then(|base| {
                self.frames
                    .keys()
                    .copied()
                    .max_by_key(|seq| seq.wrapping_sub(base))
            });
            self.expected = fresh;
            if let Some(seq) = fresh {
                self.frames.retain(|&s, _| s == seq);
            }
            due = now;
        }
        while due <= now && result.len() < 3 {
            let Some(seq) = self.expected else {
                self.clear();
                break;
            };
            let pcm = if let Some(pcm) = self.frames.remove(&seq) {
                self.missing = 0;
                self.last_frame = Some(pcm);
                pcm
            } else {
                self.missing += 1;
                if self.missing > 2 {
                    self.clear();
                    break;
                }
                let attenuation = 0.35f32.powi(self.missing as i32);
                self.last_frame
                    .unwrap_or([0; FRAME_SAMPLES])
                    .map(|x| (x as f32 * attenuation) as i16)
            };
            self.delivered = Some(seq);
            self.expected = Some(seq.wrapping_add(1));
            result.push(pcm);
            due += FRAME_TIME;
            self.due = Some(due);
        }
        result
    }
}

pub struct VoiceChat {
    settings: VoiceSettings,
    capture: Option<Capture>,
    retry_at: Instant,
    error: Option<String>,
    outgoing: VecDeque<(Instant, VoicePacket)>,
    playback_queue: VecDeque<Playback>,
    peers: BTreeMap<u64, Peer>,
    clear_peers: BTreeSet<u64>,
    clear_all: bool,
    sequence: u32,
    hold_frames: u32,
    preroll: VecDeque<[i16; FRAME_SAMPLES]>,
    transmitting: bool,
    input_level: f32,
    last_input: Option<Instant>,
}
impl Default for VoiceChat {
    fn default() -> Self {
        Self::new()
    }
}
impl VoiceChat {
    pub fn new() -> Self {
        Self {
            settings: VoiceSettings::default(),
            capture: None,
            retry_at: Instant::now(),
            error: None,
            outgoing: VecDeque::new(),
            playback_queue: VecDeque::new(),
            peers: BTreeMap::new(),
            clear_peers: BTreeSet::new(),
            clear_all: false,
            sequence: 0,
            hold_frames: 0,
            preroll: VecDeque::new(),
            transmitting: false,
            input_level: 0.0,
            last_input: None,
        }
    }
    pub fn configure(&mut self, settings: VoiceSettings) {
        let settings = settings.sanitise();
        if self.settings == settings {
            return;
        }
        let previous = self.settings;
        let input_changed = previous.mode != settings.mode
            || previous.muted != settings.muted
            || previous.deafened != settings.deafened
            || previous.input_gain != settings.input_gain
            || previous.activation_threshold != settings.activation_threshold
            || previous.hold_ms != settings.hold_ms;
        let output_changed =
            previous.output_gain != settings.output_gain || previous.deafened != settings.deafened;
        self.settings = settings;
        if input_changed {
            if let Some(capture) = &self.capture {
                capture.stop();
            }
            self.error = None;
            self.retry_at = Instant::now();
            self.reset_input();
        }
        if output_changed {
            self.clear_all = true;
            self.playback_queue.clear();
            for peer in self.peers.values_mut() {
                peer.clear();
            }
        }
    }
    /// Call when leaving multiplayer. A new session requires opting in again.
    pub fn disconnect(&mut self) {
        if let Some(capture) = &self.capture {
            capture.stop();
        }
        self.settings.mode = VoiceMode::Disabled;
        self.reset_input();
        self.peers.clear();
        self.playback_queue.clear();
        self.clear_peers.clear();
        self.clear_all = true;
    }
    pub fn settings(&self) -> VoiceSettings {
        self.settings
    }
    pub fn capture_error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn capturing(&self) -> bool {
        self.capture.as_ref().is_some_and(|c| {
            !c.stop.load(Ordering::Acquire) && c.state.lock().is_ok_and(|s| s.ready)
        })
    }
    pub fn transmitting(&self) -> bool {
        self.transmitting
    }
    pub fn input_level(&self) -> f32 {
        self.input_level
    }
    fn reset_input(&mut self) {
        self.outgoing.clear();
        self.preroll.clear();
        self.hold_frames = 0;
        self.transmitting = false;
        self.input_level = 0.0;
        self.last_input = None;
    }
    /// Call each frame. PTT only opens the device while held. Activation mode
    /// opens it while focused, after explicit configuration by the user.
    pub fn tick(&mut self, push_to_talk: bool, focused: bool) {
        let now = Instant::now();
        let permitted = focused
            && !self.settings.muted
            && !self.settings.deafened
            && match self.settings.mode {
                VoiceMode::Disabled => false,
                VoiceMode::PushToTalk => push_to_talk,
                VoiceMode::VoiceActivation => true,
            };
        if !permitted {
            if let Some(capture) = &self.capture {
                capture.stop();
            }
            self.reset_input();
        }
        let finished = self.capture.as_ref().is_some_and(Capture::finished);
        if finished {
            if let Some(capture) = self.capture.take() {
                if let Ok(state) = capture.state.lock() {
                    if let Some(error) = &state.error {
                        self.error = Some(error.clone());
                        self.retry_at = now + Duration::from_secs(5);
                    }
                }
            }
        }
        if permitted && self.capture.is_none() && now >= self.retry_at {
            self.error = None;
            self.capture = Some(Capture::start());
        }
        let mut frames = Vec::new();
        if permitted {
            if let Some(capture) = &self.capture {
                if !capture.stop.load(Ordering::Acquire) {
                    if let Ok(mut state) = capture.state.lock() {
                        frames.extend(state.frames.drain(..));
                    }
                }
            }
        }
        for frame in frames {
            if now.saturating_duration_since(frame.at) < Duration::from_millis(180) {
                self.process_frame(frame.pcm, push_to_talk, now);
            }
        }
        if self
            .last_input
            .is_none_or(|last| now.saturating_duration_since(last) > Duration::from_millis(120))
        {
            self.transmitting = false;
            self.input_level = 0.0;
        }
        if !self.settings.deafened {
            for (&id, peer) in &mut self.peers {
                if peer.muted {
                    continue;
                }
                for pcm in peer.playout(now) {
                    if self.playback_queue.len() == MAX_PEERS * 3 {
                        self.playback_queue.pop_front();
                    }
                    self.playback_queue.push_back(Playback {
                        peer: id,
                        pcm,
                        gain: peer.gain * self.settings.output_gain,
                        at: now,
                    });
                }
            }
        }
    }
    fn process_frame(&mut self, mut pcm: [i16; FRAME_SAMPLES], ptt: bool, now: Instant) {
        if self.settings.muted
            || self.settings.deafened
            || self.settings.mode == VoiceMode::Disabled
        {
            return;
        }
        for x in &mut pcm {
            *x = (*x as f32 * self.settings.input_gain).clamp(-32767.0, 32767.0) as i16;
        }
        self.input_level = level(&pcm);
        self.last_input = Some(now);
        let was_open = self.hold_frames > 0;
        let send = match self.settings.mode {
            VoiceMode::Disabled => false,
            VoiceMode::PushToTalk => ptt,
            VoiceMode::VoiceActivation => {
                if self.input_level >= self.settings.activation_threshold {
                    self.hold_frames = self.settings.hold_ms.div_ceil(20).max(1);
                    true
                } else if self.hold_frames > 0 {
                    self.hold_frames -= 1;
                    true
                } else {
                    false
                }
            }
        };
        self.transmitting = send;
        if send {
            if self.settings.mode == VoiceMode::VoiceActivation && !was_open {
                while let Some(pcm) = self.preroll.pop_front() {
                    self.queue_packet(&pcm);
                }
            }
            self.queue_packet(&pcm);
        } else if self.settings.mode == VoiceMode::VoiceActivation {
            if self.preroll.len() == 2 {
                self.preroll.pop_front();
            }
            self.preroll.push_back(pcm);
        }
    }
    fn queue_packet(&mut self, pcm: &[i16; FRAME_SAMPLES]) {
        if self.outgoing.len() == MAX_QUEUE {
            self.outgoing.pop_front();
        }
        self.outgoing.push_back((
            Instant::now(),
            VoicePacket {
                seq: self.sequence,
                data: encode(pcm).to_vec(),
            },
        ));
        self.sequence = self.sequence.wrapping_add(1);
    }
    pub fn drain_outgoing(&mut self) -> Vec<VoicePacket> {
        let now = Instant::now();
        self.outgoing
            .drain(..)
            .filter_map(|(at, packet)| {
                (now.saturating_duration_since(at) < Duration::from_millis(180)).then_some(packet)
            })
            .collect()
    }
    /// The transport must authenticate peer_id against the session roster.
    pub fn receive(&mut self, peer_id: u64, seq: u32, data: &[u8]) -> bool {
        let Some(pcm) = decode(data) else {
            return false;
        };
        if self.settings.deafened {
            if let Some(peer) = self.peers.get_mut(&peer_id) {
                if peer.delivered.is_none_or(|last| newer(seq, last)) {
                    peer.delivered = Some(seq);
                }
            }
            return false;
        }
        if !self.peers.contains_key(&peer_id) && self.peers.len() >= MAX_PEERS {
            return false;
        }
        self.peers
            .entry(peer_id)
            .or_default()
            .receive(seq, pcm, Instant::now())
    }
    fn mark_clear(&mut self, peer_id: u64) {
        if self.clear_peers.len() >= MAX_PEERS {
            self.clear_all = true;
            self.clear_peers.clear();
        }
        self.clear_peers.insert(peer_id);
    }
    pub fn set_peer_muted(&mut self, peer_id: u64, muted: bool) {
        if !self.peers.contains_key(&peer_id) && self.peers.len() >= MAX_PEERS {
            return;
        }
        let peer = self.peers.entry(peer_id).or_default();
        if peer.muted == muted {
            return;
        }
        peer.muted = muted;
        peer.clear();
        self.mark_clear(peer_id);
        self.playback_queue.retain(|frame| frame.peer != peer_id);
    }
    pub fn set_peer_gain(&mut self, peer_id: u64, gain: f32) {
        if !self.peers.contains_key(&peer_id) && self.peers.len() >= MAX_PEERS {
            return;
        }
        let gain = finite(gain, 1.0, 0.0, 2.0);
        let peer = self.peers.entry(peer_id).or_default();
        if peer.gain == gain {
            return;
        }
        peer.gain = gain;
        self.mark_clear(peer_id);
        self.playback_queue.retain(|frame| frame.peer != peer_id);
    }
    pub fn remove_peer(&mut self, peer_id: u64) {
        if self.peers.remove(&peer_id).is_some() {
            self.mark_clear(peer_id);
        }
        self.playback_queue.retain(|frame| frame.peer != peer_id);
    }
    pub fn peer_talking(&self, peer_id: u64) -> bool {
        !self.settings.deafened
            && self.peers.get(&peer_id).is_some_and(|p| {
                !p.muted && p.talking_until.is_some_and(|until| Instant::now() < until)
            })
    }
    /// Call after tick(), also after configure/mute/deafen, to immediately
    /// discard buffered output and submit only fresh, ordered voice frames.
    pub fn playback(&mut self, audio: &crate::audio::Audio) {
        if self.clear_all {
            audio.voice_clear(None);
            self.clear_all = false;
        }
        for id in std::mem::take(&mut self.clear_peers) {
            audio.voice_clear(Some(id));
        }
        let now = Instant::now();
        for frame in self.playback_queue.drain(..) {
            if !self.settings.deafened
                && now.saturating_duration_since(frame.at) < Duration::from_millis(120)
            {
                audio.voice_pcm(frame.peer, &frame.pcm, SAMPLE_RATE, frame.gain);
            }
        }
    }
}

// IMA ADPCM block: predictor i16 LE, step index u8, reserved zero, then
// 319 4-bit differences, low nibble first. Each packet resets its predictor,
// so losing/reordering one packet cannot corrupt a later block.
const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const INDEX: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];
fn nibble(code: u8, predictor: &mut i32, index: &mut i32) -> i16 {
    let step = STEPS[*index as usize];
    let mut delta = step >> 3;
    if code & 4 != 0 {
        delta += step;
    }
    if code & 2 != 0 {
        delta += step >> 1;
    }
    if code & 1 != 0 {
        delta += step >> 2;
    }
    *predictor = (*predictor + if code & 8 != 0 { -delta } else { delta }).clamp(-32768, 32767);
    *index = (*index + INDEX[(code & 7) as usize]).clamp(0, 88);
    *predictor as i16
}
fn encode(pcm: &[i16; FRAME_SAMPLES]) -> [u8; PACKET_BYTES] {
    let mut out = [0u8; PACKET_BYTES];
    out[..2].copy_from_slice(&pcm[0].to_le_bytes());
    // A useful initial step avoids the large transient of starting each
    // independent block with the smallest step in the table.
    let slope = pcm
        .windows(2)
        .take(32)
        .map(|p| (p[1] as i32 - p[0] as i32).abs())
        .sum::<i32>()
        / 32;
    let mut index = STEPS.iter().position(|&s| s >= slope).unwrap_or(88) as i32;
    out[2] = index as u8;
    let mut predictor = pcm[0] as i32;
    for (i, &sample) in pcm.iter().enumerate().skip(1) {
        let mut diff = sample as i32 - predictor;
        let mut code = if diff < 0 {
            diff = -diff;
            8
        } else {
            0
        };
        let step = STEPS[index as usize];
        if diff >= step {
            code |= 4;
            diff -= step;
        }
        if diff >= step >> 1 {
            code |= 2;
            diff -= step >> 1;
        }
        if diff >= step >> 2 {
            code |= 1;
        }
        nibble(code, &mut predictor, &mut index);
        let offset = i - 1;
        out[4 + offset / 2] |= code << ((offset % 2) * 4);
    }
    out
}
fn decode(data: &[u8]) -> Option<[i16; FRAME_SAMPLES]> {
    if data.len() != PACKET_BYTES
        || data[2] > 88
        || data[3] != 0
        || data[PACKET_BYTES - 1] & 0xf0 != 0
    {
        return None;
    }
    let mut pcm = [0i16; FRAME_SAMPLES];
    let mut predictor = i16::from_le_bytes([data[0], data[1]]) as i32;
    let mut index = data[2] as i32;
    pcm[0] = predictor as i16;
    for (i, sample) in pcm.iter_mut().enumerate().skip(1) {
        let offset = i - 1;
        *sample = nibble(
            (data[4 + offset / 2] >> ((offset % 2) * 4)) & 15,
            &mut predictor,
            &mut index,
        );
    }
    Some(pcm)
}

#[cfg(windows)]
mod capture_backend {
    use super::*;
    use std::ffi::c_void;
    #[repr(C)]
    struct Format {
        tag: u16,
        channels: u16,
        rate: u32,
        bytes_sec: u32,
        align: u16,
        bits: u16,
        extra: u16,
    }
    #[repr(C)]
    struct Header {
        data: *mut i8,
        length: u32,
        recorded: u32,
        user: usize,
        flags: u32,
        loops: u32,
        next: *mut Header,
        reserved: usize,
    }
    #[link(name = "winmm")]
    extern "system" {
        fn waveInOpen(
            handle: *mut *mut c_void,
            device: u32,
            format: *const Format,
            callback: usize,
            user: usize,
            flags: u32,
        ) -> u32;
        fn waveInPrepareHeader(handle: *mut c_void, header: *mut Header, size: u32) -> u32;
        fn waveInUnprepareHeader(handle: *mut c_void, header: *mut Header, size: u32) -> u32;
        fn waveInAddBuffer(handle: *mut c_void, header: *mut Header, size: u32) -> u32;
        fn waveInStart(handle: *mut c_void) -> u32;
        fn waveInStop(handle: *mut c_void) -> u32;
        fn waveInReset(handle: *mut c_void) -> u32;
        fn waveInClose(handle: *mut c_void) -> u32;
    }
    struct Input {
        handle: *mut c_void,
        buffers: Vec<Box<[i16; FRAME_SAMPLES]>>,
        headers: Vec<Header>,
        prepared: usize,
    }
    impl Drop for Input {
        fn drop(&mut self) {
            unsafe {
                if self.handle.is_null() {
                    return;
                }
                waveInStop(self.handle);
                let mut released = waveInReset(self.handle) == 0;
                for header in self.headers.iter_mut().take(self.prepared) {
                    if waveInUnprepareHeader(
                        self.handle,
                        header,
                        std::mem::size_of::<Header>() as u32,
                    ) != 0
                    {
                        released = false;
                    }
                }
                if waveInClose(self.handle) != 0 {
                    released = false;
                }
                if !released {
                    // A broken driver must not keep pointers into freed memory.
                    std::mem::forget(std::mem::take(&mut self.buffers));
                    std::mem::forget(std::mem::take(&mut self.headers));
                }
            }
        }
    }
    pub fn run(stop: &AtomicBool, state: &Mutex<CaptureState>) -> Result<(), String> {
        unsafe {
            if stop.load(Ordering::Acquire) {
                return Ok(());
            }
            let mut input = Input {
                handle: std::ptr::null_mut(),
                buffers: Vec::new(),
                headers: Vec::new(),
                prepared: 0,
            };
            let format = Format {
                tag: 1,
                channels: 1,
                rate: SAMPLE_RATE,
                bytes_sec: SAMPLE_RATE * 2,
                align: 2,
                bits: 16,
                extra: 0,
            };
            let error = waveInOpen(&mut input.handle, u32::MAX, &format, 0, 0, 0);
            if error != 0 {
                return Err(format!("Microphone unavailable (waveInOpen {error}); check Windows microphone permission/default input"));
            }
            if stop.load(Ordering::Acquire) {
                return Ok(());
            }
            input.buffers = (0..4).map(|_| Box::new([0; FRAME_SAMPLES])).collect();
            input.headers = input
                .buffers
                .iter_mut()
                .map(|buffer| Header {
                    data: buffer.as_mut_ptr() as *mut i8,
                    length: (FRAME_SAMPLES * 2) as u32,
                    recorded: 0,
                    user: 0,
                    flags: 0,
                    loops: 0,
                    next: std::ptr::null_mut(),
                    reserved: 0,
                })
                .collect();
            let size = std::mem::size_of::<Header>() as u32;
            for index in 0..input.headers.len() {
                let error = waveInPrepareHeader(input.handle, &mut input.headers[index], size);
                if error != 0 {
                    return Err(format!("Microphone prepare failed ({error})"));
                }
                input.prepared += 1;
                let error = waveInAddBuffer(input.handle, &mut input.headers[index], size);
                if error != 0 {
                    return Err(format!("Microphone buffer failed ({error})"));
                }
            }
            let error = waveInStart(input.handle);
            if error != 0 {
                return Err(format!("Microphone start failed ({error})"));
            }
            ready(state);
            while !stop.load(Ordering::Acquire) {
                for index in 0..input.headers.len() {
                    if std::ptr::read_volatile(&input.headers[index].flags) & 1 == 0 {
                        continue;
                    }
                    std::sync::atomic::fence(Ordering::Acquire);
                    if input.headers[index].recorded as usize == FRAME_SAMPLES * 2 {
                        captured(state, stop, *input.buffers[index]);
                    }
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    input.headers[index].flags &= !1;
                    input.headers[index].recorded = 0;
                    let error = waveInAddBuffer(input.handle, &mut input.headers[index], size);
                    if error != 0 {
                        return Err(format!("Microphone requeue failed ({error})"));
                    }
                }
                std::thread::sleep(Duration::from_millis(3));
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
mod capture_backend {
    use super::*;
    use std::ffi::{c_char, c_int, c_void, CString};
    struct Library(*mut c_void);
    impl Library {
        unsafe fn open(names: &[&str]) -> Option<Self> {
            for name in names {
                let name = CString::new(*name).ok()?;
                let pointer = libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
                if !pointer.is_null() {
                    return Some(Self(pointer));
                }
            }
            None
        }
        unsafe fn symbol<T: Copy>(&self, name: &str) -> Result<T, String> {
            let symbol = CString::new(name).map_err(|_| "Invalid audio symbol".to_owned())?;
            let pointer = libc::dlsym(self.0, symbol.as_ptr());
            if pointer.is_null() {
                return Err(format!("Missing audio function {name}"));
            }
            Ok(std::mem::transmute_copy(&pointer))
        }
    }
    impl Drop for Library {
        fn drop(&mut self) {
            unsafe {
                libc::dlclose(self.0);
            }
        }
    }
    type CreateLoop = unsafe extern "C" fn() -> *mut c_void;
    type LoopApi = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
    type Iterate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> c_int;
    type ContextNew = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
    type ContextConnect =
        unsafe extern "C" fn(*mut c_void, *const c_char, c_int, *const c_void) -> c_int;
    type GetState = unsafe extern "C" fn(*const c_void) -> c_int;
    type Release = unsafe extern "C" fn(*mut c_void);
    #[repr(C)]
    struct SampleSpec {
        format: c_int,
        rate: u32,
        channels: u8,
    }
    #[repr(C)]
    struct BufferAttr {
        maxlength: u32,
        tlength: u32,
        prebuf: u32,
        minreq: u32,
        fragsize: u32,
    }
    type StreamNew = unsafe extern "C" fn(
        *mut c_void,
        *const c_char,
        *const SampleSpec,
        *const c_void,
    ) -> *mut c_void;
    type ConnectRecord =
        unsafe extern "C" fn(*mut c_void, *const c_char, *const BufferAttr, c_int) -> c_int;
    type Peek = unsafe extern "C" fn(*mut c_void, *mut *const c_void, *mut usize) -> c_int;
    type StreamAction = unsafe extern "C" fn(*mut c_void) -> c_int;
    struct Pulse {
        _library: Library,
        mainloop: *mut c_void,
        context: *mut c_void,
        stream: *mut c_void,
        iterate: Iterate,
        context_state: GetState,
        stream_state: GetState,
        context_disconnect: Release,
        context_unref: Release,
        loop_free: Release,
        stream_disconnect: StreamAction,
        stream_unref: Release,
        peek: Peek,
        drop_data: StreamAction,
    }
    impl Drop for Pulse {
        fn drop(&mut self) {
            unsafe {
                if !self.stream.is_null() {
                    (self.stream_disconnect)(self.stream);
                    (self.stream_unref)(self.stream);
                }
                if !self.context.is_null() {
                    (self.context_disconnect)(self.context);
                    (self.context_unref)(self.context);
                }
                if !self.mainloop.is_null() {
                    (self.loop_free)(self.mainloop);
                }
            }
        }
    }
    impl Pulse {
        unsafe fn open(stop: &AtomicBool) -> Result<Self, String> {
            let library = Library::open(&["libpulse.so.0", "libpulse.so"])
                .ok_or("PulseAudio library unavailable")?;
            let create: CreateLoop = library.symbol("pa_mainloop_new")?;
            let api: LoopApi = library.symbol("pa_mainloop_get_api")?;
            let new_context: ContextNew = library.symbol("pa_context_new")?;
            let connect: ContextConnect = library.symbol("pa_context_connect")?;
            let stream_new: StreamNew = library.symbol("pa_stream_new")?;
            let record: ConnectRecord = library.symbol("pa_stream_connect_record")?;
            let mut session = Self {
                mainloop: std::ptr::null_mut(),
                context: std::ptr::null_mut(),
                stream: std::ptr::null_mut(),
                iterate: library.symbol("pa_mainloop_iterate")?,
                context_state: library.symbol("pa_context_get_state")?,
                stream_state: library.symbol("pa_stream_get_state")?,
                context_disconnect: library.symbol("pa_context_disconnect")?,
                context_unref: library.symbol("pa_context_unref")?,
                loop_free: library.symbol("pa_mainloop_free")?,
                stream_disconnect: library.symbol("pa_stream_disconnect")?,
                stream_unref: library.symbol("pa_stream_unref")?,
                peek: library.symbol("pa_stream_peek")?,
                drop_data: library.symbol("pa_stream_drop")?,
                _library: library,
            };
            session.mainloop = create();
            if session.mainloop.is_null() {
                return Err("PulseAudio mainloop unavailable".into());
            }
            let name = CString::new("Viper Run voice").unwrap();
            session.context = new_context(api(session.mainloop), name.as_ptr());
            if session.context.is_null() {
                return Err("PulseAudio context unavailable".into());
            }
            // NOAUTOSPAWN: use the user's existing audio server.
            if connect(session.context, std::ptr::null(), 1, std::ptr::null()) < 0 {
                return Err("PulseAudio connection failed".into());
            }
            session.wait(stop, session.context, session.context_state, 4, 5)?;
            if stop.load(Ordering::Acquire) {
                return Err("Capture cancelled".into());
            }
            let spec = SampleSpec {
                format: 3,
                rate: SAMPLE_RATE,
                channels: 1,
            };
            session.stream = stream_new(session.context, name.as_ptr(), &spec, std::ptr::null());
            if session.stream.is_null() {
                return Err("PulseAudio record stream unavailable".into());
            }
            let attr = BufferAttr {
                maxlength: (FRAME_SAMPLES * 2 * MAX_QUEUE) as u32,
                tlength: u32::MAX,
                prebuf: u32::MAX,
                minreq: u32::MAX,
                fragsize: (FRAME_SAMPLES * 2) as u32,
            };
            if record(session.stream, std::ptr::null(), &attr, 0) < 0 {
                return Err("PulseAudio microphone connection failed".into());
            }
            session.wait(stop, session.stream, session.stream_state, 2, 3)?;
            Ok(session)
        }
        unsafe fn pump(&self) -> Result<(), String> {
            let mut result = 0;
            for _ in 0..4 {
                if (self.iterate)(self.mainloop, 0, &mut result) < 0 {
                    return Err("PulseAudio disconnected".into());
                }
            }
            Ok(())
        }
        unsafe fn wait(
            &self,
            stop: &AtomicBool,
            object: *mut c_void,
            state: GetState,
            wanted: c_int,
            failed: c_int,
        ) -> Result<(), String> {
            let deadline = Instant::now() + Duration::from_secs(3);
            while !stop.load(Ordering::Acquire) && Instant::now() < deadline {
                self.pump()?;
                let value = state(object);
                if value == wanted {
                    return Ok(());
                }
                if value >= failed {
                    return Err("PulseAudio microphone permission/device unavailable".into());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err("PulseAudio connection cancelled or timed out".into())
        }
        unsafe fn record(
            &self,
            stop: &AtomicBool,
            state: &Mutex<CaptureState>,
        ) -> Result<(), String> {
            let mut pcm = [0i16; FRAME_SAMPLES];
            let mut used = 0;
            ready(state);
            while !stop.load(Ordering::Acquire) {
                self.pump()?;
                if (self.stream_state)(self.stream) != 2 {
                    return Err("PulseAudio microphone disconnected".into());
                }
                let mut pointer = std::ptr::null();
                let mut bytes = 0;
                if (self.peek)(self.stream, &mut pointer, &mut bytes) < 0 {
                    return Err("PulseAudio microphone read failed".into());
                }
                if bytes > 0 {
                    if bytes <= FRAME_SAMPLES * 2 * MAX_QUEUE && bytes % 2 == 0 {
                        let data = if pointer.is_null() {
                            None
                        } else {
                            Some(std::slice::from_raw_parts(pointer as *const u8, bytes))
                        };
                        for index in 0..bytes / 2 {
                            pcm[used] = data.map_or(0, |d| {
                                i16::from_le_bytes([d[index * 2], d[index * 2 + 1]])
                            });
                            used += 1;
                            if used == FRAME_SAMPLES {
                                captured(state, stop, pcm);
                                used = 0;
                            }
                        }
                    } else {
                        used = 0;
                    }
                    if (self.drop_data)(self.stream) < 0 {
                        return Err("PulseAudio microphone stream failed".into());
                    }
                }
                std::thread::sleep(Duration::from_millis(3));
            }
            Ok(())
        }
    }
    type AlsaOpen = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
    type AlsaSet = unsafe extern "C" fn(*mut c_void, c_int, c_int, u32, u32, c_int, u32) -> c_int;
    type AlsaRead = unsafe extern "C" fn(*mut c_void, *mut c_void, libc::c_ulong) -> libc::c_long;
    type AlsaRecover = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
    type AlsaAction = unsafe extern "C" fn(*mut c_void) -> c_int;
    struct Alsa {
        _library: Library,
        pcm: *mut c_void,
        drop_pcm: AlsaAction,
        close: AlsaAction,
    }
    impl Drop for Alsa {
        fn drop(&mut self) {
            unsafe {
                if !self.pcm.is_null() {
                    (self.drop_pcm)(self.pcm);
                    (self.close)(self.pcm);
                }
            }
        }
    }
    unsafe fn alsa(stop: &AtomicBool, state: &Mutex<CaptureState>) -> Result<(), String> {
        let library =
            Library::open(&["libasound.so.2", "libasound.so"]).ok_or("ALSA library unavailable")?;
        let open: AlsaOpen = library.symbol("snd_pcm_open")?;
        let set: AlsaSet = library.symbol("snd_pcm_set_params")?;
        let read: AlsaRead = library.symbol("snd_pcm_readi")?;
        let recover: AlsaRecover = library.symbol("snd_pcm_recover")?;
        let mut input = Alsa {
            pcm: std::ptr::null_mut(),
            drop_pcm: library.symbol("snd_pcm_drop")?,
            close: library.symbol("snd_pcm_close")?,
            _library: library,
        };
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        let name = CString::new("default").unwrap();
        // CAPTURE, NONBLOCK, S16_LE, RW_INTERLEAVED; server/hardware resamples.
        if open(&mut input.pcm, name.as_ptr(), 1, 1) < 0 || input.pcm.is_null() {
            return Err("ALSA default microphone unavailable".into());
        }
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if set(input.pcm, 2, 3, 1, SAMPLE_RATE, 1, 60000) < 0 {
            return Err("ALSA microphone does not support 16 kHz mono capture".into());
        }
        let mut pcm = [0i16; FRAME_SAMPLES];
        let mut used = 0;
        ready(state);
        while !stop.load(Ordering::Acquire) {
            let count = read(
                input.pcm,
                pcm.as_mut_ptr().add(used) as *mut c_void,
                (FRAME_SAMPLES - used) as libc::c_ulong,
            );
            if count > 0 {
                if count as usize > FRAME_SAMPLES - used {
                    return Err("Invalid ALSA frame count".into());
                }
                used += count as usize;
                if used == FRAME_SAMPLES {
                    captured(state, stop, pcm);
                    used = 0;
                }
            } else if count == -(libc::EAGAIN as libc::c_long) || count == 0 {
                std::thread::sleep(Duration::from_millis(3));
            } else {
                used = 0;
                // Recover overruns only. ALSA's suspend recovery can block
                // indefinitely, which would delay closing a muted microphone.
                if count != -(libc::EPIPE as libc::c_long)
                    || recover(input.pcm, count as c_int, 1) < 0
                {
                    return Err(format!("ALSA microphone read failed ({count})"));
                }
            }
        }
        Ok(())
    }
    pub fn run(stop: &AtomicBool, state: &Mutex<CaptureState>) -> Result<(), String> {
        unsafe {
            match Pulse::open(stop) {
                Ok(pulse) => pulse.record(stop, state),
                Err(_) if stop.load(Ordering::Acquire) => Ok(()),
                Err(pulse_error) => {
                    alsa(stop, state).map_err(|error| format!("{pulse_error}; {error}"))
                }
            }
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod capture_backend {
    use super::*;
    pub fn run(_: &AtomicBool, _: &Mutex<CaptureState>) -> Result<(), String> {
        Err("Microphone capture is supported on Windows and Linux".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone() -> [i16; FRAME_SAMPLES] {
        std::array::from_fn(|i| {
            ((i as f32 * 440.0 * std::f32::consts::TAU / SAMPLE_RATE as f32).sin() * 12000.0) as i16
        })
    }
    #[test]
    fn construction_and_disabled_ticks_never_create_a_capture_worker() {
        let mut voice = VoiceChat::new();
        assert_eq!(voice.settings().mode, VoiceMode::Disabled);
        voice.tick(true, true);
        assert!(voice.capture.is_none());
        assert!(!voice.capturing() && !voice.transmitting());
        assert!(voice.drain_outgoing().is_empty());
    }
    #[test]
    fn codec_blocks_are_independent_bounded_and_preserve_speech_band_waveforms() {
        let original = tone();
        let packet = encode(&original);
        let decoded = decode(&packet).unwrap();
        let mse = original
            .iter()
            .zip(decoded)
            .map(|(&a, b)| (a as f64 - b as f64).powi(2))
            .sum::<f64>()
            / FRAME_SAMPLES as f64;
        assert!(mse.sqrt() < 600.0, "ADPCM RMS error {}", mse.sqrt());
        assert_eq!(packet.len(), PACKET_BYTES);
        assert_eq!(decoded[0], original[0]);
        for value in [i16::MIN, 0, i16::MAX] {
            assert_eq!(
                decode(&encode(&[value; FRAME_SAMPLES])).unwrap(),
                [value; FRAME_SAMPLES]
            );
        }
        let mut broken = packet;
        broken[2] = 89;
        assert!(decode(&broken).is_none());
        broken = packet;
        broken[3] = 1;
        assert!(decode(&broken).is_none());
        broken = packet;
        broken[PACKET_BYTES - 1] |= 0xf0;
        assert!(decode(&broken).is_none());
        assert!(decode(&packet[..PACKET_BYTES - 1]).is_none());
        assert!(decode(&[0; PACKET_BYTES + 1]).is_none());
        // Missing one block leaves the next predictor and decoded samples intact.
        assert_eq!(decode(&encode(&original)).unwrap(), decoded);
    }
    #[test]
    fn ptt_release_mute_deafen_and_focus_loss_discard_pending_speech() {
        let mut voice = VoiceChat::new();
        voice.configure(VoiceSettings {
            mode: VoiceMode::PushToTalk,
            ..Default::default()
        });
        voice.process_frame(tone(), false, Instant::now());
        assert!(voice.drain_outgoing().is_empty());
        voice.process_frame(tone(), true, Instant::now());
        assert_eq!(voice.drain_outgoing().len(), 1);
        voice.process_frame(tone(), true, Instant::now());
        // Unfocused tick cannot open a real microphone in this test.
        voice.tick(true, false);
        assert!(voice.capture.is_none() && voice.drain_outgoing().is_empty());
        voice.configure(VoiceSettings {
            mode: VoiceMode::PushToTalk,
            muted: true,
            ..Default::default()
        });
        voice.process_frame(tone(), true, Instant::now());
        assert!(voice.drain_outgoing().is_empty());
        voice.configure(VoiceSettings {
            mode: VoiceMode::PushToTalk,
            deafened: true,
            ..Default::default()
        });
        voice.process_frame(tone(), true, Instant::now());
        assert!(voice.drain_outgoing().is_empty());
    }
    #[test]
    fn activation_has_two_frames_of_preroll_and_a_finite_hold() {
        let mut voice = VoiceChat::new();
        voice.configure(VoiceSettings {
            mode: VoiceMode::VoiceActivation,
            hold_ms: 40,
            ..Default::default()
        });
        let now = Instant::now();
        voice.process_frame([0; FRAME_SAMPLES], false, now);
        voice.process_frame([0; FRAME_SAMPLES], false, now);
        assert!(voice.drain_outgoing().is_empty());
        voice.process_frame(tone(), false, now);
        assert_eq!(voice.drain_outgoing().len(), 3);
        for _ in 0..3 {
            voice.process_frame([0; FRAME_SAMPLES], false, now);
        }
        assert_eq!(voice.drain_outgoing().len(), 2);
        assert!(!voice.transmitting());
        assert!(voice.capture.is_none());
    }
    #[test]
    fn outgoing_and_capture_queues_drop_stale_audio_and_never_grow() {
        let mut voice = VoiceChat::new();
        voice.configure(VoiceSettings {
            mode: VoiceMode::PushToTalk,
            ..Default::default()
        });
        for _ in 0..100 {
            voice.process_frame(tone(), true, Instant::now());
        }
        assert_eq!(voice.outgoing.len(), MAX_QUEUE);
        assert_eq!(voice.drain_outgoing().first().unwrap().seq, 92);
        voice.outgoing.push_back((
            Instant::now() - Duration::from_secs(1),
            VoicePacket {
                seq: 1,
                data: encode(&tone()).to_vec(),
            },
        ));
        assert!(voice.drain_outgoing().is_empty());
        let state = Mutex::new(CaptureState::default());
        let stop = AtomicBool::new(false);
        for _ in 0..100 {
            captured(&state, &stop, tone());
        }
        assert_eq!(state.lock().unwrap().frames.len(), MAX_QUEUE);
        stop.store(true, Ordering::Release);
        let before = state.lock().unwrap().frames.back().unwrap().at;
        captured(&state, &stop, tone());
        assert_eq!(state.lock().unwrap().frames.back().unwrap().at, before);
    }
    #[test]
    fn jitter_orders_packets_handles_wrap_and_rejects_replays() {
        let now = Instant::now();
        let mut peer = Peer::default();
        assert!(peer.receive(u32::MAX, [2; FRAME_SAMPLES], now));
        assert!(peer.receive(0, [3; FRAME_SAMPLES], now));
        assert!(peer.receive(u32::MAX - 1, [1; FRAME_SAMPLES], now));
        assert!(!peer.receive(0, [3; FRAME_SAMPLES], now));
        assert!(peer.playout(now + Duration::from_millis(59)).is_empty());
        assert_eq!(
            peer.playout(now + Duration::from_millis(60))[0],
            [1; FRAME_SAMPLES]
        );
        assert_eq!(
            peer.playout(now + Duration::from_millis(80))[0],
            [2; FRAME_SAMPLES]
        );
        assert_eq!(
            peer.playout(now + Duration::from_millis(100))[0],
            [3; FRAME_SAMPLES]
        );
        assert!(!peer.receive(u32::MAX, [2; FRAME_SAMPLES], now));
    }
    #[test]
    fn loss_concealment_decays_and_stalls_do_not_replay_a_backlog() {
        let now = Instant::now();
        let mut peer = Peer::default();
        peer.receive(0, [10000; FRAME_SAMPLES], now);
        assert_eq!(peer.playout(now + JITTER_DELAY)[0][0], 10000);
        assert_eq!(peer.playout(now + JITTER_DELAY + FRAME_TIME)[0][0], 3500);
        assert!(peer.playout(now + JITTER_DELAY + FRAME_TIME * 2)[0][0].abs() < 1500);
        assert!(peer.playout(now + JITTER_DELAY + FRAME_TIME * 3).is_empty());
        assert!(peer.due.is_none());
        peer.receive(4, [4000; FRAME_SAMPLES], now);
        peer.receive(5, [5000; FRAME_SAMPLES], now);
        assert_eq!(peer.playout(now + Duration::from_secs(1))[0][0], 5000);
    }
    #[test]
    fn mute_and_deafen_keep_a_replay_floor_and_disconnect_requires_opt_in_again() {
        let mut voice = VoiceChat::new();
        let packet = encode(&tone());
        assert!(voice.receive(7, 10, &packet));
        voice.set_peer_muted(7, true);
        assert!(!voice.receive(7, 11, &packet));
        voice.set_peer_muted(7, false);
        assert!(!voice.receive(7, 11, &packet));
        assert!(voice.receive(7, 12, &packet));
        voice.configure(VoiceSettings {
            deafened: true,
            ..Default::default()
        });
        assert!(!voice.receive(7, 13, &packet));
        voice.configure(VoiceSettings::default());
        assert!(!voice.receive(7, 13, &packet));
        assert!(voice.receive(7, 14, &packet));
        voice.disconnect();
        assert_eq!(voice.settings.mode, VoiceMode::Disabled);
        assert!(voice.peers.is_empty() && voice.playback_queue.is_empty());
        assert!(voice.clear_all);
    }
    #[test]
    fn receive_window_and_pending_clear_set_remain_bounded_under_churn() {
        let now = Instant::now();
        let mut peer = Peer::default();
        for seq in 0..1000 {
            peer.receive(seq, [1; FRAME_SAMPLES], now);
        }
        assert!(peer.frames.len() <= JITTER_LIMIT as usize);
        let mut voice = VoiceChat::new();
        for id in 0..1000 {
            voice.set_peer_muted(id, true);
            voice.remove_peer(id);
        }
        assert!(voice.clear_peers.len() <= MAX_PEERS && voice.clear_all);
    }
    #[test]
    fn peers_and_mute_cleanup_are_bounded_and_settings_are_sanitised() {
        let mut voice = VoiceChat::new();
        let packet = encode(&tone());
        for id in 0..MAX_PEERS as u64 {
            assert!(voice.receive(id, 0, &packet));
        }
        assert!(!voice.receive(100, 0, &packet));
        voice.set_peer_muted(0, true);
        assert!(!voice.peer_talking(0));
        assert!(!voice.receive(0, 1, &packet));
        voice.configure(VoiceSettings {
            deafened: true,
            input_gain: f32::NAN,
            activation_threshold: f32::INFINITY,
            hold_ms: u32::MAX,
            ..Default::default()
        });
        assert_eq!(voice.settings().input_gain, 1.0);
        assert_eq!(voice.settings().hold_ms, 1000);
        assert!(!voice.receive(1, 1, &packet));
        for id in 0..1000 {
            voice.remove_peer(id);
        }
        assert!(voice.clear_peers.len() <= MAX_PEERS);
        assert!(voice.peers.is_empty());
    }
}
