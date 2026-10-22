// Curse effects. Each part changes one thing about the user's session and
// records in `Restore` exactly what it changed, so it can be undone later: on
// expiry, by the panic hotkey, on quit, by the watchdog if the app is killed,
// or on the next launch. On Windows every change is session-only (nothing is
// written to the user's profile), so a reboot also undoes it; on Linux/macOS,
// wallpaper and volume changes go through gsettings/pactl/osascript and can
// outlive a reboot until gachaos next runs to revert them. Overlay parts
// aren't handled here: gachaos draws them in its own window (see main.rs).
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::game;

#[cfg(windows)]
mod win32;
#[cfg(windows)]
use self::win32 as platform;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use self::linux as platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use self::macos as platform;

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod stub;
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
use self::stub as platform;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Restore {
    /// System cursors were replaced; reload the user's scheme.
    pub cursors: bool,
    /// Original "buttons swapped" setting.
    pub swap: Option<bool>,
    /// Original wallpaper path ("" means no wallpaper).
    pub wallpaper: Option<String>,
    /// Original dark-mode wallpaper path, Linux only (gsettings tracks light
    /// and dark wallpapers separately; None everywhere else).
    pub wallpaper_dark: Option<String>,
    /// Quarter turns this curse actually applied to the display orientation.
    /// Reverting subtracts them from the CURRENT orientation rather than
    /// restoring an absolute original, same delta-based model as
    /// `volume_delta` — capturing an absolute "original" broke when rotation
    /// was rolled twice in a row: the second roll's "original" was already
    /// the first roll's rotated state, so curing both left the display
    /// rotated no matter which reverted last.
    pub orientation_turns: Option<i32>,
    /// Signed delta this curse applied to master volume (0-1 scale). Reverting
    /// subtracts it from the CURRENT value rather than restoring an absolute
    /// original, so concurrently stacked volume curses combine and cancel
    /// correctly regardless of apply/revert order. Mute is simply left
    /// unmuted by these curses; the original mute state isn't tracked.
    pub volume_delta: Option<f32>,
    /// Original desktop background colour (COLORREF).
    pub desktop_color: Option<u32>,
    /// Signed delta this curse applied to pointer speed (1-20 scale), same
    /// delta-based model as `volume_delta`.
    pub mouse_speed_delta: Option<i32>,
    /// Original number of mouse trails (0 = off).
    pub trails: Option<u32>,
    /// Original double-click time in milliseconds.
    pub dblclick: Option<u32>,
}

/// A repeating effect (cursor drift, beeps, screen colour) that runs on its
/// own thread until the curse is reverted. `finish` runs on that same thread
/// when it stops, for APIs that must be torn down where they were set up.
pub struct Worker {
    pub every: Duration,
    pub tick: Box<dyn FnMut() + Send>,
    pub finish: Option<Box<dyn FnOnce() + Send>>,
}

impl Worker {
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn new(every_ms: u64, tick: impl FnMut() + Send + 'static) -> Self {
        Worker { every: Duration::from_millis(every_ms), tick: Box::new(tick), finish: None }
    }
}

/// Identifies one curse's own worker threads, so reverting it stops only
/// those, not other curses stacked alongside it. 0 means "no workers" (the
/// value a `#[serde(skip)]` `ActiveCurse.token` gets back from a save file,
/// where it's meaningless anyway: workers never survive a restart).
pub type Token = u64;

/// What applying a curse's parts achieved.
pub struct Applied {
    pub restore: Restore,
    /// Parts that took effect (overlay parts always do).
    pub parts: Vec<String>,
    /// Parts Windows refused, with the reason.
    pub failed: Vec<(String, String)>,
    pub token: Token,
}

#[derive(Default)]
pub struct Runner {
    workers: Mutex<Vec<(Token, Arc<AtomicBool>, JoinHandle<()>)>>,
    next_token: AtomicU64,
}

impl Runner {
    /// Applies every part it can. Stacked curses keep whatever worked; it's an
    /// error only when nothing took effect at all.
    pub fn apply(&self, parts: &[&str]) -> Result<Applied, String> {
        let token = self.next_token.fetch_add(1, Ordering::Relaxed) + 1; // never 0
        let mut out = Applied { restore: Restore::default(), parts: Vec::new(), failed: Vec::new(), token };
        for p in parts {
            if game::part(p).is_some_and(|d| d.overlay) {
                out.parts.push(p.to_string());
                continue;
            }
            let mut r = Restore::default();
            match platform::apply(p, &mut r) {
                Ok(worker) => {
                    merge(&mut out.restore, r);
                    if let Some(w) = worker {
                        self.start(w, token);
                    }
                    out.parts.push(p.to_string());
                }
                Err(e) => {
                    platform::revert(&r); // undo whatever part of it did take effect
                    out.failed.push((p.to_string(), e));
                }
            }
        }
        if out.parts.is_empty() {
            let reason = out.failed.first().map(|f| f.1.clone()).unwrap_or_else(|| "nothing to apply".into());
            return Err(reason);
        }
        Ok(out)
    }

    /// Reverts one curse: stops only the worker threads tagged with its own
    /// `token`, leaving any other stacked curse's workers running.
    pub fn revert(&self, token: Token, restore: &Restore) {
        self.stop(token);
        platform::revert(restore);
    }

    fn start(&self, w: Worker, token: Token) {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let Worker { every, mut tick, finish } = w;
        let handle = std::thread::spawn(move || {
            let step = Duration::from_millis(15);
            while !flag.load(Ordering::Relaxed) {
                tick();
                let mut waited = Duration::ZERO;
                while waited < every && !flag.load(Ordering::Relaxed) {
                    std::thread::sleep(step);
                    waited += step;
                }
            }
            if let Some(f) = finish {
                f();
            }
        });
        self.workers.lock().unwrap().push((token, stop, handle));
    }

    fn stop(&self, token: Token) {
        let (mine, rest): (Vec<_>, Vec<_>) =
            std::mem::take(&mut *self.workers.lock().unwrap()).into_iter().partition(|(t, _, _)| *t == token);
        *self.workers.lock().unwrap() = rest;
        for (_, stop, _) in &mine {
            stop.store(true, Ordering::Relaxed);
        }
        for (_, _, handle) in mine {
            let _ = handle.join();
        }
    }
}

/// Undoes a saved curse with no workers running: crash recovery and the watchdog.
pub fn revert_saved(restore: &Restore) {
    platform::revert(restore);
}

/// Parts never share a slot, so at most one side has each field set.
fn merge(into: &mut Restore, r: Restore) {
    into.cursors |= r.cursors;
    into.swap = into.swap.or(r.swap);
    into.wallpaper = into.wallpaper.take().or(r.wallpaper);
    into.wallpaper_dark = into.wallpaper_dark.take().or(r.wallpaper_dark);
    into.orientation_turns = into.orientation_turns.or(r.orientation_turns);
    into.volume_delta = into.volume_delta.or(r.volume_delta);
    into.desktop_color = into.desktop_color.or(r.desktop_color);
    into.mouse_speed_delta = into.mouse_speed_delta.or(r.mouse_speed_delta);
    into.trails = into.trails.or(r.trails);
    into.dblclick = into.dblclick.or(r.dblclick);
}

/// The pixel arrow used for Giant Cursor. '#' outline, 'o' fill.
#[cfg_attr(not(windows), allow(dead_code))]
pub const ARROW: [&str; 19] = [
    "#...........", "##..........", "#o#.........", "#oo#........", "#ooo#.......", "#oooo#......", "#ooooo#.....",
    "#oooooo#....", "#ooooooo#...", "#oooooooo#..", "#ooooooooo#.", "#oooooo#####", "#ooo#oo#....", "#oo#.#oo#...",
    "#o#..#oo#...", "##....#oo#..", "#.....#oo#..", ".......#oo#.", ".......###..",
];

/// Monochrome cursor masks (AND then XOR, 1 bit per pixel, rows padded to
/// 16 bits) for the arrow scaled up, on a `size` x `size` canvas.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn arrow_masks(scale: usize, size: usize) -> Vec<u8> {
    let stride = size.div_ceil(16) * 2;
    let mut and = vec![0xFFu8; stride * size];
    let mut xor = vec![0u8; stride * size];
    for (y, row) in ARROW.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == '.' {
                continue;
            }
            for sy in 0..scale {
                for sx in 0..scale {
                    let (px, py) = (x * scale + sx, y * scale + sy);
                    if px >= size || py >= size {
                        continue;
                    }
                    let (byte, bit) = (py * stride + px / 8, 0x80u8 >> (px % 8));
                    and[byte] &= !bit;
                    if ch == 'o' {
                        xor[byte] |= bit;
                    }
                }
            }
        }
    }
    and.extend(xor);
    and
}

/// A loud 24-bit BMP for Wallpaper Heist: a magenta/lime zigzag on navy,
/// drawn as chunky pixels.
#[cfg_attr(not(any(windows, target_os = "linux", target_os = "macos")), allow(dead_code))]
pub fn heist_bmp(w: u32, h: u32, px: u32) -> Vec<u8> {
    let row = (w * 3).div_ceil(4) * 4;
    let size = 54 + row * h;
    let mut b = Vec::with_capacity(size as usize);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(w as i32).to_le_bytes());
    b.extend_from_slice(&(h as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0; 24]);
    for y in (0..h).rev() {
        let start = b.len();
        for x in 0..w {
            let (cx, cy) = (x / px, y / px);
            let [r, g, bl] = if (cx + cy) % 8 < 2 {
                [0xff, 0x4f, 0xd8]
            } else if (cx + 8 - cy % 8) % 8 < 2 {
                [0x7c, 0xff, 0x4f]
            } else {
                [0x2b, 0x24, 0x61]
            };
            b.extend_from_slice(&[bl, g, r]);
        }
        b.resize(start + row as usize, 0);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heist_bmp_has_a_valid_header_and_size() {
        let b = heist_bmp(64, 32, 4);
        assert_eq!(&b[0..2], b"BM");
        assert_eq!(u32::from_le_bytes(b[2..6].try_into().unwrap()) as usize, b.len());
        assert_eq!(b.len(), 54 + 64 * 3 * 32);
    }

    #[test]
    fn arrow_masks_mark_outline_and_fill() {
        let m = arrow_masks(4, 80);
        let stride = 10;
        assert_eq!(m.len(), stride * 80 * 2);
        // Top-left pixel is outline: AND bit clear, XOR bit clear (black).
        assert_eq!(m[0] & 0x80, 0);
        assert_eq!(m[stride * 80] & 0x80, 0);
        // Far right of the first row is transparent: AND bit set.
        assert_eq!(m[stride - 1] & 0x01, 0x01);
        // A fill pixel (arrow row 4, col 1 -> pixel 4,16) is white: XOR bit set.
        let (px, py) = (4usize, 16usize);
        assert_ne!(m[stride * 80 + py * stride + px / 8] & (0x80 >> (px % 8)), 0);
    }

    #[test]
    fn runner_stops_only_the_reverted_curses_workers() {
        let r = Runner::default();
        let hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let h = hits.clone();
        r.start(Worker::new(20, move || { h.fetch_add(1, Ordering::Relaxed); }), 1);
        let finished = Arc::new(AtomicBool::new(false));
        let f = finished.clone();
        r.start(Worker { finish: Some(Box::new(move || f.store(true, Ordering::Relaxed))), ..Worker::new(20, || {}) }, 1);
        // A second curse's worker, stacked alongside the first under a different token.
        let other_hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let oh = other_hits.clone();
        r.start(Worker::new(20, move || { oh.fetch_add(1, Ordering::Relaxed); }), 2);
        std::thread::sleep(Duration::from_millis(120));
        r.revert(1, &Restore::default());
        let after = hits.load(Ordering::Relaxed);
        assert!(after > 0);
        assert!(finished.load(Ordering::Relaxed), "finish ran on revert");
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(hits.load(Ordering::Relaxed), after, "token 1's worker stayed stopped");
        assert!(other_hits.load(Ordering::Relaxed) > 0, "token 2's worker kept running, unaffected by token 1's revert");
    }
}
