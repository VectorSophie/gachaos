# gachaos — Design Spec

Date: 2026-07-27

## Concept

A Windows exe that turns boredom into a gacha-style dopamine hit. The app's own
window is skinned like a Windows 95 dialog box — plain, chunky, "boring" on
purpose — housing just a Pull button, a ticket counter, and a chances table
(styled like a Win95 "Information" dialog). Pulling triggers a bright, flashy,
modern anime-gacha-style reveal animation (particles, light rays, screen
shake) — a deliberate contrast between the drab shell and the payoff. The pull
result is a temporary "curse": a silly, reversible modification to the user's
Windows interface (giant cursor, muted volume, flipped screen, etc.), scaled
in severity by rarity.

## Core loop & economy

- **Currency**: "Tickets." Regenerate +1/hour, capped at 24 (a full day to
  refill from empty).
- **Rarity tiers**: Common 60% / Rare 30% / Epic 8% / Legendary 2%.
- **Pity**: guarantees an Epic+ result if none has landed in the last ~20
  pulls.

## Curse effects (draft roster)

Duration scales with rarity tier:

| Tier | Duration | Example effects |
|---|---|---|
| Common | ~2 min | Giant cursor, rainbow cursor trail, desktop icons shuffled |
| Rare | ~5 min | Muted volume, inverted cursor axis, wallpaper swapped, taskbar auto-hidden |
| Epic | ~8 min | Screen flipped 180°, cursor teleports randomly, volume maxed + random beeps |
| Legendary | ~10 min or manual "cure" | Screen rotated 90°, fullscreen confetti overlay, disco wallpaper cycling, siren + rainbow flash |

**Guardrail**: no fake-crash, fake-data-loss, or fake-lockout effects (e.g. a
fake BSOD). These read as genuinely alarming and edge toward real
malware-like behavior rather than a harmless prank — out of scope for every
tier.

**Safety valve**: a global hotkey (Ctrl+Alt+P) instantly reverts all active
effects, reachable even while the app is minimized to the tray. On launch,
the app checks its save file for any effect that never got reverted (e.g. the
app crashed) and reverts it immediately.

## Visual design

- **App chrome**: styled like classic Windows 95/3.1 — gray beveled panels,
  colored title bar, chunky pixel-style buttons and fonts, modeled directly
  on reference screenshots of period dialog boxes (error/message boxes,
  install wizards, progress bars). Applies to the main window, the chances
  table, and any in-app dialogs.
- **Pull reveal animation**: stays fully modern and flashy regardless of the
  retro chrome — anime-gacha style (think Genshin/Honkai): particle buildup
  that scales with the rolled rarity without revealing it too early, a burst
  with rarity-colored light rays, screen shake for Epic/Legendary pulls, and
  a result card with the curse name, description, and countdown timer.

## Architecture

- **Effect system**: each curse is a class implementing `Apply()` /
  `Revert()`. A scheduler tracks active effects and their expiry, calling
  `Revert()` on timeout. State (ticket count, last regen timestamp, pity
  counter, active effects + expiry timestamps) persists to a single JSON save
  file, read on startup to clean up anything left active from a crash.
- **OS integration**: a small C# interop layer wraps the Win32 calls needed
  per effect (`user32.dll` for cursor/display, `winmm.dll`/Core Audio for
  volume).
- **Pull animation**: built with Godot's tween/particle system — buildup
  intensity keyed to the rolled rarity, rarity-colored burst, result card.
- **App shell**: single main window, minimizes to a system tray icon so it
  can sit idle between pulls. The panic hotkey is registered globally and
  works even while tray-minimized.

## Tech stack & packaging

Godot 4.x (C#/Mono), exported as a single portable Windows exe (no
installer). Chosen over WPF/WinUI because it's cross-platform — this matters
for the dev workflow below — and because its particle/tween/shader tools are
a more direct fit for genuine gacha-game-style VFX than a XAML animation
timeline.

## Dev & QA workflow

The dev machine is Linux, and no tool in this environment can screenshot a
native window — only browser tab content is visually inspectable. Because
Godot is cross-platform, the project runs under Xvfb (virtual display) on
this machine during development: the pull animation is scripted to
auto-play, with PNG frames dumped to disk at key moments, which are read
directly to self-verify timing, color, and particle feel without needing a
round-trip through the user for most iterations.

This closes the loop for the animation and app-chrome visuals. It does
**not** cover the OS-facing curse effects (cursor/volume/display changes),
which can only be verified on a real Windows session. Those get a manual QA
pass by the user once a build exists. The user also does a final taste-check
of the built exe.

## Testing

- **Rarity/pity roll logic**: a small self-check asserting rolled
  distributions land within tolerance of the configured weights over N rolls,
  and that pity fires at the correct pull count.
- **OS-facing effects**: manual QA checklist run on real Windows, covering
  apply + auto-revert + panic-hotkey-revert for each curse.
