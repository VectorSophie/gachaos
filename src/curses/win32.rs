// Win32 implementations. Each `apply` records what it is about to change in
// `Restore` before changing it, so a partial failure can still be undone.
// Screen colour and zoom use the Magnification API: Windows drops those
// effects by itself when the process ends, so they can't outlive gachaos.
use std::ffi::c_void;

use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsW, CreateBitmap, DeleteObject, EnumDisplaySettingsW, GetSysColor, SetSysColors, CDS_TYPE, COLOR_DESKTOP,
    DEVMODEW, DEVMODE_DISPLAY_ORIENTATION, DISP_CHANGE_SUCCESSFUL, DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH,
    ENUM_CURRENT_SETTINGS,
};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetDoubleClickTime, SetDoubleClickTime, SwapMouseButton};
use windows::Win32::UI::Magnification::{
    MagInitialize, MagSetFullscreenColorEffect, MagSetFullscreenTransform, MagUninitialize, MAGCOLOREFFECT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, GetCursorPos, GetForegroundWindow, GetSystemMetrics, GetWindowRect, GetWindowThreadProcessId, IsZoomed,
    SetCursorPos, SetSystemCursor, SetWindowPos, SystemParametersInfoW, HCURSOR, ICONINFO, MB_ICONASTERISK, MB_ICONEXCLAMATION,
    MB_ICONHAND, MB_ICONQUESTION, MB_OK, OCR_HAND, OCR_IBEAM, OCR_NORMAL, SM_CXSCREEN, SM_CYSCREEN, SM_SWAPBUTTON, SPIF_SENDCHANGE,
    SPI_GETDESKWALLPAPER, SPI_GETMOUSESPEED, SPI_GETMOUSETRAILS, SPI_SETCURSORS, SPI_SETDESKWALLPAPER, SPI_SETMOUSESPEED,
    SPI_SETMOUSETRAILS, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

use super::{arrow_masks, heist_bmp, Restore, Worker};

/// Applies now, not saved to the user's profile.
const SESSION_ONLY: SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS = SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0);

pub fn apply(id: &str, r: &mut Restore) -> Result<Option<Worker>, String> {
    match id {
        "giant" => {
            r.cursors = true;
            giant_cursor()?;
            Ok(None)
        }
        "wrong" => {
            let was = unsafe { GetSystemMetrics(SM_SWAPBUTTON) } != 0;
            r.swap = Some(was);
            unsafe { let _ = SwapMouseButton(!was); }
            Ok(None)
        }
        "sloth" | "turbo" => {
            let delta = if id == "sloth" { -SPEED_STEP } else { SPEED_STEP };
            r.mouse_speed_delta = Some(adjust_mouse_speed(delta)?);
            Ok(None)
        }
        "trails" => {
            r.trails = Some(get_trails()?);
            set_trails(7)?;
            Ok(None)
        }
        "butter" => {
            r.dblclick = Some(unsafe { GetDoubleClickTime() });
            unsafe { SetDoubleClickTime(120) }.map_err(|e| format!("couldn't change double-click speed: {e}"))?;
            Ok(None)
        }
        "whisper" => {
            r.volume_delta = Some(adjust_volume(-VOLUME_STEP).map_err(|e| format!("couldn't reach the speakers: {e}"))?);
            Ok(None)
        }
        "wobble" => Ok(Some(wobble())),
        "jitter" => Ok(Some(jitter())),
        "teleport" => Ok(Some(teleport())),
        "wall" => {
            r.wallpaper = Some(get_wallpaper()?);
            let path = std::env::temp_dir().join("gachaos-heist.bmp");
            std::fs::write(&path, heist_bmp(640, 360, 6)).map_err(|e| format!("couldn't write wallpaper: {e}"))?;
            set_wallpaper(&path.to_string_lossy())?;
            Ok(None)
        }
        "upside" => {
            rotate(r, 2)?;
            Ok(None)
        }
        "sideways" => {
            rotate(r, 1)?;
            Ok(None)
        }
        "beep" => {
            // Volume is best-effort: some machines have no default output device.
            if let Ok(delta) = adjust_volume(VOLUME_STEP) {
                r.volume_delta = Some(delta);
            }
            Ok(Some(beeps()))
        }
        "disco" => {
            r.desktop_color = Some(unsafe { GetSysColor(COLOR_DESKTOP) });
            r.wallpaper = Some(get_wallpaper()?);
            set_wallpaper("")?; // the desktop colour only shows with no wallpaper
            Ok(Some(disco()))
        }
        "negative" => colour_effect(INVERT),
        "silent" => colour_effect(GREYSCALE),
        "bloodred" => colour_effect(BLOOD),
        "tooclose" => zoom(),
        _ => Err(format!("unknown curse part {id}")),
    }
}

pub fn revert(r: &Restore) {
    if r.cursors {
        unsafe { let _ = SystemParametersInfoW(SPI_SETCURSORS, 0, None, SESSION_ONLY); }
    }
    if let Some(was) = r.swap {
        unsafe { let _ = SwapMouseButton(was); }
    }
    if let Some(delta) = r.mouse_speed_delta {
        let _ = unapply_mouse_speed(delta);
    }
    if let Some(n) = r.trails {
        let _ = set_trails(n);
    }
    if let Some(ms) = r.dblclick {
        unsafe { let _ = SetDoubleClickTime(ms); }
    }
    if let Some((orient, w, h)) = r.orientation {
        let _ = set_orientation(orient, w, h);
    }
    if let Some(delta) = r.volume_delta {
        let _ = unapply_volume(delta);
    }
    if let Some(c) = r.desktop_color {
        set_desktop_color(c);
    }
    if let Some(path) = &r.wallpaper {
        let _ = set_wallpaper(path);
    }
}

// ---------- cursor ----------

fn giant_cursor() -> Result<(), String> {
    const SIZE: usize = 80;
    let masks = arrow_masks(4, SIZE);
    // SetSystemCursor takes ownership of the handle, so each slot needs its own copy.
    for slot in [OCR_NORMAL, OCR_IBEAM, OCR_HAND] {
        unsafe {
            let bmp = CreateBitmap(SIZE as i32, (SIZE * 2) as i32, 1, 1, Some(masks.as_ptr() as *const _));
            let info = ICONINFO { fIcon: BOOL(0), xHotspot: 0, yHotspot: 0, hbmMask: bmp, hbmColor: Default::default() };
            let icon = CreateIconIndirect(&info);
            let _ = DeleteObject(bmp.into());
            let icon = icon.map_err(|e| format!("couldn't build cursor: {e}"))?;
            SetSystemCursor(HCURSOR(icon.0), slot).map_err(|e| format!("couldn't set cursor: {e}"))?;
        }
    }
    Ok(())
}

fn get_mouse_speed() -> Result<u32, String> {
    let mut v = 0u32;
    unsafe { SystemParametersInfoW(SPI_GETMOUSESPEED, 0, Some(&mut v as *mut u32 as *mut c_void), SESSION_ONLY) }
        .map_err(|e| format!("couldn't read pointer speed: {e}"))?;
    Ok(v)
}

fn set_mouse_speed(v: u32) -> Result<(), String> {
    // SPI_SETMOUSESPEED takes the value itself in the pointer argument.
    unsafe { SystemParametersInfoW(SPI_SETMOUSESPEED, 0, Some(v as usize as *mut c_void), SESSION_ONLY) }
        .map_err(|e| format!("couldn't change pointer speed: {e}"))
}

/// Sloth/Turbo shift the current pointer speed by this many steps per roll,
/// rather than jumping to an absolute extreme, so stacked rolls (and the
/// opposite curse) combine and cancel: current + delta, clamped to 1-20.
const SPEED_STEP: i32 = 6;

/// Nudges pointer speed by `delta` and returns the delta actually applied
/// (may be smaller than requested if clamped), for an exact revert later.
fn adjust_mouse_speed(delta: i32) -> Result<i32, String> {
    let current = get_mouse_speed()? as i32;
    let new = (current + delta).clamp(1, 20);
    set_mouse_speed(new as u32)?;
    Ok(new - current)
}

/// Undoes exactly the delta `adjust_mouse_speed` applied, relative to
/// whatever the speed is now (other stacked curses may have changed it too).
fn unapply_mouse_speed(delta: i32) -> Result<(), String> {
    let current = get_mouse_speed()? as i32;
    set_mouse_speed((current - delta).clamp(1, 20) as u32)
}

fn get_trails() -> Result<u32, String> {
    let mut v = 0u32;
    unsafe { SystemParametersInfoW(SPI_GETMOUSETRAILS, 0, Some(&mut v as *mut u32 as *mut c_void), SESSION_ONLY) }
        .map_err(|e| format!("couldn't read mouse trails: {e}"))?;
    Ok(v)
}

fn set_trails(n: u32) -> Result<(), String> {
    unsafe { SystemParametersInfoW(SPI_SETMOUSETRAILS, n, None, SESSION_ONLY) }.map_err(|e| format!("couldn't set mouse trails: {e}"))
}

fn screen_size() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

fn cursor() -> Option<(i32, i32)> {
    let mut p = Default::default();
    unsafe { GetCursorPos(&mut p) }.ok().map(|_| (p.x, p.y))
}

fn jitter() -> Worker {
    let mut rng = fastrand::Rng::new();
    let (mut vx, mut vy, mut ax, mut ay, mut t) = (0f32, 0f32, 0f32, 0f32, 0u32);
    Worker::new(30, move || {
        // Drift in one direction for ~1.2 s, then pick another.
        if t % 40 == 0 {
            let a = rng.f32() * std::f32::consts::TAU;
            let s = 1.2 + rng.f32() * 2.0;
            (vx, vy) = (a.cos() * s, a.sin() * s);
        }
        t += 1;
        ax += vx;
        ay += vy;
        let (dx, dy) = (ax.trunc() as i32, ay.trunc() as i32);
        ax -= dx as f32;
        ay -= dy as f32;
        if dx != 0 || dy != 0 {
            if let Some((x, y)) = cursor() {
                unsafe { let _ = SetCursorPos(x + dx, y + dy); }
            }
        }
    })
}

fn teleport() -> Worker {
    let mut rng = fastrand::Rng::new();
    let mut first = true;
    Worker::new(3500, move || {
        if std::mem::take(&mut first) {
            return; // give the user a moment before the first jump
        }
        let (w, h) = screen_size();
        unsafe { let _ = SetCursorPos(rng.i32(0..w.max(1)), rng.i32(0..h.max(1))); }
    })
}

/// Nudges the foreground window around its own spot, and puts it back when
/// focus moves on or the curse ends.
fn wobble() -> Worker {
    let mut rng = fastrand::Rng::new();
    let tracked: std::sync::Arc<std::sync::Mutex<Option<(isize, i32, i32)>>> = Default::default();
    let fin = tracked.clone();
    let put_back = |t: Option<(isize, i32, i32)>| {
        if let Some((h, x, y)) = t {
            unsafe { let _ = SetWindowPos(HWND(h as *mut c_void), None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE); }
        }
    };
    Worker {
        finish: Some(Box::new(move || put_back(fin.lock().unwrap().take()))),
        ..Worker::new(110, move || unsafe {
            let hwnd = GetForegroundWindow();
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let mut t = tracked.lock().unwrap();
            if hwnd.0.is_null() || pid == GetCurrentProcessId() || IsZoomed(hwnd).as_bool() {
                put_back(t.take());
                return;
            }
            if t.map(|(h, _, _)| h) != Some(hwnd.0 as isize) {
                put_back(t.take());
                let mut rc = RECT::default();
                if GetWindowRect(hwnd, &mut rc).is_err() {
                    return;
                }
                *t = Some((hwnd.0 as isize, rc.left, rc.top));
            }
            let (_, x, y) = t.unwrap();
            let _ = SetWindowPos(hwnd, None, x + rng.i32(-4..=4), y + rng.i32(-3..=3), 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
        })
    }
}

// ---------- screen colour / zoom (Magnification API) ----------

#[rustfmt::skip]
const INVERT: [f32; 25] = [
    -1.0, 0.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0, 0.0,
    0.0, 0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 0.0, 1.0,
];
#[rustfmt::skip]
const GREYSCALE: [f32; 25] = [
    0.30, 0.30, 0.30, 0.0, 0.0,
    0.59, 0.59, 0.59, 0.0, 0.0,
    0.11, 0.11, 0.11, 0.0, 0.0,
    0.0, 0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 0.0, 1.0,
];
#[rustfmt::skip]
const BLOOD: [f32; 25] = [
    0.55, 0.05, 0.05, 0.0, 0.0,
    0.65, 0.04, 0.04, 0.0, 0.0,
    0.25, 0.02, 0.02, 0.0, 0.0,
    0.0, 0.0, 0.0, 1.0, 0.0,
    0.12, 0.0, 0.0, 0.0, 1.0,
];
#[rustfmt::skip]
const IDENTITY: [f32; 25] = [
    1.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Checks the magnifier can start here (it can't while Windows Magnifier runs).
fn mag_probe() -> Result<(), String> {
    unsafe {
        if !MagInitialize().as_bool() {
            return Err("the Windows magnifier isn't available (is Magnifier already running?)".into());
        }
        let _ = MagUninitialize();
    }
    Ok(())
}

/// The magnifier must be set up and torn down on one thread, so it lives on the worker's.
fn colour_effect(matrix: [f32; 25]) -> Result<Option<Worker>, String> {
    mag_probe()?;
    let mut ready = false;
    Ok(Some(Worker {
        finish: Some(Box::new(|| unsafe {
            let _ = MagSetFullscreenColorEffect(&MAGCOLOREFFECT { transform: IDENTITY });
            let _ = MagUninitialize();
        })),
        ..Worker::new(1000, move || unsafe {
            if !std::mem::replace(&mut ready, true) {
                let _ = MagInitialize();
            }
            let _ = MagSetFullscreenColorEffect(&MAGCOLOREFFECT { transform: matrix });
        })
    }))
}

fn zoom() -> Result<Option<Worker>, String> {
    mag_probe()?;
    const LEVEL: f32 = 1.8;
    let mut ready = false;
    Ok(Some(Worker {
        finish: Some(Box::new(|| unsafe {
            let _ = MagSetFullscreenTransform(1.0, 0, 0);
            let _ = MagUninitialize();
        })),
        ..Worker::new(16, move || unsafe {
            if !std::mem::replace(&mut ready, true) {
                let _ = MagInitialize();
            }
            let (w, h) = screen_size();
            let (vw, vh) = ((w as f32 / LEVEL) as i32, (h as f32 / LEVEL) as i32);
            let (cx, cy) = cursor().unwrap_or((w / 2, h / 2));
            let x = (cx - vw / 2).clamp(0, (w - vw).max(0));
            let y = (cy - vh / 2).clamp(0, (h - vh).max(0));
            let _ = MagSetFullscreenTransform(LEVEL, x, y);
        })
    }))
}

// ---------- wallpaper / desktop colour ----------

fn get_wallpaper() -> Result<String, String> {
    let mut buf = [0u16; 260];
    unsafe {
        SystemParametersInfoW(SPI_GETDESKWALLPAPER, buf.len() as u32, Some(buf.as_mut_ptr() as *mut _), SESSION_ONLY)
            .map_err(|e| format!("couldn't read wallpaper: {e}"))?;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Ok(String::from_utf16_lossy(&buf[..end]))
}

fn set_wallpaper(path: &str) -> Result<(), String> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    // SPIF_SENDCHANGE without SPIF_UPDATEINIFILE: applies now, not saved to the profile.
    unsafe {
        SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, Some(wide.as_ptr() as *mut _), SPIF_SENDCHANGE)
            .map_err(|e| format!("couldn't set wallpaper: {e}"))
    }
}

fn set_desktop_color(c: u32) {
    unsafe { let _ = SetSysColors(1, &(COLOR_DESKTOP.0 as i32), &COLORREF(c)); }
}

fn disco() -> Worker {
    // COLORREF is 0x00BBGGRR.
    const COLORS: [u32; 6] = [0xd84fff, 0xffd84f, 0x4ad8ff, 0x4fff7c, 0xd8369a, 0x3a6aff];
    let mut i = 0;
    Worker::new(300, move || {
        set_desktop_color(COLORS[i % COLORS.len()]);
        i += 1;
    })
}

// ---------- display orientation ----------

fn current_mode() -> Result<DEVMODEW, String> {
    let mut dm = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
    let ok = unsafe { EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut dm) };
    if ok.as_bool() { Ok(dm) } else { Err("couldn't read the display mode".into()) }
}

fn rotate(r: &mut Restore, quarter_turns: u32) -> Result<(), String> {
    let dm = current_mode()?;
    let orient = unsafe { dm.Anonymous1.Anonymous2.dmDisplayOrientation.0 };
    r.orientation = Some((orient, dm.dmPelsWidth, dm.dmPelsHeight));
    let (w, h) = if quarter_turns % 2 == 1 { (dm.dmPelsHeight, dm.dmPelsWidth) } else { (dm.dmPelsWidth, dm.dmPelsHeight) };
    set_orientation((orient + quarter_turns) % 4, w, h).map_err(|e| {
        r.orientation = None; // nothing changed, nothing to undo
        e
    })
}

fn set_orientation(orient: u32, w: u32, h: u32) -> Result<(), String> {
    let mut dm = current_mode()?;
    dm.Anonymous1.Anonymous2.dmDisplayOrientation = DEVMODE_DISPLAY_ORIENTATION(orient);
    dm.dmPelsWidth = w;
    dm.dmPelsHeight = h;
    dm.dmFields |= DM_DISPLAYORIENTATION | DM_PELSWIDTH | DM_PELSHEIGHT;
    // CDS_TYPE(0): change the mode dynamically without saving it to the registry.
    let res = unsafe { ChangeDisplaySettingsW(Some(&dm), CDS_TYPE(0)) };
    if res == DISP_CHANGE_SUCCESSFUL { Ok(()) } else { Err(format!("the display driver refused to rotate (code {})", res.0)) }
}

// ---------- audio ----------

fn endpoint() -> windows::core::Result<IAudioEndpointVolume> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        en.GetDefaultAudioEndpoint(eRender, eConsole)?.Activate(CLSCTX_ALL, None)
    }
}

/// Whisper/Beep shift the current volume by this many steps per roll (0-1
/// scale), same delta-based model as `SPEED_STEP`.
const VOLUME_STEP: f32 = 0.20;

/// Nudges master volume by `delta`, force-unmuting, and returns the delta
/// actually applied (may be smaller than requested if clamped).
fn adjust_volume(delta: f32) -> windows::core::Result<f32> {
    let ep = endpoint()?;
    unsafe {
        let current = ep.GetMasterVolumeLevelScalar()?;
        let new = (current + delta).clamp(0.0, 1.0);
        ep.SetMasterVolumeLevelScalar(new, std::ptr::null())?;
        ep.SetMute(false, std::ptr::null())?;
        Ok(new - current)
    }
}

/// Undoes exactly the delta `adjust_volume` applied, relative to whatever
/// the volume is now (other stacked curses may have changed it too).
fn unapply_volume(delta: f32) -> windows::core::Result<()> {
    let ep = endpoint()?;
    unsafe {
        let current = ep.GetMasterVolumeLevelScalar()?;
        ep.SetMasterVolumeLevelScalar((current - delta).clamp(0.0, 1.0), std::ptr::null())
    }
}

fn beeps() -> Worker {
    let mut rng = fastrand::Rng::new();
    let mut wait = 0u32;
    Worker::new(100, move || {
        if wait > 0 {
            wait -= 1;
            return;
        }
        let kind = [MB_OK, MB_ICONHAND, MB_ICONEXCLAMATION, MB_ICONASTERISK, MB_ICONQUESTION][rng.usize(..5)];
        unsafe { let _ = MessageBeep(kind); }
        wait = rng.u32(3..9);
    })
}
