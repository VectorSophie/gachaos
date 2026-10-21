// macOS implementations for the only curses with a real cross-platform
// effect: wallpaper and volume, via `osascript`. Everything else Win32-only
// stays on `win32.rs`; macOS has no public runtime API for pointer speed or
// double-click timing to match it.
use std::process::Command;

use super::{heist_bmp, Restore, Worker};

pub fn apply(id: &str, r: &mut Restore) -> Result<Option<Worker>, String> {
    match id {
        "wall" => {
            r.wallpaper = Some(get_wallpaper()?);
            let path = std::env::temp_dir().join("gachaos-heist.bmp");
            std::fs::write(&path, heist_bmp(640, 360, 6)).map_err(|e| format!("couldn't write wallpaper: {e}"))?;
            set_wallpaper(&path.to_string_lossy())?;
            Ok(None)
        }
        "whisper" => {
            r.volume_delta = Some(adjust_volume(-VOLUME_STEP)?);
            Ok(None)
        }
        "beep" => {
            if let Ok(delta) = adjust_volume(VOLUME_STEP) {
                r.volume_delta = Some(delta);
            }
            Ok(None)
        }
        _ => Err(format!("unknown curse part {id}")),
    }
}

pub fn revert(r: &Restore) {
    if let Some(delta) = r.volume_delta {
        let _ = unapply_volume(delta);
    }
    if let Some(path) = &r.wallpaper {
        let _ = set_wallpaper(path);
    }
}

fn get_wallpaper() -> Result<String, String> {
    osascript(&[r#"tell application "System Events" to set p to picture of desktop 1"#, "POSIX path of p"])
}

fn set_wallpaper(path: &str) -> Result<(), String> {
    let script = format!(r#"tell application "System Events" to set picture of desktop 1 to "{path}""#);
    osascript(&[script.as_str()]).map(|_| ())
}

fn volume_get_scalar() -> Result<f32, String> {
    let vol_out = osascript(&["output volume of (get volume settings)"])?;
    let vol: f32 = vol_out.trim().parse().map_err(|_| "couldn't parse osascript volume".to_string())?;
    Ok(vol / 100.0)
}

fn volume_set_scalar(vol: f32) -> Result<(), String> {
    let script = format!("set volume output volume {} output muted false", (vol * 100.0).round() as i32);
    osascript(&[script.as_str()]).map(|_| ())
}

/// Whisper/Beep shift the current volume by this many steps per roll (0-1
/// scale), same delta-based model as `win32.rs`'s `SPEED_STEP`/`VOLUME_STEP`.
const VOLUME_STEP: f32 = 0.20;

/// Nudges volume by `delta` and returns the delta actually applied (may be
/// smaller than requested if clamped), for an exact revert later.
fn adjust_volume(delta: f32) -> Result<f32, String> {
    let current = volume_get_scalar()?;
    let new = (current + delta).clamp(0.0, 1.0);
    volume_set_scalar(new)?;
    Ok(new - current)
}

/// Undoes exactly the delta `adjust_volume` applied, relative to whatever
/// the volume is now (other stacked curses may have changed it too).
fn unapply_volume(delta: f32) -> Result<(), String> {
    let current = volume_get_scalar()?;
    volume_set_scalar((current - delta).clamp(0.0, 1.0))
}

fn osascript(lines: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("osascript");
    for line in lines {
        cmd.arg("-e").arg(line);
    }
    let out = cmd.output().map_err(|e| format!("couldn't run osascript: {e}"))?;
    if !out.status.success() {
        return Err(format!("osascript failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
