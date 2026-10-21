//! Per-executable compatibility with capture tools on Windows.
//!
//! RTSS and OBS can both patch OpenGL's SwapBuffers entry point. On affected
//! systems their hooks corrupt its prologue and crash before capture starts.
//! RTSS recognises this data export as the application's compatibility opt-out.
//! OBS Game Capture remains available; no external profiles are changed.

#[cfg(target_os = "windows")]
#[no_mangle]
#[used]
pub static RTSSHooksCompatibility: u32 = 0;
