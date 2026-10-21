// Linux implementations for the only curses with a real cross-platform
// effect: wallpaper and volume. Everything else Win32-only stays on
// `win32.rs`; there's no portable equivalent (Wayland exposes no
// pointer-speed API, and window manager behaviour varies too much to fake
// window distortion consistently).
use std::process::Command;

use super::{heist_bmp, Restore, Worker};

pub fn apply(id: &str, r: &mut Restore) -> Result<Option<Worker>, String> {
    match id {
        "wall" => {
            let (light, dark) = get_wallpaper()?;
            r.wallpaper = Some(light);
            r.wallpaper_dark = Some(dark);
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
            // No portable system-beep API across desktops; raise the volume only.
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
    match (&r.wallpaper, &r.wallpaper_dark) {
        (Some(light), Some(dark)) => {
            let _ = restore_wallpaper(light, dark);
        }
        (Some(light), None) => {
            let _ = set_wallpaper(light); // no separate dark value recorded
        }
        _ => {}
    }
}

fn get_wallpaper() -> Result<(String, String), String> {
    let strip = |s: String| s.trim().trim_matches('\'').trim_start_matches("file://").to_string();
    let light = run(&["gsettings", "get", "org.gnome.desktop.background", "picture-uri"])?;
    let dark = run(&["gsettings", "get", "org.gnome.desktop.background", "picture-uri-dark"])?;
    Ok((strip(light), strip(dark)))
}

/// Sets both the light and dark wallpaper to the same image — used while a
/// curse is active, so the prank shows regardless of the user's colour scheme.
fn set_wallpaper(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Ok(()); // nothing to set; avoids writing a broken "file://" URI
    }
    let uri = format!("file://{path}");
    run(&["gsettings", "set", "org.gnome.desktop.background", "picture-uri", &uri])?;
    let _ = run(&["gsettings", "set", "org.gnome.desktop.background", "picture-uri-dark", &uri]);
    Ok(())
}

/// Restores the light and dark wallpapers independently, for revert.
fn restore_wallpaper(light: &str, dark: &str) -> Result<(), String> {
    if !light.is_empty() {
        run(&["gsettings", "set", "org.gnome.desktop.background", "picture-uri", &format!("file://{light}")])?;
    }
    if !dark.is_empty() {
        let _ = run(&["gsettings", "set", "org.gnome.desktop.background", "picture-uri-dark", &format!("file://{dark}")]);
    }
    Ok(())
}

fn parse_pct(out: &str) -> Option<f32> {
    out.split('/').nth(1).and_then(|s| s.trim().trim_end_matches('%').parse().ok())
}

/// 0-1 scale, same convention as `win32.rs`'s `GetMasterVolumeLevelScalar`;
/// converted to a percentage only at the `pactl` command boundary.
fn volume_get_scalar() -> Result<f32, String> {
    let vol_out = run(&["pactl", "get-sink-volume", "@DEFAULT_SINK@"])?;
    parse_pct(&vol_out).ok_or_else(|| "couldn't parse pactl volume output".to_string()).map(|pct| pct / 100.0)
}

fn volume_set_scalar(vol: f32) -> Result<(), String> {
    run(&["pactl", "set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", (vol * 100.0).round() as i32)])?;
    run(&["pactl", "set-sink-mute", "@DEFAULT_SINK@", "0"])?;
    Ok(())
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
    run(&[
        "pactl",
        "set-sink-volume",
        "@DEFAULT_SINK@",
        &format!("{}%", ((current - delta).clamp(0.0, 1.0) * 100.0).round() as i32),
    ])?;
    Ok(())
}

fn run(args: &[&str]) -> Result<String, String> {
    let out = Command::new(args[0]).args(&args[1..]).output().map_err(|e| format!("couldn't run {}: {e}", args[0]))?;
    if !out.status.success() {
        return Err(format!("{} failed: {}", args[0], String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_pct;

    #[test]
    fn volume_pct_parses_from_pactl_style_output() {
        let line = "Volume: front-left: 65536 /  73% / 0.00 dB,   front-right: 65536 /  73% / 0.00 dB\n";
        assert_eq!(parse_pct(line), Some(73.0));
    }
}
