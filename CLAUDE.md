# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

gachaos: a Windows app whose UI is deliberately styled like a boring Win95
dialog box (ticket counter, Pull button, chances table) that, on pull,
plays a flashy modern anime-gacha-style reveal animation and then applies a
temporary, reversible "curse" to the user's Windows interface (giant
cursor, flipped screen, swapped mouse buttons, etc.), scaled in severity by
rarity. Full design spec: `docs/superpowers/specs/2026-07-27-gachaos-design.md`
— read it for the intended economy, curse roster, and safety-valve
behavior before changing those areas.

Tech stack: Godot 4.7 (C#/.NET 8, `net8.0`), targeting export as a single
portable Windows exe. Godot was chosen specifically because it's
cross-platform, which matters for the dev workflow below.

## Dev environment caveats

Neither the `godot` nor `dotnet` CLI is available in this sandbox — there
is no way to build, run, or self-check the project from here. Changes must
be made by reading/editing source directly and reasoning about correctness;
the commands below are for a machine that has Godot 4.7.1 (Mono/C# build)
installed, not for this session.

## Commands (on a machine with Godot installed)

```
# Rarity/pity distribution self-check (headless, no display needed)
godot --headless --path . -- selfcheck

# Visual self-verification: simulate N pulls and dump a PNG of the resulting
# frame (requires a virtual display since headless mode never renders)
xvfb-run -a godot --path . -- capture:/tmp/gachaos_capture.png simulatepulls:5

# Regenerate the Win95 bevel/window-frame theme PNGs from code
godot --headless --path . --script tools/gen_theme_assets.gd
```

There is no test framework or dotnet test suite — `RarityRollerSelfCheck`
(invoked via `selfcheck`) is the only automated check, asserting rolled
rarity distributions land within tolerance over 100k rolls and that pity
correctly caps the gap between Epic+ results. `DebugCapture`/`DebugSelfCheck`
are both autoloads (registered in `project.godot`) that stay inert unless
launched with the matching `--` user arg, so they're always present in
normal play without side effects.

The OS-facing curse effects (cursor/display/volume) only ever no-op outside
Windows (`Win32Interop.IsWindows` gate) and must get a manual QA pass on a
real Windows build — nothing in this repo can verify them.

## Architecture

**Autoloads** (`project.godot` `[autoload]`, singletons reachable via
`GetNode<T>("/root/Name")`): `TicketEconomy`, `EffectScheduler`,
`DebugSelfCheck`, `DebugCapture`. `MainWindow` (the scene script on
`scenes/Main.tscn`) fetches both `TicketEconomy` and `EffectScheduler` this
way rather than through dependency injection.

**Economy** (`src/Economy/`): `TicketEconomy` owns the ticket count and
regen timer. Regen is computed from elapsed wall-clock time against
`LastRegenUnixTime`, not a running in-memory counter, so tickets accrue
correctly across app restarts/offline time. `SaveData` is the single
serializable state blob (tickets, pity counter, active effects + their
expiry timestamps, pull history/stats) and `SaveManager` round-trips it
to/from a single JSON file at `user://save.json`, falling back to a fresh
save on a missing or corrupt file.

**Gacha** (`src/Gacha/`): `RarityRoller.Roll()` is pure (takes an RNG and
the current pity counter, returns a `Rarity`) — no Godot node state — which
is what makes it unit-testable via `RarityRollerSelfCheck`. Pity forces an
Epic-or-higher result once `pullsSinceEpicPlus >= PityThreshold - 1`.

**Effects** (`src/Effects/`): `IEffect` is the curse contract
(`Apply()`/`Revert()`, an `Id`/`Tier`/`DurationSeconds`, where
`DurationSeconds == -1` means manual-cure-only). `EffectScheduler` is the
central coordinator: it registers all `IEffect` implementations from
`src/Effects/Curses/` in `_Ready()`, picks a random effect matching the
rolled tier in `ApplyPullResult()`, persists the result into
`SaveData.ActiveEffects`/`PullHistory`/stats via `SaveManager`, ticks a
1-second `Timer` to auto-revert expired effects, and recovers from an
unclean shutdown by reverting any effect still marked active on startup
(`RecoverFromCrash()`). Ctrl+Alt+P is the panic hotkey
(`_UnhandledKeyInput`) that force-reverts everything immediately — this
only fires while the app window has focus today; a global/tray-minimized
hotkey needs a Win32 `RegisterHotKey` message-loop hook (noted as a
follow-up in `EffectScheduler`).

`src/Effects/Native/Win32Interop.cs` centralizes the Win32 P/Invoke surface
and the `IsWindows` no-op gate; individual curse effects
(`src/Effects/Curses/*.cs`) each P/Invoke directly for their own narrow
need rather than funneling everything through `Win32Interop`. Several
effects (`ScreenFlipEffect`, `ScreenRotate90Effect`, and
`Win32Interop.SetMasterMute`) are currently wired as no-op stubs pending
real Windows-only APIs (`ChangeDisplaySettingsEx`, Core Audio
`IAudioEndpointVolume`) — check for a `TODO` comment before assuming an
effect is fully implemented.

**UI** (`src/UI/MainWindow.cs` + `scenes/Main.tscn`): one `Control` scene.
Node lookups use Godot's `%UniqueName` syntax (`unique_name_in_owner = true`
in the `.tscn`), not full node paths. `OnPullPressed` is the single
pull-button handler; `DebugSimulatePull()` exists only so `DebugCapture` can
drive the exact same code path for headless visual verification.

**Theme** (`assets/theme/`): Win95 look comes from `win95_theme.tres` plus
bevel/window-frame PNGs generated by `tools/gen_theme_assets.gd` (not
hand-drawn) — regenerate via the command above rather than editing the
PNGs directly if the bevel colors/sizes need to change.

Every `.cs` file has a matching `.cs.uid` (Godot's own generated file —
don't hand-edit or worry about keeping these in sync manually).
