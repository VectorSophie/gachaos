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
            // No portable system-beep API across desktops; raise the volume only.
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
    let out = run(&["gsettings", "get", "org.gnome.desktop.background", "picture-uri"])?;
    Ok(out.trim().trim_matches('\'').trim_start_matches("file://").to_string())
}

fn set_wallpaper(path: &str) -> Result<(), String> {
    let uri = format!("file://{path}");
    run(&["gsettings", "set", "org.gnome.desktop.background", "picture-uri", &uri])?;
    let _ = run(&["gsettings", "set", "org.gnome.desktop.background", "picture-uri-dark", &uri]);
    Ok(())
}

fn volume_get() -> Result<(f32, bool), String> {
    let vol_out = run(&["pactl", "get-sink-volume", "@DEFAULT_SINK@"])?;
    let pct: f32 = vol_out
        .split('/')
        .nth(1)
        .and_then(|s| s.trim().trim_end_matches('%').parse().ok())
        .ok_or_else(|| "couldn't parse pactl volume output".to_string())?;
    let mute_out = run(&["pactl", "get-sink-mute", "@DEFAULT_SINK@"])?;
    Ok((pct / 100.0, mute_out.contains("yes")))
}

fn volume_set(vol: f32, mute: bool) -> Result<(), String> {
    run(&["pactl", "set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", (vol * 100.0).round() as i32)])?;
    run(&["pactl", "set-sink-mute", "@DEFAULT_SINK@", if mute { "1" } else { "0" }])?;
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
    #[test]
    fn volume_pct_parses_from_pactl_style_output() {
        let line = "Volume: front-left: 65536 /  73% / 0.00 dB,   front-right: 65536 /  73% / 0.00 dB\n";
        let pct: f32 = line.split('/').nth(1).and_then(|s| s.trim().trim_end_matches('%').parse().ok()).unwrap();
        assert_eq!(pct, 73.0);
    }
}
