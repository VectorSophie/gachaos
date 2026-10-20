# Cross-Platform Curse Pool & Fibonacci-Sized Tiers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebalance `TIER_ODDS` to clean ratios, grow/shrink each tier's curse and blessing count on a Fibonacci-flavored taper, and give the three cross-platform curses (wallpaper, volume) real Linux and macOS implementations instead of silently no-opping through `stub.rs`.

**Architecture:** All catalog/odds/availability logic lives in `src/game.rs` (pure, platform-independent, unit-tested everywhere). Platform-specific curse *effects* live one file per OS under `src/curses/` (`win32.rs`, new `linux.rs`, new `macos.rs`, `stub.rs` as the fallback), selected at compile time by `cfg` in `src/curses/mod.rs`. The frontend (`ui/index.html`, `ui/overlay.html`) is data-driven from the backend catalog except for the overlay's per-effect rendering code, which needs one new block per new overlay curse.

**Tech Stack:** Rust (Tauri 2 backend), vanilla JS canvas (overlay window), no new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-07-gachaos-redesign.md` (see "Update 2026-10-20: cross-platform pool and Fibonacci tier sizes")

## Global Constraints

- `TIER_ODDS` becomes `[432, 216, 108, 36, 12, 3]` (ratios 2×, 2×, 3×, 3×, 4× going up the tiers). It no longer sums to 1000 — anything that assumed that must be fixed, not patched around.
- Curse counts per tier (Common→Mythic): `13, 8, 5, 3, 3, 3`. Blessing counts: `5, 3, 2, 1, 1, 1`.
- `Part.overlay` (works everywhere via the click-through window) stays; add `Part.cross_platform` for non-overlay parts with a real native implementation on every OS (today: wallpaper, volume). Everything else non-overlay stays Windows-only — no Linux/macOS pointer-speed or window-distortion API exists.
- No new Cargo dependencies. Linux/macOS wallpaper/volume shell out to `gsettings`/`pactl` and `osascript` respectively via `std::process::Command`.
- Every existing passing test must still pass; tests whose expected values depend on the old counts/weights get updated, not deleted.

---

### Task 1: Rebalance odds and resize the catalog

**Files:**
- Modify: `src/game.rs` (whole file's data section: `TIER_ODDS`, `Part`/`PARTS`, `CATALOG`, `fixed_parts`, `one_in`, and the affected `#[cfg(test)]` tests)

**Interfaces:**
- Consumes: nothing new.
- Produces: `Part.cross_platform: bool` (read by `part_supported`, and by Task 2's `Runner::apply` dispatch indirectly through `part_supported`); the `both(id, slot)` `Part` constructor (used nowhere else in this plan, but documents the new category for future entries).

This task rewrites `src/game.rs`'s data section in one pass — the intermediate "weights changed but catalog counts didn't" state isn't independently meaningful, so there's no useful halfway checkpoint.

- [ ] **Step 1: Update the production code**

Replace the `TIER_ODDS` constant and its doc comment:

```rust
/// Common, Uncommon, Rare, Epic, Legendary, Mythic — relative weights
/// (ratios of 2x, 2x, 3x, 3x, 4x going up the tiers). Don't need to sum to
/// any particular total; `one_in` and `roll_tier` derive everything from
/// the array itself.
pub const TIER_ODDS: [u32; TIERS] = [432, 216, 108, 36, 12, 3];
```

Replace the `Part` struct and its constructors:

```rust
pub struct Part {
    pub id: &'static str,
    pub slot: &'static str,
    pub overlay: bool,
    /// Non-overlay parts with a real implementation on every OS (today: wallpaper, volume).
    pub cross_platform: bool,
}

const fn os(id: &'static str, slot: &'static str) -> Part {
    Part { id, slot, overlay: false, cross_platform: false }
}
const fn both(id: &'static str, slot: &'static str) -> Part {
    Part { id, slot, overlay: false, cross_platform: true }
}
const fn ov(id: &'static str, slot: &'static str) -> Part {
    Part { id, slot, overlay: true, cross_platform: false }
}
```

Replace the `PARTS` array:

```rust
pub const PARTS: &[Part] = &[
    os("giant", "cursor-art"),
    os("wrong", "buttons"),
    os("sloth", "speed"),
    os("trails", "trails"),
    ov("pal", "pal"),
    os("turbo", "speed"),
    os("butter", "dblclick"),
    both("whisper", "volume"),
    os("wobble", "window"),
    ov("trail", "trail"),
    os("jitter", "cursor-move"),
    both("wall", "wallpaper"),
    os("teleport", "cursor-move"),
    ov("snow", "snow"),
    ov("fog", "fog"),
    os("upside", "display"),
    both("beep", "volume"),
    os("negative", "colour"),
    os("silent", "colour"),
    ov("spotlight", "spot"),
    os("sideways", "display"),
    os("disco", "wallpaper"),
    os("tooclose", "colour"),
    ov("confetti", "confetti"),
    ov("ghost", "ghost"),
    ov("static", "fuzz"),
    ov("matrix", "matrix"),
    ov("eyes", "eyes"),
    ov("firefly", "firefly"),
    ov("crack", "crack"),
    // redpulse and doomclock only ever appear inside Mythic combos; bloodred
    // and eclipse do too, but also back their own standalone curses
    // (Crimson Tint, Tunnel Vision).
    os("bloodred", "colour"),
    ov("redpulse", "pulse"),
    ov("doomclock", "clock"),
    ov("eclipse", "spot"),
];
```

Replace `part_supported`:

```rust
/// Whether this platform can run a part: overlay and cross-platform parts
/// everywhere, Windows-only parts on Windows.
pub fn part_supported(id: &str) -> bool {
    part(id).is_some_and(|p| p.overlay || p.cross_platform || cfg!(windows))
}
```

Replace `fixed_parts` (drop the removed `bloodmoon` combo and the now-standalone `eclipse`):

```rust
/// The fixed parts of a curse (Pandemonium is chosen at claim time).
fn fixed_parts(id: &str) -> &'static [&'static str] {
    match id {
        "possession" => &["upside", "negative", "jitter", "redpulse"],
        "doom" => &["doomclock", "beep", "teleport"],
        _ => &[],
    }
}
```

Replace the doc comment and body of `CATALOG`:

```rust
/// Entry counts taper like Fibonacci, shrinking from Common to Mythic:
/// curses 13/8/5/3/3/3, blessings 5/3/2/1/1/1.
pub const CATALOG: &[Entry] = &[
    // Common
    curse("giant", "Giant Cursor", 0, "Your mouse cursor is four times its normal size."),
    curse("wrong", "Wrong-Handed", 0, "Your left and right mouse buttons are swapped."),
    curse("sloth", "Sloth Mouse", 0, "Your mouse pointer crawls at its slowest speed."),
    curse("trails", "Long Tail", 0, "Your pointer drags a long trail of ghost cursors."),
    curse("pal", "Pixel Pal", 0, "A small pixel creature wanders along the bottom of your screen."),
    curse("negative", "Negative Space", 0, "Every colour on your screen is inverted."),
    curse("silent", "Silent Film", 0, "Your whole screen turns black and white."),
    curse("bloodred", "Crimson Tint", 0, "Your whole screen is washed in deep red."),
    curse("static", "Static Fuzz", 0, "Bursts of TV static sweep across your screen."),
    curse("matrix", "Matrix Drip", 0, "Columns of glowing characters cascade down your screen."),
    curse("eyes", "Eyes Everywhere", 0, "A scatter of eyes appears on your screen, watching your cursor."),
    curse("firefly", "Firefly Dusk", 0, "Glowing fireflies drift and pulse across your screen."),
    curse("crack", "Crack Spreads", 0, "A crack appears on your screen and keeps spreading."),
    bless("pocket", "Pocket Change", 0, "+1 ticket.", Boon::Tickets(1)),
    bless("freespin", "Free Spin", 0, "Your next pull costs nothing.", Boon::FreePulls(1)),
    bless("talisman", "Talisman", 0, "Blocks the next curse you roll.", Boon::Shield(1)),
    bless("midas", "Penny From Heaven", 0, "Your next pull is free and guaranteed Epic or better.", Boon::Midas(1)),
    bless("rabbitfoot", "Rabbit's Foot", 0, "Your next pull has double the odds of Rare or better.", Boon::Luck(1)),
    // Uncommon
    curse("turbo", "Turbo Mouse", 1, "Your mouse pointer moves at its fastest speed."),
    curse("butter", "Butterfingers", 1, "Double-clicks have to be lightning fast to count."),
    curse("whisper", "Whisper Mode", 1, "Your volume drops to 10%."),
    curse("wobble", "Window Wobble", 1, "Whatever window you're using won't sit still."),
    curse("trail", "Rainbow Trail", 1, "A pixel rainbow follows your cursor everywhere."),
    curse("sideways", "Sideways Vision", 1, "Your screen is rotated 90 degrees."),
    curse("confetti", "Confetti Storm", 1, "Confetti pours over your desktop without stopping."),
    curse("eclipse", "Tunnel Vision", 1, "Everything goes dark except a shrinking circle around your cursor."),
    bless("spare", "Spare Change", 1, "+2 tickets.", Boon::Tickets(2)),
    bless("clover", "Four-Leaf Clover", 1, "Your next 3 pulls have double the odds of Rare or better.", Boon::Luck(3)),
    bless("jackpot", "Mini Jackpot", 1, "+3 tickets.", Boon::Tickets(3)),
    // Rare
    curse("jitter", "Jitterbug Cursor", 2, "Your cursor keeps drifting away from where you point."),
    curse("wall", "Wallpaper Heist", 2, "Someone replaced your wallpaper."),
    curse("teleport", "Teleporter", 2, "Your cursor jumps to a random spot every few seconds."),
    curse("snow", "Pixel Snow", 2, "Snow falls across your screen and piles up at the bottom."),
    curse("fog", "Fog Bank", 2, "Dithered fog creeps in from the edges of your screen."),
    bless("lucky", "Lucky Break", 2, "+4 tickets.", Boon::Tickets(4)),
    bless("umbrella", "Umbrella", 2, "Blocks the next curse you roll.", Boon::Shield(1)),
    // Epic
    curse("upside", "Upside-Down World", 3, "Your screen is flipped 180 degrees."),
    curse("beep", "Beep Storm", 3, "Your volume goes up and random system beeps play."),
    curse("spotlight", "Spotlight", 3, "Everything goes dark except a small circle around your cursor."),
    bless("windfall", "Windfall", 3, "+8 tickets.", Boon::Tickets(8)),
    // Legendary
    curse("disco", "Disco Inferno", 4, "Your desktop flashes through disco colours."),
    curse("tooclose", "Too Close", 4, "Your screen is zoomed in and follows your cursor around."),
    curse("ghost", "Ghost Hunt", 4, "A pixel ghost chases your cursor. Don't let it catch you."),
    bless("sanctuary", "Sanctuary", 4, "No curse can touch you for the next 30 minutes.", Boon::Immunity(1800)),
    // Mythic
    curse("possession", "The Possession", 5, "Upside-Down World, Negative Space and Jitterbug Cursor, all at once."),
    curse("pandemonium", "Pandemonium", 5, "One random curse from every tier below Mythic, all at once."),
    curse("doom", "Doom Clock", 5, "A giant countdown takes over your screen while Beep Storm and Teleporter run."),
    bless("grace", "Divine Grace", 5, "Tickets refilled to full and no curses for 2 hours.", Boon::Grace),
];
```

Replace `one_in` (it hardcoded `1000.0`, assuming `TIER_ODDS` summed to 1000 — it no longer does):

```rust
/// Odds of one specific entry, as "1 in N".
pub fn one_in(e: &Entry) -> u32 {
    let total: u32 = TIER_ODDS.iter().sum();
    let same_tier = CATALOG.iter().filter(|d| d.tier == e.tier && available(d)).count() as f64;
    (total as f64 * same_tier / TIER_ODDS[e.tier as usize] as f64).round() as u32
}
```

- [ ] **Step 2: Update the tests in the same file's `#[cfg(test)] mod tests`**

Replace `catalog_has_five_curses_and_two_blessings_per_tier`:

```rust
#[test]
fn catalog_counts_follow_the_fibonacci_taper() {
    let want_curses = [13, 8, 5, 3, 3, 3];
    let want_bless = [5, 3, 2, 1, 1, 1];
    for t in 0..TIERS as u8 {
        let curses = CATALOG.iter().filter(|e| e.tier == t && e.is_curse()).count();
        let blessings = CATALOG.iter().filter(|e| e.tier == t && !e.is_curse()).count();
        assert_eq!((curses, blessings), (want_curses[t as usize], want_bless[t as usize]), "tier {t}");
    }
    let ids: HashSet<_> = CATALOG.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), CATALOG.len(), "duplicate ids");
}
```

Remove the `if !cfg!(windows) { return; }` guard from `pandemonium_takes_one_curse_from_each_lower_tier` — every tier 0..4 now has at least one overlay/cross-platform curse available off Windows too:

```rust
#[test]
fn pandemonium_takes_one_curse_from_each_lower_tier() {
    let mut rng = fastrand::Rng::with_seed(8);
    let parts = curse_parts("pandemonium", &mut rng);
    let tiers: HashSet<_> = parts.iter().filter_map(|p| entry(p)).map(|e| e.tier).collect();
    assert_eq!(tiers, (0..5).collect());
}
```

In `odds_are_close_to_the_table_and_both_pities_cap_the_drought`, replace the want/tolerance table (new per-mille targets are `weight / 807 * 1000`):

```rust
for (t, (want, tol)) in [(535.3, 35.0), (267.7, 16.0), (133.8, 10.0), (44.6, 12.0), (14.9, 8.0), (3.7, 3.0)].iter().enumerate() {
    assert!((pct(t) - want).abs() < *tol, "tier {t}: {} per mille, want {want}", pct(t));
}
```

Replace `one_in_matches_the_chances_table` (new counts per tier on Windows are 18/11/7/4/4/4; values are `807 * count / weight`, rounded):

```rust
#[test]
fn one_in_matches_the_chances_table() {
    if !cfg!(windows) {
        return; // off Windows, fewer curses are available per tier
    }
    let get = |id| one_in(entry(id).unwrap());
    assert_eq!([get("giant"), get("turbo"), get("jitter"), get("upside"), get("ghost"), get("doom")], [34, 41, 52, 90, 269, 1076]);
}
```

In `blessings_take_effect`, update for the demoted/renamed blessings (Mini Jackpot gives +3 not +16; Talisman now blocks 1 curse not 2; Midas Touch — now "Penny From Heaven" — is a single guaranteed pull, not 5):

```rust
#[test]
fn blessings_take_effect() {
    let mut rng = fastrand::Rng::with_seed(5);
    let mut st = State { tickets: 0, ..State::default() };
    let now = 1000;
    assert_eq!(st.claim("jackpot", now, &mut rng), Some(Claim::Blessed));
    assert_eq!(st.tickets, 3);

    // Free pulls don't spend tickets.
    st.claim("freespin", now, &mut rng);
    assert!(st.pull(now, 0, &mut rng, None).unwrap().free);
    assert_eq!(st.tickets, 3);

    // A shield blocks one curse, then curses land again.
    st.claim("talisman", now, &mut rng);
    assert_eq!(
        st.claim("upside", now, &mut rng),
        Some(Claim::Blocked("Your shield blocked it. That was your last one.".into()))
    );
    assert_eq!(st.claim("upside", now, &mut rng), Some(Claim::Curse(vec!["upside"])));

    // Immunity blocks without spending shields, until it runs out.
    st.claim("umbrella", now, &mut rng);
    st.claim("sanctuary", now, &mut rng);
    assert!(matches!(st.claim("giant", now + 10, &mut rng), Some(Claim::Blocked(_))));
    assert_eq!(st.shields, 1);
    assert!(matches!(st.claim("giant", now + 1800, &mut rng), Some(Claim::Blocked(_))));
    assert_eq!(st.shields, 0);

    // Penny From Heaven: one guaranteed free Epic-or-better pull.
    let mut st = State { tickets: 0, ..State::default() };
    st.claim("midas", now, &mut rng);
    let o = st.pull(now, 0, &mut rng, None).unwrap();
    assert!(o.free && o.tier >= 3, "{o:?}");
    assert_eq!(st.pull(now, 0, &mut rng, None), Err(PullError::NoTickets));

    // Grace refills and protects.
    let mut st = State { tickets: 2, ..State::default() };
    st.claim("grace", now, &mut rng);
    assert_eq!(st.tickets, TICKET_CAP);
    assert_eq!(st.immune_until, now + 7200);
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test`
Expected: all tests in `game.rs` pass. If `odds_are_close_to_the_table_and_both_pities_cap_the_drought` flakes (it's statistical over 400k pulls), widen the relevant tolerance slightly and rerun — don't change the `want` values, only `tol`.

- [ ] **Step 4: Commit**

```bash
git add src/game.rs
git commit -m "$(cat <<'EOF'
Rebalance tier weights to clean ratios, taper tier sizes on Fibonacci

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Real Linux and macOS implementations for wallpaper and volume curses

**Files:**
- Modify: `src/curses/mod.rs` (platform dispatch, `heist_bmp`'s `dead_code` gate)
- Create: `src/curses/linux.rs`
- Create: `src/curses/macos.rs`

**Interfaces:**
- Consumes: `Restore`, `Worker` (from `src/curses/mod.rs`); `heist_bmp` (from `src/curses/mod.rs`); `game::part_supported`/`Part.cross_platform` (from Task 1 — gates which ids these modules are ever called with: only `"wall"`, `"whisper"`, `"beep"`).
- Produces: `pub fn apply(id: &str, r: &mut Restore) -> Result<Option<Worker>, String>` and `pub fn revert(r: &Restore)`, matching `win32.rs`'s and `stub.rs`'s existing signatures exactly (same `platform` alias contract `Runner` already dispatches through).

**Note:** this machine is Windows, so `cargo build`/`cargo test` here only compiles `win32.rs`. `linux.rs`/`macos.rs` are cfg-gated out locally and are first compiled by CI's `linux`/`macos` jobs on push. Read them carefully; there's no local compiler feedback loop for this task.

- [ ] **Step 1: Widen `heist_bmp`'s dead-code gate in `src/curses/mod.rs`**

It's currently only used by `win32.rs`'s Wallpaper Heist; now `linux.rs`/`macos.rs` use it too, so it's never truly dead except on the `stub` fallback:

```rust
/// A loud 24-bit BMP for Wallpaper Heist: a magenta/lime zigzag on navy,
/// drawn as chunky pixels.
#[cfg_attr(not(any(windows, target_os = "linux", target_os = "macos")), allow(dead_code))]
pub fn heist_bmp(w: u32, h: u32, px: u32) -> Vec<u8> {
```

(Leave `ARROW`/`arrow_masks`'s `#[cfg_attr(not(windows), allow(dead_code))]` alone — Giant Cursor stays Windows-only.)

- [ ] **Step 2: Replace the platform module selection in `src/curses/mod.rs`**

Find:

```rust
#[cfg(windows)]
mod win32;
#[cfg(windows)]
use self::win32 as platform;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
use self::stub as platform;
```

Replace with:

```rust
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
```

- [ ] **Step 3: Create `src/curses/linux.rs`**

```rust
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
```

- [ ] **Step 4: Create `src/curses/macos.rs`**

```rust
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
```

- [ ] **Step 5: Run what can run locally**

Run: `cargo test` (on this Windows machine, this only re-verifies `win32.rs` + `game.rs`, which Task 1 already confirmed green; it does not compile `linux.rs`/`macos.rs`)
Expected: PASS, same as Task 1's final run.

- [ ] **Step 6: Commit**

```bash
git add src/curses/mod.rs src/curses/linux.rs src/curses/macos.rs
git commit -m "$(cat <<'EOF'
Port wallpaper and volume curses to Linux and macOS

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Five new overlay effects

**Files:**
- Modify: `ui/overlay.html`

**Interfaces:**
- Consumes: `parts` (a `Set<string>` of active part ids, already polled from the backend by the existing `poll()` function), `cur` (`[x, y]` cursor position or `null`), `t`/`dt` (elapsed/delta time), the existing helpers `rnd(a, b)`, `clamp(v, a, b)`.
- Produces: nothing new consumed elsewhere — these are leaf rendering blocks inside `frame()`.

There's no automated test harness for this file's visuals (the Playwright test in `tests/ui/run.js` drives the main window, not the separate overlay window). Verification is manual, via the QA checklist Task 4 adds to `docs/QA.md`.

- [ ] **Step 1: Add persistent state for the five new effects**

In the "state" section, right after the existing array declarations:

```js
let parts=new Set(), untilMs=0, cur=null, t=0, last=performance.now();
const trail=[], flakes=[], confetti=[]; let piles=new Float32Array(1);
const pal={x:40,dir:1,hop:0,vy:0,next:2}; const ghost={x:-20,y:-20,boo:0};
```

add:

```js
const drops=[], eyes=[], flies=[], cracks=[]; let crackGrow=0, crackA=0;
```

- [ ] **Step 2: Add the five rendering blocks inside `frame()`**

Right after the existing `ghost hunt` block (`if(parts.has('ghost')){ ... }`) and before the `doom clock` block, insert:

```js
  // static fuzz: random bright/dark dabs, no persistent state needed
  if(parts.has('static')){ for(let k=0;k<160;k++){ const x=Math.floor(rnd(0,LW)), y=Math.floor(rnd(0,LH));
    g.fillStyle=Math.random()<.5?W:'#9aa0ad'; g.fillRect(x,y,1,1); } }
  // matrix drip: falling glyph-less streaks, bright head fading to dark green tail
  if(parts.has('matrix')){ while(drops.length<LW/4) drops.push({x:Math.floor(rnd(0,LW)/4)*4+1,y:rnd(-LH,0),sp:rnd(30,70),len:Math.floor(rnd(6,16))});
    for(const d of drops){ d.y+=d.sp*dt; if(d.y>LH+d.len){ d.y=-rnd(0,40); d.sp=rnd(30,70); d.len=Math.floor(rnd(6,16)); }
      for(let i=0;i<d.len;i++){ const yy=Math.floor(d.y)-i; if(yy<0||yy>=LH) continue;
        g.fillStyle=i===0?'#d6ffd6':i<3?'#4fff6a':'#0a6b1a'; g.fillRect(d.x,yy,2,2); } } }
  else drops.length=0;
  // eyes everywhere: a fixed scatter of eyes, pupils track the cursor
  if(parts.has('eyes')){ if(!eyes.length) for(let k=0;k<9;k++) eyes.push({x:rnd(10,LW-10),y:rnd(10,LH-10)});
    for(const e of eyes){ g.fillStyle=W; g.fillRect(e.x-3,e.y-2,7,5);
      const dx=(cur?cur[0]:LW/2)-e.x, dy=(cur?cur[1]:LH/2)-e.y, d=Math.hypot(dx,dy)||1;
      g.fillStyle=K; g.fillRect(e.x+Math.round(dx/d)-1,e.y+Math.round(dy/d)-1,2,2); } }
  else eyes.length=0;
  // firefly dusk: wandering glow-dots that pulse brighter and dimmer
  if(parts.has('firefly')){ while(flies.length<14) flies.push({x:rnd(0,LW),y:rnd(0,LH),vx:rnd(-8,8),vy:rnd(-8,8),ph:rnd(0,6)});
    for(const f of flies){ f.x+=f.vx*dt; f.y+=f.vy*dt; f.ph+=dt*2;
      if(f.x<0||f.x>LW) f.vx*=-1; if(f.y<0||f.y>LH) f.vy*=-1;
      const b=.4+.6*Math.max(0,Math.sin(f.ph)); g.fillStyle=b>.7?'#ffe98a':b>.4?'#c8a84a':'#5a4a1a';
      g.fillRect(Math.round(f.x),Math.round(f.y),1,1); } }
  else flies.length=0;
  // crack spreads: a jagged dotted line that grows from a random point
  if(parts.has('crack')){ if(!cracks.length){ cracks.push({x:rnd(LW*.3,LW*.7),y:rnd(LH*.3,LH*.7)}); crackA=rnd(0,Math.PI*2); }
    crackGrow+=dt;
    if(crackGrow>.03&&cracks.length<220){ crackGrow=0; const p=cracks[cracks.length-1]; crackA+=rnd(-.4,.4);
      cracks.push({x:clamp(p.x+Math.cos(crackA)*1.6,0,LW-1),y:clamp(p.y+Math.sin(crackA)*1.6,0,LH-1)});
      if(Math.random()<.05) crackA+=rnd(1.2,2.2); }
    g.fillStyle='#cfd6e6'; for(const p of cracks) g.fillRect(Math.round(p.x),Math.round(p.y),1,1); }
  else { cracks.length=0; crackGrow=0; }
```

- [ ] **Step 3: Update the file's docstring**

Line 7-8 currently lists the curse names this window draws. Update it:

```html
Click-through, always-on-top window that draws the overlay curses (pal, rainbow trail, snow, fog, spotlight, confetti,
ghost, doom clock, red pulse, eclipse, static fuzz, matrix drip, eyes everywhere, firefly dusk, crack spreads). Same
pixel rules as the main UI: integer scale, fixed palette, Bayer dither instead of alpha. It only exists while gachaos
runs, so these curses can never outlive the app.
```

- [ ] **Step 4: Manual smoke test**

Run: `cargo build --release` then launch `target/release/gachaos.exe --qa`, open **Options › Force next roll**, pick Common repeatedly until each of Static Fuzz, Matrix Drip, Eyes Everywhere, Firefly Dusk, and Crack Spreads has shown up, confirming each draws something and clears cleanly when the curse expires (use `Ctrl+Alt+P` to cure instantly instead of waiting out the full minute).
Expected: each effect renders, and nothing is left on screen after curing.

- [ ] **Step 5: Commit**

```bash
git add ui/overlay.html
git commit -m "$(cat <<'EOF'
Add five new overlay curses for the expanded Common tier

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/QA.md`

**Interfaces:**
- Consumes: the final roster/odds/one_in numbers from Task 1, and the platform scope from Task 2.
- Produces: nothing consumed by other tasks (this is the last task).

- [ ] **Step 1: Replace the odds table in `README.md`**

```markdown
| Tier | Odds | Each entry | Curses last |
|---|---|---|---|
| Common | 53.5% | 1 in 34 | 1 min |
| Uncommon | 26.8% | 1 in 41 | 2 min |
| Rare | 13.4% | 1 in 52 | 4 min |
| Epic | 4.46% | 1 in 90 | 6 min |
| Legendary | 1.49% | 1 in 269 | 8 min |
| Mythic | 0.37% | 1 in 1076 | 10 min |
```

- [ ] **Step 2: Replace the roster table and its footnote in `README.md`**

```markdown
| Tier | Curses | Blessings |
|---|---|---|
| Common | Giant Cursor, Wrong-Handed (swapped buttons), Sloth Mouse (slowest pointer), Long Tail (mouse trails), Negative Space (inverted colours), Silent Film (greyscale), Crimson Tint (red screen wash), Pixel Pal*, Static Fuzz*, Matrix Drip*, Eyes Everywhere*, Firefly Dusk*, Crack Spreads* | Pocket Change (+1 ticket), Free Spin (next pull free), Talisman (blocks 1 curse), Penny From Heaven (next pull free, guaranteed Epic+), Rabbit's Foot (2× Rare+ odds for 1 pull) |
| Uncommon | Turbo Mouse (fastest pointer), Butterfingers (tiny double-click window), Whisper Mode† (volume 10%), Window Wobble, Sideways Vision (90° rotation), Rainbow Trail*, Confetti Storm*, Tunnel Vision* | Spare Change (+2), Four-Leaf Clover (2× Rare+ odds for 3 pulls), Mini Jackpot (+3) |
| Rare | Jitterbug Cursor, Wallpaper Heist†, Teleporter, Pixel Snow*, Fog Bank* | Lucky Break (+4), Umbrella (blocks 1 curse) |
| Epic | Upside-Down World, Beep Storm†, Spotlight* | Windfall (+8) |
| Legendary | Disco Inferno, Too Close, Ghost Hunt* | Sanctuary (30 min immunity) |
| Mythic | The Possession, Pandemonium, Doom Clock | Divine Grace (full tickets + 2 h immunity) |

\* Drawn by gachaos's own click-through overlay window rather than by changing
a Windows setting. These work on any OS and vanish the moment gachaos closes.

† Changes a real OS setting (wallpaper or system volume), with a native
implementation on Windows, Linux and macOS. On Linux/macOS, Beep Storm only
raises the volume — there's no portable repeating system-beep API, so the
beeping itself stays Windows-only.
```

- [ ] **Step 3: Fix the platform-availability sentence in `README.md`**

Find:

```markdown
To cut a release, push a tag like `v0.3.0`, or run the **release** workflow by
```

Just above it, find and replace:

```markdown
`gachaos-<version>-windows-x64.exe` (single file), a universal macOS `.dmg`, or
a Linux `.AppImage` / `.deb`. The full roster of curses runs on Windows; on
macOS and Linux only the overlay curses are available.
```

with:

```markdown
`gachaos-<version>-windows-x64.exe` (single file), a universal macOS `.dmg`, or
a Linux `.AppImage` / `.deb`. The full roster runs on Windows; on macOS and
Linux, the overlay curses (marked \*) and the wallpaper/volume curses (marked
†) are available — everything else needs a Windows-only API with no portable
equivalent.
```

- [ ] **Step 4: Fix the stale CI line in `README.md`**

Find:

```markdown
CI builds `gachaos.exe` on `windows-latest` for every push. Grab it from the
run's **gachaos-exe** artifact.
```

Replace with:

```markdown
CI builds and unit-tests on Windows, Linux and macOS for every push, plus a
headless Chromium run of the UI flow test. Grab the Windows build from the
run's **gachaos-exe** artifact.
```

- [ ] **Step 5: Update `docs/QA.md`'s title and intro**

Find:

```markdown
# Windows QA checklist

The curses change real Windows settings, so they can only be checked on a real
Windows machine. Launch `gachaos.exe --qa` and use **Options › Force next roll**
to pick a tier. Inside a tier the result is random, so roll until you've seen
each entry.
```

Replace with:

```markdown
# QA checklist

Most curses change real Windows settings, so they can only be fully checked on
a real Windows machine; the overlay curses (\*) and the wallpaper/volume
curses (†) can also be checked on Linux/macOS. Launch with `--qa` (or
`GACHAOS_QA=1`) and use **Options › Force next roll** to pick a tier. Inside a
tier the result is random, so roll until you've seen each entry.
```

- [ ] **Step 6: Replace the "Each curse" table in `docs/QA.md`**

```markdown
| Curse | Applies when | Undone means |
|---|---|---|
| Giant Cursor | Arrow, text and hand cursors are about 4× normal size | Your normal cursor scheme is back |
| Wrong-Handed | Left click acts as right click | Buttons back to how you had them (also if you're left-handed) |
| Sloth Mouse / Turbo Mouse | Pointer at slowest / fastest speed | Your pointer speed is back |
| Long Tail | Mouse trails on | Trails back to how you had them |
| Pixel Pal* | A green blob walks along the bottom of the screen | Gone |
| Negative Space / Silent Film | Whole screen inverted / greyscale | Normal colours |
| Crimson Tint | Whole screen washed in deep red | Normal colours |
| Static Fuzz* | Bursts of TV static sweep the screen | Gone |
| Matrix Drip* | Green streaks cascade down the screen | Gone |
| Eyes Everywhere* | A scatter of eyes tracks the cursor | Gone |
| Firefly Dusk* | Glowing dots wander and pulse | Gone |
| Crack Spreads* | A crack grows across the screen over time | Gone |
| Butterfingers | Double-click only works if very fast | Double-click speed back |
| Whisper Mode† | Volume drops to 10% | Volume and mute back |
| Window Wobble | The window you're using shuffles a few pixels | It sits back where it was |
| Rainbow Trail* | A pixel rainbow follows the cursor | Gone |
| Sideways Vision | Primary display rotated 90° | Original orientation and resolution |
| Confetti Storm* | Confetti falls | Gone |
| Tunnel Vision* | Dark except a shrinking circle around the cursor | Gone |
| Jitterbug Cursor | The pointer drifts on its own | No drift |
| Wallpaper Heist† | Wallpaper becomes a magenta/lime zigzag | Your wallpaper is back |
| Teleporter | The cursor jumps somewhere random every 3.5 s | No jumps |
| Pixel Snow* / Fog Bank* | Snow piles up at the bottom / fog at the edges | Gone |
| Upside-Down World | Primary display flipped 180° | Original orientation and resolution |
| Beep Storm† | Volume to at least 75%; on Windows, system beeps every 0.3–0.9 s too | Volume back, beeps stop |
| Spotlight* | Dark except a circle around the cursor | Gone |
| Disco Inferno | Wallpaper hidden, desktop colour cycles | Colour and wallpaper back |
| Too Close | Screen zoomed 1.8×, following the cursor | Normal zoom |
| Ghost Hunt* | A ghost that chases the cursor | Gone |
| The Possession | Upside-down + inverted colours + drifting cursor + red pulse | All of it undone |
| Pandemonium | One curse from each lower tier at once | All of it undone |
| Doom Clock | Giant countdown + Beep Storm + Teleporter | All of it undone |

\* Drawn by the overlay window; click-through, so clicks reach whatever is
under it. Works on Windows, Linux and macOS.

† Changes a real OS setting, with a native implementation on Windows, Linux
and macOS (Linux: `gsettings`/`pactl`; macOS: `osascript`).
```

- [ ] **Step 7: Update the "Blessings" and "Also check" sections in `docs/QA.md`**

Find:

```markdown
## Blessings

- [ ] Tickets blessings add tickets, even past 24.
- [ ] Free Spin / Midas Touch: the Pull button says "(free)" and no ticket is spent.
- [ ] Umbrella / Talisman: the next curse shows "Curse blocked" and nothing changes.
- [ ] Sanctuary / Divine Grace: curses are blocked until the immunity runs out.
```

Replace with:

```markdown
## Blessings

- [ ] Tickets blessings (Pocket Change, Spare Change, Lucky Break, Windfall,
      Mini Jackpot) add tickets, even past 24.
- [ ] Free Spin / Penny From Heaven: the Pull button says "(free)" and no
      ticket is spent; Penny From Heaven's pull also lands Epic or better.
- [ ] Umbrella / Talisman: the next curse shows "Curse blocked" and nothing
      changes. One block each.
- [ ] Rabbit's Foot / Four-Leaf Clover: Rare+ odds roughly double for 1 / 3
      pulls.
- [ ] Sanctuary / Divine Grace: curses are blocked until the immunity runs out.
```

Find, in "Also check":

```markdown
Known things to watch: the giant cursor size depends on how Windows scales
custom cursors, rotation depends on the display driver, and the Magnifier-based
effects (Negative Space, Silent Film, Too Close, Blood Moon) can't run while
Windows Magnifier is open.
```

Replace with:

```markdown
Known things to watch: the giant cursor size depends on how Windows scales
custom cursors, rotation depends on the display driver, and the Magnifier-based
effects (Negative Space, Silent Film, Too Close, Crimson Tint) can't run while
Windows Magnifier is open. The Linux wallpaper port assumes a GNOME-style
`gsettings` desktop and the volume port assumes PulseAudio/PipeWire via
`pactl`; both silently fail (logged, curse just doesn't take effect) on
anything else.
```

- [ ] **Step 8: Commit**

```bash
git add README.md docs/QA.md
git commit -m "$(cat <<'EOF'
Update README and QA checklist for the new roster and platform scope

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
EOF
)"
```
