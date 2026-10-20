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
            let (vol, mute) = volume_get()?;
            r.volume = Some((vol, mute));
            volume_set(0.10_f32.min(vol), false)?;
            Ok(None)
        }
        "beep" => {
            if let Ok((vol, mute)) = volume_get() {
                r.volume = Some((vol, mute));
                let _ = volume_set(vol.max(0.75), false);
            }
            Ok(None)
        }
        _ => Err(format!("unknown curse part {id}")),
    }
}

pub fn revert(r: &Restore) {
    if let Some((vol, mute)) = r.volume {
        let _ = volume_set(vol, mute);
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

fn volume_get() -> Result<(f32, bool), String> {
    let vol_out = osascript(&["output volume of (get volume settings)"])?;
    let vol: f32 = vol_out.trim().parse().map_err(|_| "couldn't parse osascript volume".to_string())?;
    let mute_out = osascript(&["output muted of (get volume settings)"])?;
    Ok((vol / 100.0, mute_out.trim() == "true"))
}

fn volume_set(vol: f32, mute: bool) -> Result<(), String> {
    let script = format!("set volume output volume {} output muted {}", (vol * 100.0).round() as i32, mute);
    osascript(&[script.as_str()]).map(|_| ())
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
