# gachaos: redesign spec

Date: 2026-10-07. Supersedes the Godot plan in `2026-07-27-gachaos-design.md`.
The concept, economy and guardrails from that spec still apply.

## What changed

- **Stack**: Tauri 2 (Rust) with a single HTML/canvas UI, instead of Godot/C#.
  The UI is the prototype that was signed off, moved over as is. Rust owns
  everything that must be trustworthy: save file, odds, Win32 curses, tray,
  global hotkey, crash recovery.
- **Roll**: a three-reel slot machine in place of the card reveal, with a
  Sol's-RNG-style name ticker that flickers curse names and "1 IN N" odds while
  the reels spin.
- **Look**: the shell stays a drab legacy `.exe` (Win95 bevels, navy title bar,
  menu bar, status bar). The reveal is pixel art: fixed palette, integer
  scaling, Bayer-dithered light, with a blurred copy layered on top as glow.

## Escalation

| Tier | What happens |
|---|---|
| Common | The reels stop, a ding plays, a plain message box appears. |
| Rare | Sparks inside the reels, a few leak onto the window, and the title bar flashes. |
| Epic | The window grows to cover the monitor (transparent, always on top). The real desktop dims and rays fire from the reels across the screen, then the curse name slams in. |
| Legendary | The screen goes black, stars warp past and a "1 IN N" counter climbs as an orb charges. Then a white flash, gold rays, confetti and the title. The window drops back in. |

A matching pair on the first two reels slows the third, for longer the rarer
the pair. Epic and Legendary sometimes stop one symbol short, glitch, then
click up. Clicking during a big reveal fast-forwards it.

## Curses

Each curse records what it changes before changing it (`Restore` in
`src/curses/mod.rs`), and that record is saved, so it can be undone on
expiry, by `Ctrl+Alt+P`, on quit, or on the next launch after a crash. None of
the changes are written to the Windows profile, so a reboot also undoes them.
See the README for the roster and `docs/QA.md` for what each one should do.

## Windows

- Borderless, transparent window sized to the chrome. Dragged by the title bar.
- ✕ closes to the tray. The tray menu has Open, Cure all and Quit.
- `--qa` unlocks forcing a tier, refilling tickets and resetting the save.

## Testing

- `cargo test`: odds within tolerance over 100k pulls, pity cap, ticket
  regen, history cap, save round-trip and corrupt-save handling, cursor and
  wallpaper bitmaps, worker shutdown.
- `tests/ui/run.js`: rolls every tier in Chromium against a mocked bridge and
  checks that the rolled curse is the one applied and the window is restored.
- Win32 effects: manual checklist in `docs/QA.md`.
