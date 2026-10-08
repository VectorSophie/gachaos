# Windows QA checklist

The curses change real Windows settings, so they can only be checked on a real
Windows machine. Launch `gachaos.exe --qa` and use **Options › Force next roll**
to pick a tier. Inside a tier the result is random, so roll until you've seen
each entry.

## Getting out of a curse

Check each of these with a curse running (Jitterbug Cursor is a good one, it
was the one that got stuck before):

- [ ] **Close button**: ✕ asks "Close gachaos?". **Cure and close** undoes it
      at once; **Keep it** hides to the tray and the tray tooltip counts down.
- [ ] **Panic hotkey**: `Ctrl+Alt+P` cures, including while hidden in the tray.
- [ ] **Tray**: right-click › Cure all, and right-click › Quit both undo it.
- [ ] **Killed**: end `gachaos.exe` in Task Manager. The watchdog should undo
      the curse within a second. (If Task Manager kills the watchdog too, the
      curse stays until the next launch; note which happened.)
- [ ] **Relaunch**: starting gachaos again while it runs brings the window back
      instead of opening a second copy. `gachaos.exe --cure` cures through it.
- [ ] **Expiry**: wait it out.
- [ ] After a reboot, nothing a curse changed is still there.

## Each curse

| Curse | Applies when | Undone means |
|---|---|---|
| Giant Cursor | Arrow, text and hand cursors are about 4× normal size | Your normal cursor scheme is back |
| Wrong-Handed | Left click acts as right click | Buttons back to how you had them (also if you're left-handed) |
| Sloth Mouse / Turbo Mouse | Pointer at slowest / fastest speed | Your pointer speed is back |
| Long Tail | Mouse trails on | Trails back to how you had them |
| Pixel Pal* | A green blob walks along the bottom of the screen | Gone |
| Butterfingers | Double-click only works if very fast | Double-click speed back |
| Whisper Mode | Volume drops to 10% | Volume and mute back |
| Window Wobble | The window you're using shuffles a few pixels | It sits back where it was |
| Rainbow Trail* | A pixel rainbow follows the cursor | Gone |
| Jitterbug Cursor | The pointer drifts on its own | No drift |
| Wallpaper Heist | Wallpaper becomes a magenta/lime zigzag | Your wallpaper is back |
| Teleporter | The cursor jumps somewhere random every 3.5 s | No jumps |
| Pixel Snow* / Fog Bank* | Snow piles up at the bottom / fog at the edges | Gone |
| Upside-Down World / Sideways Vision | Primary display flipped 180° / rotated 90° | Original orientation and resolution |
| Beep Storm | Volume to at least 75%, system beeps every 0.3–0.9 s | Volume back, beeps stop |
| Negative Space / Silent Film | Whole screen inverted / greyscale | Normal colours |
| Spotlight* | Dark except a circle around the cursor | Gone |
| Disco Inferno | Wallpaper hidden, desktop colour cycles | Colour and wallpaper back |
| Too Close | Screen zoomed 1.8×, following the cursor | Normal zoom |
| Confetti Storm* / Ghost Hunt* | Confetti / a ghost that chases the cursor | Gone |
| The Possession | Upside-down + inverted colours + drifting cursor + red pulse | All of it undone |
| Pandemonium | One curse from each lower tier at once | All of it undone |
| Doom Clock | Giant countdown + Beep Storm + Teleporter | All of it undone |
| Blood Moon | Screen tinted blood red + Spotlight + red pulse | All of it undone |
| Eclipse | Sideways + a shrinking ring of light + snow | All of it undone |

\* Drawn by the overlay window; click-through, so clicks reach whatever is under it.

## Blessings

- [ ] Tickets blessings add tickets, even past 24.
- [ ] Free Spin / Midas Touch: the Pull button says "(free)" and no ticket is spent.
- [ ] Umbrella / Talisman: the next curse shows "Curse blocked" and nothing changes.
- [ ] Sanctuary / Divine Grace: curses are blocked until the immunity runs out.

## Also check

- [ ] Epic and up cover the whole monitor (taskbar included), then the window
      shrinks back to where it was. Mythic shatters the window and rebuilds it.
- [ ] Click the tray icon: the flyout opens next to it, and hides when you click
      elsewhere. ⤢ switches to the full window.
- [ ] The window can be dragged by its title bar and minimized.
- [ ] If Windows refuses a curse (rotation on some drivers and VMs; colour
      effects while Windows Magnifier is running), the dialog says so and
      nothing changes. In a stacked Mythic curse, the parts that worked stay.

Known things to watch: the giant cursor size depends on how Windows scales
custom cursors, rotation depends on the display driver, and the Magnifier-based
effects (Negative Space, Silent Film, Too Close, Blood Moon) can't run while
Windows Magnifier is open.
