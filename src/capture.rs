//! Desktop integration. Do not export an RTSS opt-out marker: external
//! performance overlays, screenshots and recorders must be allowed to hook.

/// Miniquad forwards WM_ACTIVATE as a minimise event, but some overlays
/// only move foreground focus. Poll it too so a hidden cursor never traps
/// the screenshot picker or leaves a flight control held down.
#[cfg(windows)]
pub fn is_foreground() -> bool {
    use std::ffi::c_void;
    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> *mut c_void;
        fn GetWindowThreadProcessId(window: *mut c_void, process: *mut u32) -> u32;
    }
    unsafe {
        let mut process = 0;
        let window = GetForegroundWindow();
        !window.is_null() && GetWindowThreadProcessId(window, &mut process) != 0 && process == std::process::id()
    }
}

#[cfg(not(windows))]
pub fn is_foreground() -> bool { true }

/// Save the composed OpenGL back buffer before SwapBuffers, so fullscreen
/// screenshots do not depend on a vendor overlay or a desktop capture hook.
/// Call inside the final render pass, after flushing the UI batch.
pub fn screenshot(w: u32, h: u32) -> std::io::Result<std::path::PathBuf> {
    use miniquad::gl;
    use std::{io, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
    let len = (w as usize).checked_mul(h as usize).and_then(|n| n.checked_mul(4))
        .filter(|&n| n > 0 && n <= 256 * 1024 * 1024)
        .ok_or_else(|| io::Error::other("invalid screenshot dimensions"))?;
    let mut pixels = vec![0u8; len];
    unsafe { gl::glReadPixels(0, 0, w as i32, h as i32, gl::GL_RGBA, gl::GL_UNSIGNED_BYTE, pixels.as_mut_ptr().cast()); }
    let base = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from).ok_or_else(|| io::Error::other("pictures directory unavailable"))?;
    let dir = base.join("Pictures").join("viper-run");
    std::fs::create_dir_all(&dir)?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let path = dir.join(format!("viper-run-{stamp}.png"));
    crate::png::write(&path.to_string_lossy(), w, h, &pixels, true)?;
    Ok(path)
}