// gachaos: a drab legacy-looking exe that rolls a slot machine and curses
// your desktop. The UI (ui/index.html) handles the roll and the effects; this
// side owns the save, the odds, the real Windows curses, the tray icon and
// the Ctrl+Alt+P panic hotkey.
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
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, RunEvent, State, WebviewWindow, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use game::{PullError, CURSES, PITY, TICKET_CAP, TIER_NAMES};

struct Core {
    st: game::State,
    /// The curse rolled by the last pull, waiting for its reveal to finish.
    pending: Option<&'static str>,
    /// Name of a curse undone at startup because the app had crashed.
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
struct CurseDto {
    id: &'static str,
    name: &'static str,
    tier: u8,
    secs: u64,
    desc: &'static str,
    one_in: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ActiveDto {
    id: String,
    until_ms: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    tickets: u32,
    cap: u32,
    pity: u32,
    pity_threshold: u32,
    pulls: u32,
    counts: [u32; 4],
    best: i32,
    best_name: String,
    history: Vec<game::PullRecord>,
    sound: bool,
    qa: bool,
    next_ticket_at_ms: Option<u64>,
    active: Option<ActiveDto>,
    recovered: Option<String>,
    tiers: [&'static str; 4],
    catalog: Vec<CurseDto>,
}

fn snapshot(ctx: &Ctx, core: &mut Core) -> Snapshot {
    let st = &core.st;
    Snapshot {
        tickets: st.tickets,
        cap: TICKET_CAP,
        pity: st.pity,
        pity_threshold: PITY,
        pulls: st.pulls,
        counts: st.counts,
        best: st.best,
        best_name: st.best_name.clone(),
        history: st.history.clone(),
        sound: st.sound,
        qa: ctx.qa,
        next_ticket_at_ms: st.next_ticket_at().map(|s| s * 1000),
        active: st.active.as_ref().map(|a| ActiveDto { id: a.id.clone(), until_ms: a.until * 1000 }),
        recovered: core.recovered.take(),
        tiers: TIER_NAMES,
        catalog: CURSES
            .iter()
            .map(|c| CurseDto { id: c.id, name: c.name, tier: c.tier, secs: c.secs, desc: c.desc, one_in: game::one_in(c) })
            .collect(),
    }
}

/// Reverts the active curse, if any. Returns true when something was undone.
fn cure_locked(ctx: &Ctx, core: &mut Core) -> bool {
    match core.st.active.take() {
        Some(a) => {
            ctx.runner.revert(&a.restore);
            ctx.save(&core.st);
            true
        }
        None => false,
    }
}

/// Cures everything and tells the UI. Used by the hotkey, the tray and quitting.
fn cure_all_and_notify(app: &AppHandle) {
    let ctx = app.state::<Ctx>();
    let mut core = ctx.core.lock().unwrap();
    cure_locked(&ctx, &mut core);
    let snap = snapshot(&ctx, &mut core);
    drop(core);
    let _ = app.emit("state", snap);
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
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

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PullDto {
    tier: u8,
    curse_id: &'static str,
    forced_by_pity: bool,
    state: Snapshot,
}

#[tauri::command]
fn pull(app: AppHandle, ctx: State<Ctx>, force: Option<u8>) -> Result<PullDto, String> {
    let mut core = ctx.core.lock().unwrap();
    // One curse at a time: a new pull lifts the old one first.
    if cure_locked(&ctx, &mut core) {
        let _ = app.emit("cured", ());
    }
    let force = if ctx.qa { force } else { None };
    let core = &mut *core;
    let out = core.st.pull(now_secs(), now_ms(), &mut core.rng, force).map_err(|e| match e {
        PullError::NoTickets => "no_tickets".to_string(),
    })?;
    core.pending = Some(out.id);
    ctx.save(&core.st);
    Ok(PullDto { tier: out.tier, curse_id: out.id, forced_by_pity: out.forced_by_pity, state: snapshot(&ctx, core) })
}

#[tauri::command]
fn apply_curse(ctx: State<Ctx>, id: String) -> Result<Snapshot, String> {
    let mut core = ctx.core.lock().unwrap();
    // Only the curse that was actually rolled can be applied.
    let def = match core.pending.take() {
        Some(p) if p == id => game::curse(p).ok_or("unknown curse")?,
        _ => return Err("that curse wasn't rolled".into()),
    };
    cure_locked(&ctx, &mut core);
    let restore = ctx.runner.apply(def.id)?;
    core.st.active = Some(game::ActiveCurse { id: def.id.into(), until: now_secs() + def.secs, restore });
    ctx.save(&core.st);
    Ok(snapshot(&ctx, &mut core))
}

#[tauri::command]
fn cure_all(ctx: State<Ctx>) -> Snapshot {
    let mut core = ctx.core.lock().unwrap();
    cure_locked(&ctx, &mut core);
    snapshot(&ctx, &mut core)
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
fn qa_reset(ctx: State<Ctx>) -> Result<Snapshot, String> {
    if !ctx.qa {
        return Err("QA mode is off".into());
    }
    let mut core = ctx.core.lock().unwrap();
    cure_locked(&ctx, &mut core);
    let sound = core.st.sound;
    core.st = game::State { sound, ..Default::default() };
    ctx.save(&core.st);
    Ok(snapshot(&ctx, &mut core))
}

/// Sizes the borderless window to the app chrome, then shows it.
#[tauri::command]
fn fit_window(window: WebviewWindow, w: f64, h: f64) -> Result<(), String> {
    window.set_size(LogicalSize::new(w, h)).map_err(|e| e.to_string())?;
    window.center().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
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

#[tauri::command]
fn fx_restore(window: WebviewWindow, ctx: State<Ctx>) -> Result<(), String> {
    if let Some((pos, size)) = ctx.geom.lock().unwrap().take() {
        let e = |e: tauri::Error| e.to_string();
        window.set_size(size).map_err(e)?;
        window.set_position(pos).map_err(e)?;
        window.set_always_on_top(false).map_err(e)?;
    }
    Ok(())
}

/// UI errors end up in stderr (and in the terminal when run from one).
#[tauri::command]
fn ui_log(msg: String) {
    eprintln!("[ui] {msg}");
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

// ---------- startup ----------

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let save_path = dir.join("save.json");
    let mut st = save::load(&save_path);
    let runner = curses::Runner::default();

    // If the app died while a curse was active, undo it now: we can't know
    // how much time was left, so leaving it on would be worse.
    let mut recovered = None;
    if let Some(a) = st.active.take() {
        runner.revert(&a.restore);
        recovered = game::curse(&a.id).map(|c| c.name.to_string());
        save::store(&save_path, &st);
    }

    let qa = std::env::args().any(|a| a == "--qa") || std::env::var("GACHAOS_QA").is_ok_and(|v| v == "1");
    app.manage(Ctx {
        core: Mutex::new(Core { st, pending: None, recovered, rng: fastrand::Rng::new() }),
        runner,
        save_path,
        qa,
        geom: Mutex::new(None),
    });

    // Tray: the app lives here when its window is closed.
    let open = MenuItem::with_id(app, "open", "Open gachaos", true, None::<&str>)?;
    let cure = MenuItem::with_id(app, "cure", "Cure all curses (Ctrl+Alt+P)", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &cure, &quit])?;
    let mut tray = TrayIconBuilder::with_id("tray")
        .tooltip("gachaos")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => show_main(app),
            "cure" => cure_all_and_notify(app),
            "quit" => {
                cure_all_and_notify(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    // Panic hotkey, reachable even while the window is hidden.
    let panic_key = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyP);
    if let Err(e) = app.global_shortcut().register(panic_key) {
        eprintln!("gachaos: couldn't register Ctrl+Alt+P: {e}");
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

    // Expiry: curses wear off on time even while the window is hidden.
    let handle = app.handle().clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(500));
        let ctx = handle.state::<Ctx>();
        let mut core = ctx.core.lock().unwrap();
        let expired = core.st.active.as_ref().is_some_and(|a| now_secs() >= a.until);
        if expired && cure_locked(&ctx, &mut core) {
            let snap = snapshot(&ctx, &mut core);
            drop(core);
            let _ = handle.emit("state", snap);
        }
    });
    Ok(())
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        cure_all_and_notify(app);
                    }
                })
                .build(),
        )
        .setup(setup)
        .on_window_event(|window, event| {
            // The window's close goes to the tray, like the spec says.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state, pull, apply_curse, cure_all, set_sound, qa_refill, qa_reset, fit_window, fx_expand, fx_restore,
            hide_to_tray, quit, ui_log
        ])
        .build(tauri::generate_context!())
        .expect("error while building gachaos");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            // Never leave a curse behind.
            let ctx = app.state::<Ctx>();
            let mut core = ctx.core.lock().unwrap();
            cure_locked(&ctx, &mut core);
        }
    });
}
