# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

gachaos: a Windows exe that wraps a gacha-pull dopamine loop around temporary,
reversible "curse" effects on the user's desktop (giant cursor, muted volume,
flipped screen, etc.). The app chrome is deliberately styled like a Windows
95 dialog box; the pull reveal animation is deliberately flashy/modern
(particles, light rays, screen shake), rolled per Godot's tween/particle
system. Full design spec: `docs/superpowers/specs/2026-07-27-gachaos-design.md`
— read it before making product/architecture decisions, since it documents
intentional constraints (e.g. no fake-crash/fake-BSOD effects — out of scope
for every rarity tier).

Tech stack: Godot 4.7 (C#/.NET 8, Mono), targeting a single portable Windows
exe with no installer.

## Commands

Godot and `dotnet` are **not installed in this container** — verify with
`which godot dotnet` before assuming either command works.

```
# Build (compile C#)
dotnet build gachaos.csproj

# Run the rarity/pity self-check (assert-based, no test framework)
godot --headless --path . -- selfcheck

# Dump a PNG of the running app for visual verification (requires Xvfb on
# this Linux dev machine; does NOT work under --headless since headless
# never renders)
xvfb-run -a godot --path . -- capture:/tmp/gachaos_capture.png simulatepulls:5

# Regenerate the Win95 bevel theme textures (assets/theme/*.png)
godot --headless --path . --script tools/gen_theme_assets.gd
```

There is no test framework — `RarityRollerSelfCheck` is a hand-rolled
assert-based check (100k simulated rolls, verifies rarity distribution is
within tolerance and pity caps the no-Epic+ gap), run via the `selfcheck`
CLI arg above and wired through the `DebugSelfCheck` autoload.

OS-facing curse effects (cursor/volume/display Win32 calls) cannot be
verified on this Linux container at all — they no-op off Windows (see
`Win32Interop.IsWindows`) and require a manual QA pass on real Windows.

## Architecture

**Autoloads** (`project.godot` `[autoload]`, singletons reachable via
`/root/<Name>`): `TicketEconomy`, `EffectScheduler`, `DebugSelfCheck`,
`DebugCapture`. `MainWindow` (the main scene script) looks these up by
absolute path, e.g. `GetNode<TicketEconomy>("/root/TicketEconomy")`.

**Single save file** (`user://save.json`, `src/Economy/SaveData.cs`) holds
all persistent state: ticket count + last regen timestamp, pity counter,
active effects + expiry, and pull history/stats. `SaveManager` is the only
reader/writer. Regen is computed from elapsed wall-clock time on load/tick
(`TicketEconomy.ApplyOfflineRegen`), not a running counter, so tickets
accrue correctly while the app is closed. On startup, `EffectScheduler`
reads `ActiveEffects` and immediately reverts anything still marked active
— this is the crash-recovery path for effects that never got their
`Revert()` call.

**Effect system** (`src/Effects/`): each curse implements `IEffect`
(`Apply()` / `Revert()`, tier, duration, display strings) and lives under
`Effects/Curses/`. `EffectScheduler` holds the registry (keyed by `Id`),
rolls/applies pulls, ticks a 1s timer to expire+revert effects past their
`ExpiryUnixTime`, and handles the panic hotkey (Ctrl+Alt+P →
`PanicRevertAll`, currently only fires while the app window has focus — see
the TODO comment in `EffectScheduler._Ready`). A `DurationSeconds` of -1
means manual-cure-only (still cleared by panic). Adding a new curse means
implementing `IEffect` and registering it in `EffectScheduler._Ready`.

**Win32 interop** (`src/Effects/Native/Win32Interop.cs` plus P/Invoke calls
directly inside individual curse effects like `CursorJitterEffect`): every
call is guarded by `Win32Interop.IsWindows` and no-ops off Windows, which is
what lets the whole app run and be visually verified under Linux/Xvfb during
development. Several effects are still stubs pending real-Windows
verification — check for `TODO`/no-op comments (e.g. `EffectScheduler._Ready`
flags giant cursor and volume mute as needing more Win32/Core Audio work)
before assuming an effect is functional.

**Gacha roll logic** (`src/Gacha/RarityRoller.cs`): weighted roll (Common
60 / Rare 30 / Epic 8 / Legendary 2) with pity — if
`PullsSinceEpicPlus >= PityThreshold - 1` the roll is forced to Epic or
Legendary. `RarityColors` maps tiers to the colors used in history/UI.

**UI** (`src/UI/MainWindow.cs`, `scenes/Main.tscn`, `assets/theme/`): the
Win95 chrome is a Godot `Theme` resource (`win95_theme.tres`) plus
hand-generated bevel PNGs (regenerate via `tools/gen_theme_assets.gd`, don't
hand-edit the PNGs). `MainWindow._Ready` wires the pull button, ticket
label, best-pull label, and scrollable history list to `TicketEconomy` and
`EffectScheduler`; `DebugSimulatePull()` exists purely so `DebugCapture` can
drive real pulls (same code path as a click) for headless-free screenshot
verification.

**Debug-only autoloads** (`src/Debug/`): both are inert unless invoked with
a specific CLI user-arg (see Commands above), safe to leave registered in
normal runs. `DebugCapture` reaches into the scene tree
(`GetTree().Root.GetNode<MainWindow>("Main")`) to drive simulated pulls
before dumping a screenshot.
