// Curse effects. Each one changes something about the user's Windows session
// and records in `Restore` exactly what it changed, so it can be undone later:
// on expiry, by the panic hotkey, on quit, or on the next launch after a crash.
// Every change is session-only (nothing is written to the registry), so a
// reboot also undoes it.
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

#[cfg(windows)]
mod win32;
#[cfg(windows)]
use self::win32 as platform;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
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
    /// Original display orientation (0-3) and resolution.
    pub orientation: Option<(u32, u32, u32)>,
    /// Original master volume (0-1) and mute state.
    pub volume: Option<(f32, bool)>,
    /// Original desktop background colour (COLORREF).
    pub desktop_color: Option<u32>,
}

/// A repeating effect (cursor drift, beeps, colour cycling) that runs on its
/// own thread until the curse is reverted.
pub struct Worker {
    pub every: Duration,
    pub tick: Box<dyn FnMut() + Send>,
}

#[derive(Default)]
pub struct Runner {
    worker: Mutex<Option<(Arc<AtomicBool>, JoinHandle<()>)>>,
}

impl Runner {
    pub fn apply(&self, id: &str) -> Result<Restore, String> {
        let mut restore = Restore::default();
        match platform::apply(id, &mut restore) {
            Ok(worker) => {
                if let Some(w) = worker {
                    self.start(w);
                }
                Ok(restore)
            }
            Err(e) => {
                // Undo whatever part of it did take effect.
                platform::revert(&restore);
                Err(e)
            }
        }
    }

    pub fn revert(&self, restore: &Restore) {
        self.stop();
        platform::revert(restore);
    }

    fn start(&self, mut w: Worker) {
        self.stop();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::spawn(move || {
            let step = Duration::from_millis(20);
            while !flag.load(Ordering::Relaxed) {
                (w.tick)();
                let mut waited = Duration::ZERO;
                while waited < w.every && !flag.load(Ordering::Relaxed) {
                    std::thread::sleep(step);
                    waited += step;
                }
            }
        });
        *self.worker.lock().unwrap() = Some((stop, handle));
    }

    fn stop(&self) {
        if let Some((stop, handle)) = self.worker.lock().unwrap().take() {
            stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
        }
    }
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
#[cfg_attr(not(windows), allow(dead_code))]
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
    fn runner_stops_its_worker_on_revert() {
        let r = Runner::default();
        let hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let h = hits.clone();
        r.start(Worker { every: Duration::from_millis(20), tick: Box::new(move || { h.fetch_add(1, Ordering::Relaxed); }) });
        std::thread::sleep(Duration::from_millis(120));
        r.revert(&Restore::default());
        let after = hits.load(Ordering::Relaxed);
        assert!(after > 0);
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(hits.load(Ordering::Relaxed), after);
    }
}
