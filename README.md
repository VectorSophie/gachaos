# gachaos

A boredom gacha for Windows. It looks like a plain old `.exe` dialog: a ticket
counter, a Pull button and a three-reel slot machine. Each pull curses your
desktop for a few minutes, and the rarer the roll, the louder the reveal. Common
rolls stay inside the reels. Epic and Legendary rolls break out of the window
and take over the screen.

Every curse undoes itself, `Ctrl+Alt+P` undoes all of them at once (even from
the tray), and nothing is saved to your Windows profile, so a reboot also
undoes everything.

| Tier | Odds | Curses | Lasts |
|---|---|---|---|
| Common | 60% | Giant Cursor, Wrong-Handed (swapped mouse buttons) | 2 min |
| Rare | 30% | Jitterbug Cursor, Wallpaper Heist | 5 min |
| Epic | 8% | Upside-Down World (screen 180°), Beep Storm | 8 min |
| Legendary | 2% | Sideways Vision (screen 90°), Disco Inferno | 10 min |

Tickets refill at 1 per hour, up to 24. The 20th pull without an Epic is
guaranteed Epic or better.

## Layout

- `ui/index.html`: the whole interface: window chrome, reels, pixel effects
  and chip sounds. No framework, no build step.
- `app/`: the Tauri 2 shell in Rust. It owns the save file, the odds, the real
  Win32 curses (`app/src/curses/win32.rs`), the tray icon, the global hotkey
  and crash recovery.
- `tests/ui/`: a browser test that rolls every tier against a mocked bridge.
- `docs/QA.md`: the manual checklist for the curses on real Windows.

## Build

Needs Rust (stable). On Windows, WebView2 ships with Windows 10 and 11.

```sh
cd app
cargo test               # game rules, save file, curse helpers
cargo build --release    # -> app/target/release/gachaos.exe (single file)
```

CI builds `gachaos.exe` on `windows-latest` for every push. Grab it from the
run's **gachaos-exe** artifact.

Run with `--qa` (or `GACHAOS_QA=1`) to unlock **Options › Force next roll**,
**Refill tickets** and **Reset save** for testing.

From Linux you can cross-compile with `cargo xwin build --release --target
x86_64-pc-windows-msvc`. On Linux the app itself also runs, with curses as
no-ops, which is handy for UI work.

## UI test

```sh
npm install --no-save playwright && npx playwright install chromium
node tests/ui/run.js     # screenshots land in tests/ui/out/
```
