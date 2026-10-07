// Win32 implementations. Each `apply` records what it is about to change in
// `Restore` before changing it, so a partial failure can still be undone.
use std::time::Duration;

use windows::core::{PCWSTR, BOOL};
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsW, CreateBitmap, DeleteObject, EnumDisplaySettingsW, GetSysColor, SetSysColors, CDS_TYPE, COLOR_DESKTOP,
    DEVMODEW, DEVMODE_DISPLAY_ORIENTATION, DISP_CHANGE_SUCCESSFUL, DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH,
    ENUM_CURRENT_SETTINGS,
};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::UI::Input::KeyboardAndMouse::SwapMouseButton;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, GetCursorPos, GetSystemMetrics, SetCursorPos, SetSystemCursor, SystemParametersInfoW, HCURSOR, ICONINFO, MB_ICONASTERISK, MB_ICONEXCLAMATION, MB_ICONHAND,
    MB_ICONQUESTION, MB_OK, OCR_HAND, OCR_IBEAM, OCR_NORMAL, SM_SWAPBUTTON, SPIF_SENDCHANGE, SPI_GETDESKWALLPAPER,
    SPI_SETCURSORS, SPI_SETDESKWALLPAPER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

use super::{arrow_masks, heist_bmp, Restore, Worker};

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
        "jitter" => Ok(Some(jitter())),
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
            if let Ok((vol, mute)) = volume_get() {
                r.volume = Some((vol, mute));
                let _ = volume_set(vol.max(0.75), false);
            }
            Ok(Some(beeps()))
        }
        "disco" => {
            r.desktop_color = Some(unsafe { GetSysColor(COLOR_DESKTOP) });
            r.wallpaper = Some(get_wallpaper()?);
            set_wallpaper("")?; // the desktop colour only shows with no wallpaper
            Ok(Some(disco()))
        }
        _ => Err(format!("unknown curse {id}")),
    }
}

pub fn revert(r: &Restore) {
    if r.cursors {
        unsafe { let _ = SystemParametersInfoW(SPI_SETCURSORS, 0, None, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)); }
    }
    if let Some(was) = r.swap {
        unsafe { let _ = SwapMouseButton(was); }
    }
    if let Some((orient, w, h)) = r.orientation {
        let _ = set_orientation(orient, w, h);
    }
    if let Some((vol, mute)) = r.volume {
        let _ = volume_set(vol, mute);
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

fn jitter() -> Worker {
    let mut rng = fastrand::Rng::new();
    let (mut vx, mut vy, mut ax, mut ay, mut t) = (0f32, 0f32, 0f32, 0f32, 0u32);
    Worker {
        every: Duration::from_millis(30),
        tick: Box::new(move || {
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
                unsafe {
                    let mut p = Default::default();
                    if GetCursorPos(&mut p).is_ok() {
                        let _ = SetCursorPos(p.x + dx, p.y + dy);
                    }
                }
            }
        }),
    }
}

// ---------- wallpaper / desktop colour ----------

fn get_wallpaper() -> Result<String, String> {
    let mut buf = [0u16; 260];
    unsafe {
        SystemParametersInfoW(SPI_GETDESKWALLPAPER, buf.len() as u32, Some(buf.as_mut_ptr() as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0))
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
    Worker {
        every: Duration::from_millis(300),
        tick: Box::new(move || {
            set_desktop_color(COLORS[i % COLORS.len()]);
            i += 1;
        }),
    }
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

fn volume_get() -> windows::core::Result<(f32, bool)> {
    let ep = endpoint()?;
    unsafe { Ok((ep.GetMasterVolumeLevelScalar()?, ep.GetMute()?.as_bool())) }
}

fn volume_set(vol: f32, mute: bool) -> windows::core::Result<()> {
    let ep = endpoint()?;
    unsafe {
        ep.SetMasterVolumeLevelScalar(vol, std::ptr::null())?;
        ep.SetMute(mute, std::ptr::null())
    }
}

fn beeps() -> Worker {
    let mut rng = fastrand::Rng::new();
    let mut wait = 0u32;
    Worker {
        every: Duration::from_millis(100),
        tick: Box::new(move || {
            if wait > 0 {
                wait -= 1;
                return;
            }
            let kind = [MB_OK, MB_ICONHAND, MB_ICONEXCLAMATION, MB_ICONASTERISK, MB_ICONQUESTION][rng.usize(..5)];
            unsafe { let _ = MessageBeep(kind); }
            wait = rng.u32(3..9);
        }),
    }
}
