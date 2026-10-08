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
| Common | 50% | 1 in 14 | 1 min |
| Uncommon | 28% | 1 in 25 | 2 min |
| Rare | 14% | 1 in 50 | 4 min |
| Epic | 6% | 1 in 117 | 6 min |
| Legendary | 1.6% | 1 in 438 | 8 min |
| Mythic | 0.4% | 1 in 1750 | 10 min |

The 20th pull without an Epic or better is guaranteed Epic or better, and the
90th without a Legendary or better is guaranteed Legendary or better. Tickets
refill at 1 per hour, up to 24 (blessings can take you past that).

## Roster

| Tier | Curses | Blessings |
|---|---|---|
| Common | Giant Cursor, Wrong-Handed (swapped buttons), Sloth Mouse (slowest pointer), Long Tail (mouse trails), Pixel Pal* | Pocket Change (+1 ticket), Free Spin (next pull free) |
| Uncommon | Turbo Mouse (fastest pointer), Butterfingers (tiny double-click window), Whisper Mode (volume 10%), Window Wobble, Rainbow Trail* | Spare Change (+2), Four-Leaf Clover (2× Rare+ odds for 3 pulls) |
| Rare | Jitterbug Cursor, Wallpaper Heist, Teleporter, Pixel Snow*, Fog Bank* | Lucky Break (+4), Umbrella (blocks 1 curse) |
| Epic | Upside-Down World, Beep Storm, Negative Space (inverted colours), Silent Film (greyscale), Spotlight* | Windfall (+8), Talisman (blocks 2 curses) |
| Legendary | Sideways Vision, Disco Inferno, Too Close (zoom that follows the cursor), Confetti Storm*, Ghost Hunt* | Jackpot (+16), Sanctuary (30 min immunity) |
| Mythic | The Possession, Pandemonium, Doom Clock, Blood Moon, Eclipse (each stacks several curses at once) | Midas Touch (5 free Epic+ pulls), Divine Grace (full tickets + 2 h immunity) |

\* Drawn by gachaos's own click-through overlay window rather than by changing
a Windows setting. These work on any OS and vanish the moment gachaos closes.

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
a Linux `.AppImage` / `.deb`. The full roster of curses runs on Windows; on
macOS and Linux only the overlay curses (marked * above) are available.

To cut a release, push a tag like `v0.3.0`. The **release** workflow builds all
three, attaches them with checksums, and uses `.github/release-notes/<tag>.md`
as the notes.

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

CI builds `gachaos.exe` on `windows-latest` for every push. Grab it from the
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
