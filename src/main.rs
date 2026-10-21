// gachaos: a drab legacy-looking exe that rolls a slot machine and curses
// (or blesses) your desktop. The UI (ui/index.html) handles the roll and the
// reveal; this side owns the save, the odds, the real OS curses, the overlay
// window for drawn curses, the tray, the Ctrl+Alt+P panic hotkey, and the
// watchdog that undoes curses if the app is killed.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod curses;
mod game;
mod save;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, RunEvent, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use game::{Claim, PullError, CATALOG, PITY, PITY_LEGEND, TICKET_CAP, TIER_NAMES, TIER_SECS};

struct Core {
    st: game::State,
    /// The result rolled by the last pull, waiting for its reveal to finish.
    pending: Option<&'static str>,
    /// Name of a curse undone at startup because the app had been closed abruptly.
    recovered: Option<String>,
    rng: fastrand::Rng,
}

struct Ctx {
    core: Mutex<Core>,
    runner: curses::Runner,
    save_path: PathBuf,
    qa: bool,
    /// Window position and size before it grew to cover the screen.
    geom: Mutex<Option<(PhysicalPosition<i32>, PhysicalSize<u32>)>>,
}

impl Ctx {
    fn save(&self, st: &game::State) {
        save::store(&self.save_path, st);
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}
fn now_secs() -> u64 {
    now_ms() / 1000
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EntryDto {
    id: &'static str,
    name: &'static str,
    tier: u8,
    kind: &'static str,
    secs: u64,
    desc: &'static str,
    one_in: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ActiveDto {
    id: String,
    until_ms: u64,
    parts: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Boons {
    free_pulls: u32,
    luck_pulls: u32,
    midas: u32,
    shields: u32,
    immune_until_ms: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    tickets: u32,
    cap: u32,
    pity: u32,
    pity_threshold: u32,
    pity_legend: u32,
    pity_legend_threshold: u32,
    pulls: u32,
    counts: Vec<u32>,
    best: i32,
    best_name: String,
    history: Vec<game::PullRecord>,
    sound: bool,
    qa: bool,
    next_ticket_at_ms: Option<u64>,
    /// Every curse currently stacked, most-recently-applied last.
    active: Vec<ActiveDto>,
    boons: Boons,
    recovered: Option<String>,
    tiers: [&'static str; game::TIERS],
    catalog: Vec<EntryDto>,
}

fn snapshot(ctx: &Ctx, core: &mut Core) -> Snapshot {
    let st = &core.st;
    let now = now_secs();
    Snapshot {
        tickets: st.tickets,
        cap: TICKET_CAP,
        pity: st.pity,
        pity_threshold: PITY,
        pity_legend: st.pity_legend,
        pity_legend_threshold: PITY_LEGEND,
        pulls: st.pulls,
        counts: st.counts.clone(),
        best: st.best,
        best_name: st.best_name.clone(),
        history: st.history.clone(),
        sound: st.sound,
        qa: ctx.qa,
        next_ticket_at_ms: st.next_ticket_at().map(|s| s * 1000),
        active: st.active.iter().map(|a| ActiveDto { id: a.id.clone(), until_ms: a.until * 1000, parts: a.parts.clone() }).collect(),
        boons: Boons {
            free_pulls: st.free_pulls,
            luck_pulls: st.luck_pulls,
            midas: st.midas,
            shields: st.shields,
            immune_until_ms: if st.immune_until > now { st.immune_until * 1000 } else { 0 },
        },
        recovered: core.recovered.take(),
        tiers: TIER_NAMES,
        catalog: CATALOG
            .iter()
            .filter(|e| game::available(e))
            .map(|e| EntryDto {
                id: e.id,
                name: e.name,
                tier: e.tier,
                kind: if e.is_curse() { "curse" } else { "blessing" },
                secs: if e.is_curse() { TIER_SECS[e.tier as usize] } else { 0 },
                desc: e.desc,
                one_in: game::one_in(e),
            })
            .collect(),
    }
}

/// Reverts every stacked curse. Returns true when anything was undone.
fn cure_all_locked(ctx: &Ctx, core: &mut Core) -> bool {
    let active = std::mem::take(&mut core.st.active);
    if active.is_empty() {
        return false;
    }
    for a in active {
        ctx.runner.revert(a.token, &a.restore);
    }
    ctx.save(&core.st);
    true
}

/// Reverts just the most-recently-applied curse, leaving any others stacked
/// beneath it running. Returns true when anything was undone.
fn cure_one_locked(ctx: &Ctx, core: &mut Core) -> bool {
    match core.st.active.pop() {
        Some(a) => {
            ctx.runner.revert(a.token, &a.restore);
            ctx.save(&core.st);
            true
        }
        None => false,
    }
}

/// Reverts whichever stacked curses have individually run out their own
/// timer, leaving any others (rolled more recently, or a longer tier) still
/// running. Returns true when anything was undone.
fn revert_expired_locked(ctx: &Ctx, core: &mut Core) -> bool {
    let now = now_secs();
    let (expired, still_active): (Vec<_>, Vec<_>) = std::mem::take(&mut core.st.active).into_iter().partition(|a| now >= a.until);
    core.st.active = still_active;
    if expired.is_empty() {
        return false;
    }
    for a in expired {
        ctx.runner.revert(a.token, &a.restore);
    }
    ctx.save(&core.st);
    true
}

/// Cures everything and tells the UI. Used by the hotkey, the tray, quitting
/// and a second launch with `--cure`.
fn cure_all_and_notify(app: &AppHandle) {
    let ctx = app.state::<Ctx>();
    let mut core = ctx.core.lock().unwrap();
    cure_all_locked(&ctx, &mut core);
    let snap = snapshot(&ctx, &mut core);
    drop(core);
    sync_overlay(app);
    let _ = app.emit("state", snap);
}

/// Cures just the most recent layer and tells the UI. Used by the
/// Ctrl+Alt+Z hotkey and the matching tray entry.
fn cure_one_and_notify(app: &AppHandle) {
    let ctx = app.state::<Ctx>();
    let mut core = ctx.core.lock().unwrap();
    cure_one_locked(&ctx, &mut core);
    let snap = snapshot(&ctx, &mut core);
    drop(core);
    sync_overlay(app);
    let _ = app.emit("state", snap);
}

fn show_main(app: &AppHandle, mode: &str) {
    // The UI sizes and positions the window for the mode, then shows it.
    let _ = app.emit_to("main", "mode", mode);
}

// ---------- overlay window (drawn curses) ----------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OverlayState {
    parts: Vec<String>,
    until_ms: u64,
    /// Cursor position inside the overlay, in CSS pixels.
    cursor: Option<(f64, f64)>,
}

/// The union of every stacked curse's overlay-drawn parts, so multiple
/// overlay effects pile up visually. `until_ms` is the soonest of their
/// expiries: good enough for the one overlay effect (Tunnel Vision) whose
/// animation scales with time remaining, even if it isn't the soonest to end.
fn overlay_parts(st: &game::State) -> (Vec<String>, u64) {
    let parts = st
        .active
        .iter()
        .flat_map(|a| a.parts.iter())
        .filter(|p| game::part(p).is_some_and(|d| d.overlay))
        .cloned()
        .collect();
    let until_ms = st.active.iter().map(|a| a.until * 1000).min().unwrap_or(0);
    (parts, until_ms)
}

/// Shows the overlay over the main window's monitor while a drawn curse is
/// active, and hides it otherwise.
fn sync_overlay(app: &AppHandle) {
    let Some(ov) = app.get_webview_window("overlay") else { return };
    let (parts, _) = overlay_parts(&app.state::<Ctx>().core.lock().unwrap().st);
    if parts.is_empty() {
        let _ = ov.hide();
        return;
    }
    let mon = app
        .get_webview_window("main")
        .and_then(|w| w.current_monitor().ok().flatten())
        .or_else(|| ov.primary_monitor().ok().flatten());
    if let Some(m) = mon {
        // One pixel short of the full height, so Windows doesn't treat it as a
        // full-screen app and hide the taskbar.
        let size = *m.size();
        let _ = ov.set_position(*m.position());
        let _ = ov.set_size(PhysicalSize::new(size.width, size.height.saturating_sub(1)));
    }
    let _ = ov.show();
    let _ = ov.set_ignore_cursor_events(true);
    let _ = ov.set_always_on_top(true);
}

#[tauri::command]
fn overlay_state(app: AppHandle, window: WebviewWindow) -> OverlayState {
    let (parts, until_ms) = overlay_parts(&app.state::<Ctx>().core.lock().unwrap().st);
    let cursor = (|| {
        let c = window.cursor_position().ok()?;
        let o = window.outer_position().ok()?;
        let sf = window.scale_factor().ok()?;
        Some(((c.x - o.x as f64) / sf, (c.y - o.y as f64) / sf))
    })();
    OverlayState { parts, until_ms, cursor }
}

// ---------- commands ----------

#[tauri::command]
fn get_state(ctx: State<Ctx>) -> Snapshot {
    let mut core = ctx.core.lock().unwrap();
    if core.st.regen(now_secs()) {
        ctx.save(&core.st);
    }
    snapshot(&ctx, &mut core)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PullDto {
    tier: u8,
    id: &'static str,
    forced_by_pity: bool,
    free: bool,
    state: Snapshot,
}

#[tauri::command]
fn pull(_app: AppHandle, ctx: State<Ctx>, force: Option<u8>) -> Result<PullDto, String> {
    // Curses stack: pulling again doesn't disturb whatever's already active.
    let dto = {
        let mut guard = ctx.core.lock().unwrap();
        let core = &mut *guard;
        let force = if ctx.qa { force } else { None };
        let out = core.st.pull(now_secs(), now_ms(), &mut core.rng, force).map_err(|e| match e {
            PullError::NoTickets => "no_tickets".to_string(),
        })?;
        core.pending = Some(out.id);
        ctx.save(&core.st);
        PullDto { tier: out.tier, id: out.id, forced_by_pity: out.forced_by_pity, free: out.free, state: snapshot(&ctx, core) }
    };
    Ok(dto)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimDto {
    /// "blessed", "blocked" or "cursed".
    result: &'static str,
    message: Option<String>,
    /// Parts of a stacked curse that couldn't be applied, as "part: reason".
    failed: Vec<String>,
    state: Snapshot,
}

/// Applies the revealed result: a blessing takes effect, a curse is applied
/// (unless a shield or immunity stops it).
#[tauri::command]
fn claim(app: AppHandle, ctx: State<Ctx>, id: String) -> Result<ClaimDto, String> {
    // The lock is released before the overlay update, which takes it again.
    let dto = {
        let mut guard = ctx.core.lock().unwrap();
        let core = &mut *guard;
        let id = match core.pending.take() {
            Some(p) if p == id => p,
            _ => return Err("that result wasn't rolled".into()),
        };
        let claim = core.st.claim(id, now_secs(), &mut core.rng).ok_or("unknown result")?;
        let (result, message, failed) = match claim {
            Claim::Blessed => ("blessed", None, Vec::new()),
            Claim::Blocked(msg) => ("blocked", Some(msg), Vec::new()),
            Claim::Curse(parts) => {
                // Stacks on top of whatever's already active, rather than
                // replacing it.
                let applied = match ctx.runner.apply(&parts) {
                    Ok(a) => a,
                    Err(e) => {
                        ctx.save(&core.st);
                        return Err(e);
                    }
                };
                let tier = game::entry(id).map(|e| e.tier as usize).unwrap_or(0);
                core.st.active.push(game::ActiveCurse {
                    id: id.into(),
                    parts: applied.parts,
                    until: now_secs() + TIER_SECS[tier],
                    restore: applied.restore,
                    token: applied.token,
                });
                let failed = applied.failed.into_iter().map(|(p, e)| format!("{p}: {e}")).collect();
                ("cursed", None, failed)
            }
        };
        ctx.save(&core.st);
        ClaimDto { result, message, failed, state: snapshot(&ctx, core) }
    };
    sync_overlay(&app);
    Ok(dto)
}

#[tauri::command]
fn cure_all(app: AppHandle, ctx: State<Ctx>) -> Snapshot {
    let mut core = ctx.core.lock().unwrap();
    cure_all_locked(&ctx, &mut core);
    let snap = snapshot(&ctx, &mut core);
    drop(core);
    sync_overlay(&app);
    snap
}

/// Cures just the most recently applied curse, from the UI.
#[tauri::command]
fn cure_one(app: AppHandle, ctx: State<Ctx>) -> Snapshot {
    let mut core = ctx.core.lock().unwrap();
    cure_one_locked(&ctx, &mut core);
    let snap = snapshot(&ctx, &mut core);
    drop(core);
    sync_overlay(&app);
    snap
}

#[tauri::command]
fn set_sound(ctx: State<Ctx>, on: bool) {
    let mut core = ctx.core.lock().unwrap();
    core.st.sound = on;
    ctx.save(&core.st);
}

#[tauri::command]
fn qa_refill(ctx: State<Ctx>) -> Result<Snapshot, String> {
    if !ctx.qa {
        return Err("QA mode is off".into());
    }
    let mut core = ctx.core.lock().unwrap();
    core.st.tickets = TICKET_CAP;
    ctx.save(&core.st);
    Ok(snapshot(&ctx, &mut core))
}

#[tauri::command]
fn qa_reset(app: AppHandle, ctx: State<Ctx>) -> Result<Snapshot, String> {
    if !ctx.qa {
        return Err("QA mode is off".into());
    }
    let mut core = ctx.core.lock().unwrap();
    cure_all_locked(&ctx, &mut core);
    let sound = core.st.sound;
    core.st = game::State { sound, ..Default::default() };
    ctx.save(&core.st);
    let snap = snapshot(&ctx, &mut core);
    drop(core);
    sync_overlay(&app);
    Ok(snap)
}

/// Sizes the borderless window to the app chrome, places it, then shows it.
/// With `anchor` (a screen point in physical pixels, the tray icon) it opens
/// next to that point like a flyout; otherwise it's centred.
#[tauri::command]
fn fit_window(window: WebviewWindow, w: f64, h: f64, anchor: Option<(f64, f64)>) -> Result<(), String> {
    let e = |e: tauri::Error| e.to_string();
    window.set_size(LogicalSize::new(w, h)).map_err(e)?;
    match anchor {
        Some((ax, ay)) => {
            let sf = window.scale_factor().map_err(e)?;
            let (pw, ph) = ((w * sf) as i32, (h * sf) as i32);
            let margin = (12.0 * sf) as i32;
            let mon = window
                .available_monitors()
                .map_err(e)?
                .into_iter()
                .find(|m| {
                    let (p, s) = (m.position(), m.size());
                    ax >= p.x as f64 && ax < (p.x + s.width as i32) as f64 && ay >= p.y as f64 && ay < (p.y + s.height as i32) as f64
                })
                .or(window.current_monitor().map_err(e)?);
            let (mx, my, mw, mh) = match &mon {
                Some(m) => (m.position().x, m.position().y, m.size().width as i32, m.size().height as i32),
                None => (0, 0, 1920, 1080),
            };
            // Open away from the edge the tray is on (usually the bottom right).
            let x = if ax > (mx + mw / 2) as f64 { ax as i32 - pw + margin } else { ax as i32 - margin };
            let y = if ay > (my + mh / 2) as f64 { ay as i32 - ph - margin * 2 } else { ay as i32 + margin * 2 };
            let x = x.clamp(mx + margin, (mx + mw - pw - margin).max(mx));
            let y = y.clamp(my + margin, (my + mh - ph - margin).max(my));
            window.set_position(PhysicalPosition::new(x, y)).map_err(e)?;
            let _ = window.set_always_on_top(true);
        }
        None => {
            let _ = window.set_always_on_top(false);
            window.center().map_err(e)?;
        }
    }
    window.show().map_err(e)?;
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}

/// Grows the (transparent) window to cover its monitor so the reveal can spill
/// over the desktop. Returns where the app chrome must be drawn, in CSS px, so
/// it doesn't appear to move.
#[tauri::command]
fn fx_expand(window: WebviewWindow, ctx: State<Ctx>) -> Result<(f64, f64), String> {
    let e = |e: tauri::Error| e.to_string();
    let pos = window.outer_position().map_err(e)?;
    let size = window.outer_size().map_err(e)?;
    let mon = window.current_monitor().map_err(e)?.ok_or("no monitor")?;
    let (mp, ms) = (*mon.position(), *mon.size());
    ctx.geom.lock().unwrap().get_or_insert((pos, size));
    window.set_always_on_top(true).map_err(e)?;
    window.set_position(mp).map_err(e)?;
    window.set_size(ms).map_err(e)?;
    let sf = window.scale_factor().map_err(e)?;
    Ok(((pos.x - mp.x) as f64 / sf, (pos.y - mp.y) as f64 / sf))
}

/// Puts the window back after a reveal. Only the tray flyout stays on top.
#[tauri::command]
fn fx_restore(window: WebviewWindow, ctx: State<Ctx>, on_top: bool) -> Result<(), String> {
    let e = |e: tauri::Error| e.to_string();
    if let Some((pos, size)) = ctx.geom.lock().unwrap().take() {
        window.set_size(size).map_err(e)?;
        window.set_position(pos).map_err(e)?;
    }
    window.set_always_on_top(on_top).map_err(e)
}

#[tauri::command]
fn hide_to_tray(window: WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn quit(app: AppHandle) {
    cure_all_and_notify(&app);
    app.exit(0);
}

/// UI errors end up in stderr (and in the terminal when run from one).
#[tauri::command]
fn ui_log(msg: String) {
    eprintln!("[ui] {msg}");
}

// ---------- tray ----------

fn tooltip(st: &game::State) -> String {
    let mut t = format!("gachaos · {} tickets", st.tickets);
    if let Some(top) = st.active.last() {
        let left = top.until.saturating_sub(now_secs());
        let name = game::entry(&top.id).map(|e| e.name).unwrap_or("Curse");
        let more = st.active.len() - 1;
        let suffix = if more > 0 { format!(" +{more} more (Ctrl+Alt+Z cures this one)") } else { String::new() };
        t += &format!("\n{name}: {}:{:02} left{suffix} (Ctrl+Alt+P cures all)", left / 60, left % 60);
    }
    t
}

// ---------- watchdog ----------

/// `gachaos --watchdog <pid> <save>`: a tiny helper started by the app. When
/// the app's process ends for any reason (including being killed), it undoes
/// a curse the app didn't get the chance to.
#[cfg(windows)]
fn run_watchdog(pid: u32, save_path: PathBuf) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE};
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else { return };
        WaitForSingleObject(h, INFINITE);
        let _ = CloseHandle(h);
    }
    std::thread::sleep(Duration::from_millis(300)); // let a clean quit finish saving
    let mut st = save::load(&save_path);
    let active = std::mem::take(&mut st.active);
    if !active.is_empty() {
        for a in active {
            curses::revert_saved(&a.restore);
        }
        save::store(&save_path, &st);
    }
}

#[cfg(windows)]
fn spawn_watchdog(save_path: &std::path::Path) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    let Ok(exe) = std::env::current_exe() else { return };
    let res = std::process::Command::new(exe)
        .arg("--watchdog")
        .arg(std::process::id().to_string())
        .arg(save_path)
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn();
    if let Err(e) = res {
        eprintln!("gachaos: couldn't start the watchdog: {e}");
    }
}

// ---------- startup ----------

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let save_path = dir.join("save.json");
    let mut st = save::load(&save_path);
    let runner = curses::Runner::default();

    // If the app died with curses active, undo them now: we can't know how
    // much time was left, so leaving them on would be worse.
    let recovered_curses = std::mem::take(&mut st.active);
    let recovered = match recovered_curses.len() {
        0 => None,
        _ => {
            let names: Vec<_> = recovered_curses
                .iter()
                .map(|a| {
                    curses::revert_saved(&a.restore);
                    game::entry(&a.id).map(|c| c.name.to_string()).unwrap_or_else(|| a.id.clone())
                })
                .collect();
            Some(names.join(", "))
        }
    };
    save::store(&save_path, &st);

    // `gachaos --cure` with no copy running: recovery above did the job.
    if std::env::args().any(|a| a == "--cure") {
        let handle = app.handle().clone();
        std::thread::spawn(move || handle.exit(0));
        return Ok(());
    }

    #[cfg(windows)]
    spawn_watchdog(&save_path);

    let qa = std::env::args().any(|a| a == "--qa") || std::env::var("GACHAOS_QA").is_ok_and(|v| v == "1");
    app.manage(Ctx {
        core: Mutex::new(Core { st, pending: None, recovered, rng: fastrand::Rng::new() }),
        runner,
        save_path,
        qa,
        geom: Mutex::new(None),
    });

    // Overlay for drawn curses: transparent, click-through, always on top.
    let overlay = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
        .title("gachaos overlay")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .build()?;
    // Click-through is set each time it's shown (sync_overlay): GTK can't
    // change the input region of a window that was never shown.
    drop(overlay);

    // Tray: the app lives here when its window is closed.
    let open = MenuItem::with_id(app, "open", "Open gachaos", true, None::<&str>)?;
    let cure = MenuItem::with_id(app, "cure", "Cure all curses (Ctrl+Alt+P)", true, None::<&str>)?;
    let cure_one = MenuItem::with_id(app, "cure_one", "Cure most recent curse (Ctrl+Alt+Z)", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit (cures curses)", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &cure, &cure_one, &quit])?;
    let mut tray = TrayIconBuilder::with_id("tray")
        .tooltip("gachaos")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => show_main(app, "full"),
            "cure" => cure_all_and_notify(app),
            "cure_one" => cure_one_and_notify(app),
            "quit" => {
                cure_all_and_notify(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, position, .. } = e {
                let app = tray.app_handle();
                match app.get_webview_window("main") {
                    Some(w) if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) => {
                        let _ = app.emit_to("main", "flyout-toggle", ());
                    }
                    _ => {
                        let _ = app.emit_to("main", "flyout", (position.x, position.y));
                    }
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    // Panic hotkeys, reachable even while the window is hidden: P cures
    // everything stacked, Z peels off just the most recent layer.
    let panic_key = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyP);
    if let Err(e) = app.global_shortcut().register(panic_key) {
        eprintln!("gachaos: couldn't register Ctrl+Alt+P: {e}");
    }
    let layer_key = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyZ);
    if let Err(e) = app.global_shortcut().register(layer_key) {
        eprintln!("gachaos: couldn't register Ctrl+Alt+Z: {e}");
    }

    // The UI shows the window once it has sized it. If that never happens
    // (a broken webview, say), show it anyway rather than run invisibly.
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(5));
        if let Some(w) = handle.get_webview_window("main") {
            if !w.is_visible().unwrap_or(true) {
                eprintln!("gachaos: UI didn't show the window in time; showing it anyway");
                let _ = w.show();
            }
        }
    });

    // Expiry and the tray tooltip: curses wear off on time even while hidden.
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        let mut last_tip = String::new();
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let ctx = handle.state::<Ctx>();
            let mut core = ctx.core.lock().unwrap();
            let snap = revert_expired_locked(&ctx, &mut core).then(|| snapshot(&ctx, &mut core));
            let tip = tooltip(&core.st);
            drop(core);
            if let Some(snap) = snap {
                sync_overlay(&handle);
                let _ = handle.emit("state", snap);
            }
            if tip != last_tip {
                if let Some(t) = handle.tray_by_id("tray") {
                    let _ = t.set_tooltip(Some(&tip));
                }
                last_tip = tip;
            }
        }
    });
    Ok(())
}

fn main() {
    #[cfg(windows)]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("--watchdog") {
            if let (Some(pid), Some(path)) = (args.get(2).and_then(|p| p.parse().ok()), args.get(3)) {
                run_watchdog(pid, PathBuf::from(path));
            }
            return;
        }
    }

    let app = tauri::Builder::default()
        // Must come first: a second launch hands its arguments to the running
        // copy and exits. `gachaos --cure` cures through the running copy.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.iter().any(|a| a == "--cure") {
                cure_all_and_notify(app);
            } else {
                show_main(app, "full");
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let layer_key = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyZ);
                    if *shortcut == layer_key {
                        cure_one_and_notify(app);
                    } else {
                        cure_all_and_notify(app);
                    }
                })
                .build(),
        )
        .setup(setup)
        .on_window_event(|window, event| {
            // Let the UI decide: with a curse active it asks whether to cure first.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "main" {
                    let _ = window.emit_to("main", "close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state, pull, claim, cure_all, cure_one, set_sound, qa_refill, qa_reset, fit_window, fx_expand, fx_restore,
            hide_to_tray, quit, ui_log, overlay_state
        ])
        .build(tauri::generate_context!())
        .expect("error while building gachaos");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            // Never leave a curse behind.
            if let Some(ctx) = app.try_state::<Ctx>() {
                let mut core = ctx.core.lock().unwrap();
                cure_all_locked(&ctx, &mut core);
            }
        }
    });
}
