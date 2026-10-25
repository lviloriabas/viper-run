//! Frame history and OS hardware counters. Sampling runs off the render thread;
//! disabling the overlay drops the worker and its native query handles.
use crate::canvas::{rgba, Align, Canvas};
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

const HISTORY: usize = 180;

#[derive(Clone, Copy, Default, Debug)]
pub struct Hardware {
    pub cpu: Option<f64>,
    pub gpu: Option<f64>,
    /// System physical memory, GiB in use and total.
    pub ram: Option<(f64, f64)>,
}

struct Worker {
    rx: Receiver<Hardware>,
    stop: Sender<()>,
}

impl Worker {
    fn new() -> Self {
        let (tx, rx) = mpsc::sync_channel(1);
        let (stop, done) = mpsc::channel();
        std::thread::Builder::new().name("performance".into()).spawn(move || {
            let mut sampler = native::Sampler::new();
            loop {
                if tx.try_send(sampler.sample()).is_err_and(|e| matches!(e, mpsc::TrySendError::Disconnected(_))) {
                    break;
                }
                if !matches!(done.recv_timeout(Duration::from_millis(750)), Err(mpsc::RecvTimeoutError::Timeout)) {
                    break;
                }
            }
        }).ok();
        Self { rx, stop }
    }
}

impl Drop for Worker {
    fn drop(&mut self) { let _ = self.stop.send(()); }
}

pub struct Performance {
    history: VecDeque<f32>,
    last: Instant,
    pub frame: u64,
    hardware: Hardware,
    worker: Option<Worker>,
}

impl Performance {
    pub fn new() -> Self {
        Self { history: VecDeque::with_capacity(HISTORY), last: Instant::now(), frame: 0, hardware: Hardware::default(), worker: None }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled && self.worker.is_none() {
            self.worker = Some(Worker::new());
            self.history.clear();
            self.last = Instant::now();
            self.hardware = Hardware::default();
        } else if !enabled {
            self.worker = None;
        }
    }

    /// Called once per rendered frame. Instant is independent of simulation
    /// time, pause, fixed-step clamping and the benchmark's clock resets.
    pub fn presented(&mut self) {
        let now = Instant::now();
        let ms = now.duration_since(self.last).as_secs_f32() * 1000.0;
        self.last = now;
        self.frame += 1;
        if let Some(worker) = &self.worker {
            // The first interval includes startup, so it is not a frame sample.
            if self.frame > 1 && ms.is_finite() && ms > 0.0 {
                if self.history.len() == HISTORY { self.history.pop_front(); }
                self.history.push_back(ms);
            }
            while let Ok(value) = worker.rx.try_recv() { self.hardware = value; }
        }
    }

    pub fn draw(&self, cv: &mut Canvas, x: f32, y: f32, u: f32) {
        use crate::ui::{ACID, AMBER, CREAM, DIM, GREEN, LINE};
        let width = 158.0 * u;
        cv.rect(x - 4.0 * u, y - 5.0 * u, width + 8.0 * u, 103.0 * u, rgba(4, 10, 6, 0.66));
        cv.text("PERFORMANCE", x, y, u, ACID, Align::Left);
        let avg = self.history.iter().rev().take(60).copied().sum::<f32>() / self.history.len().min(60).max(1) as f32;
        let fps = if avg > 0.0 { 1000.0 / avg } else { 0.0 };
        cv.text(&format!("{fps:.0} FPS   {avg:.2} MS AVG"), x, y + 10.0 * u, u, CREAM, Align::Left);
        let peak_ms = self.history.iter().copied().fold(33.3f32, f32::max);
        let mut rates: Vec<f32> = self.history.iter().map(|v| 1000.0 / v).collect();
        rates.sort_by(f32::total_cmp);
        let peak_fps = rates.get(rates.len().saturating_sub(1) * 95 / 100).copied().unwrap_or(60.0).max(60.0) * 1.1;
        for (row, label, peak, color) in [(0, "FPS", peak_fps, GREEN), (1, "MS", peak_ms, AMBER)] {
            let gy = y + (21.0 + row as f32 * 23.0) * u;
            let gh = 18.0 * u;
            cv.rect(x, gy, width, gh, rgba(2, 6, 4, 0.6));
            cv.line(x, gy + gh, x + width, gy + gh, 1.0, LINE);
            cv.text(label, x + 2.0 * u, gy + 5.0 * u, u, DIM, Align::Left);
            let mut prev = None;
            for (i, &v) in self.history.iter().enumerate() {
                let value = if row == 0 { 1000.0 / v } else { v };
                let px = x + 30.0 * u + (HISTORY - self.history.len() + i) as f32 / (HISTORY - 1) as f32 * (width - 30.0 * u);
                let py = gy + gh - (value / peak).clamp(0.0, 1.0) * (gh - u);
                if let Some((ax, ay)) = prev { cv.line(ax, ay, px, py, u, color); }
                prev = Some((px, py));
            }
        }
        let percent = |v: Option<f64>| v.map(|n| format!("{n:.0}%")).unwrap_or_else(|| "N/A".into());
        cv.text(&format!("CPU {}  GPU {}", percent(self.hardware.cpu), percent(self.hardware.gpu)), x, y + 72.0 * u, u, CREAM, Align::Left);
        let ram = self.hardware.ram.map(|(used, total)| format!("RAM {:.0}% {used:.1}/{total:.1} GIB", 100.0 * used / total.max(0.001))).unwrap_or_else(|| "RAM N/A".into());
        cv.text(&ram, x, y + 82.0 * u, u, CREAM, Align::Left);
        cv.text("SYSTEM USAGE / LIVE", x, y + 92.0 * u, u, DIM, Align::Left);
    }
}

/// Kernel time includes idle on both supported platforms.
fn cpu_usage(previous: Option<(u64, u64)>, total: u64, idle: u64) -> Option<f64> {
    let (old_total, old_idle) = previous?;
    let elapsed = total.checked_sub(old_total)?;
    let idle = idle.checked_sub(old_idle)?;
    (elapsed > 0).then(|| 100.0 * elapsed.saturating_sub(idle) as f64 / elapsed as f64)
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::{collections::HashMap, ffi::c_void, ptr};
    type Handle = *mut c_void;
    #[repr(C)]
    #[derive(Default)]
    struct FileTime { low: u32, high: u32 }
    impl FileTime { fn value(&self) -> u64 { (self.high as u64) << 32 | self.low as u64 } }
    #[repr(C)]
    #[derive(Default)]
    struct Memory { length: u32, load: u32, total: u64, available: u64, page_total: u64, page_available: u64, virtual_total: u64, virtual_available: u64, extended: u64 }
    #[repr(C)]
    struct CounterValue { status: u32, value: f64 }
    #[repr(C)]
    struct CounterItem { name: *const u16, value: CounterValue }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
        fn GlobalMemoryStatusEx(memory: *mut Memory) -> i32;
    }
    #[link(name = "pdh")]
    extern "system" {
        fn PdhOpenQueryW(source: *const u16, user: usize, query: *mut Handle) -> u32;
        fn PdhAddEnglishCounterW(query: Handle, path: *const u16, user: usize, counter: *mut Handle) -> u32;
        fn PdhCollectQueryData(query: Handle) -> u32;
        fn PdhGetFormattedCounterArrayW(counter: Handle, format: u32, bytes: *mut u32, count: *mut u32, items: *mut CounterItem) -> u32;
        fn PdhCloseQuery(query: Handle) -> u32;
    }
    pub struct Sampler { previous: Option<(u64, u64)>, query: Handle, gpu: Handle }
    impl Sampler {
        pub fn new() -> Self {
            let mut s = Self { previous: None, query: ptr::null_mut(), gpu: ptr::null_mut() };
            unsafe {
                if PdhOpenQueryW(ptr::null(), 0, &mut s.query) == 0 {
                    let path: Vec<u16> = "\\GPU Engine(*)\\Utilization Percentage\0".encode_utf16().collect();
                    if PdhAddEnglishCounterW(s.query, path.as_ptr(), 0, &mut s.gpu) == 0 { PdhCollectQueryData(s.query); }
                }
            }
            s
        }
        fn gpu(&self) -> Option<f64> {
            if self.gpu.is_null() { return None; }
            unsafe {
                if PdhCollectQueryData(self.query) != 0 { return None; }
                let (mut bytes, mut count) = (0, 0);
                // PDH_FMT_DOUBLE | PDH_FMT_NOCAP100. Sum processes on each
                // physical engine, then use the busiest engine, like Task Manager.
                let format = 0x200 | 0x8000;
                if PdhGetFormattedCounterArrayW(self.gpu, format, &mut bytes, &mut count, ptr::null_mut()) != 0x800007d2 || bytes == 0 { return None; }
                // u64 gives the native structures their required 8-byte alignment.
                let mut buffer = vec![0u64; (bytes as usize).div_ceil(8)];
                if PdhGetFormattedCounterArrayW(self.gpu, format, &mut bytes, &mut count, buffer.as_mut_ptr().cast()) != 0 { return None; }
                if count as usize > bytes as usize / std::mem::size_of::<CounterItem>() { return None; }
                let mut engines = HashMap::<String, f64>::new();
                for item in std::slice::from_raw_parts(buffer.as_ptr().cast::<CounterItem>(), count as usize) {
                    if item.value.status > 1 || item.name.is_null() || !item.value.value.is_finite() { continue; }
                    let mut len = 0;
                    while *item.name.add(len) != 0 { len += 1; }
                    let name = String::from_utf16_lossy(std::slice::from_raw_parts(item.name, len));
                    if let Some((_, engine)) = name.split_once("_luid_") {
                        *engines.entry(engine.into()).or_default() += item.value.value.max(0.0);
                    }
                }
                engines.values().copied().reduce(f64::max).map(|v| v.clamp(0.0, 100.0))
            }
        }
        pub fn sample(&mut self) -> Hardware {
            let mut h = Hardware::default();
            unsafe {
                let (mut idle, mut kernel, mut user) = (FileTime::default(), FileTime::default(), FileTime::default());
                if GetSystemTimes(&mut idle, &mut kernel, &mut user) != 0 {
                    let (total, idle) = (kernel.value() + user.value(), idle.value());
                    h.cpu = cpu_usage(self.previous, total, idle);
                    self.previous = Some((total, idle));
                }
                let mut memory = Memory { length: std::mem::size_of::<Memory>() as u32, ..Default::default() };
                if GlobalMemoryStatusEx(&mut memory) != 0 {
                    h.ram = Some(((memory.total.saturating_sub(memory.available)) as f64 / 1073741824.0, memory.total as f64 / 1073741824.0));
                }
            }
            h.gpu = self.gpu();
            h
        }
    }
    impl Drop for Sampler { fn drop(&mut self) { if !self.query.is_null() { unsafe { PdhCloseQuery(self.query); } } } }
}

#[cfg(target_os = "linux")]
mod native {
    use super::*;
    use std::{fs, path::PathBuf};
    pub struct Sampler { previous: Option<(u64, u64)>, gpu_paths: Vec<PathBuf> }
    impl Sampler {
        pub fn new() -> Self {
            let gpu_paths = fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
                .filter(|d| { let n = d.file_name(); let n = n.to_string_lossy(); n.starts_with("card") && !n.contains('-') })
                .map(|d| d.path().join("device/gpu_busy_percent")).collect();
            Self { previous: None, gpu_paths }
        }
        pub fn sample(&mut self) -> Hardware {
            let mut h = Hardware::default();
            if let Ok(stat) = fs::read_to_string("/proc/stat") {
                let values: Vec<u64> = stat.lines().next().unwrap_or("").split_whitespace().skip(1).take(8).filter_map(|v| v.parse().ok()).collect();
                if values.len() >= 5 {
                    let total = values.iter().sum();
                    let idle = values[3] + values[4];
                    h.cpu = cpu_usage(self.previous, total, idle);
                    self.previous = Some((total, idle));
                }
            }
            if let Ok(info) = fs::read_to_string("/proc/meminfo") {
                let get = |key: &str| info.lines().find_map(|l| l.strip_prefix(key)?.split_whitespace().next()?.parse::<f64>().ok());
                if let (Some(total), Some(available)) = (get("MemTotal:"), get("MemAvailable:")) {
                    h.ram = Some(((total - available).max(0.0) / 1048576.0, total / 1048576.0));
                }
            }
            h.gpu = self.gpu_paths.iter().filter_map(|p| fs::read_to_string(p).ok()?.trim().parse::<f64>().ok())
                .filter(|v| v.is_finite()).map(|v| v.clamp(0.0, 100.0)).reduce(f64::max);
            h
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod native {
    use super::*;
    pub struct Sampler;
    impl Sampler { pub fn new() -> Self { Self } pub fn sample(&mut self) -> Hardware { Hardware::default() } }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cpu_uses_deltas_and_excludes_idle() {
        assert_eq!(cpu_usage(None, 100, 50), None);
        assert_eq!(cpu_usage(Some((100, 50)), 200, 75), Some(75.0));
        assert_eq!(cpu_usage(Some((100, 50)), 100, 50), None);
        assert_eq!(cpu_usage(Some((200, 75)), 100, 50), None);
    }
    #[test]
    fn frame_history_is_bounded_and_stops_when_disabled() {
        let mut p = Performance::new();
        p.set_enabled(true);
        for _ in 0..HISTORY + 20 { p.presented(); }
        assert_eq!(p.history.len(), HISTORY);
        p.set_enabled(false);
        p.presented();
        assert_eq!(p.history.len(), HISTORY);
        assert!(p.worker.is_none());
    }
}
