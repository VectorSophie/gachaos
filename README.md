# gachaos

A boredom gacha for Windows. It looks like a plain old `.exe` dialog: a ticket
counter, a Pull button and a three-reel slot machine. Each pull curses (or
blesses) your desktop, and the rarer the roll, the louder the reveal. Common
rolls stay inside the reels. Epic and up break out of the window and take over
the screen, and Mythic shatters the window itself.

Every curse undoes itself. `Ctrl+Alt+P` undoes all of them at once, even from
the tray. Nothing is saved to your Windows profile, so a reboot also undoes
everything. If gachaos is killed mid-curse, a small watchdog process undoes the
curse straight away; failing that, the next launch does.

## Odds

Each tier has 5 curses and 2 blessings, all equally likely within the tier.

| Tier | Odds | Each entry | Curses last |
|---|---|---|---|
| Common | 53.5% | 1 in 34 | 1 min |
| Uncommon | 26.8% | 1 in 41 | 2 min |
| Rare | 13.4% | 1 in 52 | 4 min |
| Epic | 4.46% | 1 in 90 | 6 min |
| Legendary | 1.49% | 1 in 269 | 8 min |
| Mythic | 0.37% | 1 in 1076 | 10 min |

The 20th pull without an Epic or better is guaranteed Epic or better, and the
90th without a Legendary or better is guaranteed Legendary or better. Tickets
refill at 1 per hour, up to 24 (blessings can take you past that).

## Roster

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

## Closing and the tray

- The window's ✕ hides gachaos to the tray. If a curse is running it asks first:
  **Cure and close**, or keep the curse until it wears off.
- Click the tray icon for the compact flyout (tickets, reels, Pull). Right-click
  for Open, Cure all and Quit. The tray tooltip shows the active curse and its
  time left.
- Launching gachaos again brings the running copy forward. `gachaos.exe --cure`
  cures through the running copy, or undoes a leftover curse if none is running.

## Download

Grab the latest build from the [Releases page](https://github.com/VectorSophie/gachaos/releases):
`gachaos-<version>-windows-x64.exe` (single file), a universal macOS `.dmg`, or
a Linux `.AppImage` / `.deb`. The full roster runs on Windows; on macOS and
Linux, the overlay curses (marked \*) and the wallpaper/volume curses (marked
†) are available — everything else needs a Windows-only API with no portable
equivalent.

To cut a release, push a tag like `v0.3.0`, or run the **release** workflow by
hand on `master` with the tag name (it creates the tag). It builds all three,
attaches them with checksums, and uses `.github/release-notes/<tag>.md` as the
notes.

## Layout

- `ui/index.html`: the whole interface: window chrome, reels, pixel effects
  and chip sounds. No framework, no build step.
- `ui/overlay.html`: the click-through overlay that draws the * curses.
- `src/`, `Cargo.toml`, `tauri.conf.json`: the Tauri 2 shell in Rust. It owns
  the save file, the odds and the roster (`src/game.rs`), the real Win32 curses
  (`src/curses/win32.rs`), the tray, the global hotkey, the watchdog and crash
  recovery.
- `tests/ui/`: a browser test that rolls every tier against a mocked bridge.
- `docs/QA.md`: the manual checklist for the curses on real Windows.

## Build

Needs Rust (stable). On Windows, WebView2 ships with Windows 10 and 11.

```sh
cargo test               # game rules, save file, curse helpers
cargo build --release    # -> target/release/gachaos.exe (single file)
```

CI builds and unit-tests on Windows, Linux and macOS for every push, plus a
headless Chromium run of the UI flow test. Grab the Windows build from the
run's **gachaos-exe** artifact.

Run with `--qa` (or `GACHAOS_QA=1`) to unlock **Options › Force next roll**,
**Refill tickets** and **Reset save** for testing.

From Linux you can cross-compile with `cargo xwin build --release --target
x86_64-pc-windows-msvc`. On Linux the app itself also runs with only the
overlay curses, which is handy for UI work.

## UI test

```sh
npm install --no-save playwright && npx playwright install chromium
node tests/ui/run.js     # screenshots land in tests/ui/out/
```
