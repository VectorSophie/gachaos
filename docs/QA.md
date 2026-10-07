# Windows QA checklist

The curses change real Windows settings, so they can only be checked on a real
Windows machine. Launch `gachaos.exe --qa` and use **Options › Force next roll**
to pick each tier. Inside a tier the curse is random, so roll until you've seen
both.

For every curse, check all three ways of undoing it:

1. **Expiry**: wait it out. (To save time, close the app from the tray with
   **Quit** instead: quitting must also undo it.)
2. **Panic hotkey**: press `Ctrl+Alt+P`, including while the window is closed
   to the tray.
3. **Crash**: end `gachaos.exe` in Task Manager while the curse is active, then
   relaunch. It should undo the curse and say so in a dialog.

| Curse | Applies when | Undone means |
|---|---|---|
| Giant Cursor | Arrow, text and hand cursors are about 4× normal size | Your normal cursor scheme is back |
| Wrong-Handed | Left click acts as right click | Buttons back to how you had them (also if you're left-handed) |
| Jitterbug Cursor | The pointer drifts on its own, changing direction about every second | No drift |
| Wallpaper Heist | Wallpaper becomes a magenta/lime zigzag | Your original wallpaper is back |
| Upside-Down World | Primary display flipped 180° | Original orientation |
| Beep Storm | Volume raised to at least 75% and unmuted; system beeps every 0.3–0.9 s | Volume and mute back to what they were |
| Sideways Vision | Primary display rotated 90° | Original orientation and resolution |
| Disco Inferno | Wallpaper hidden, desktop colour cycles fast | Colour and wallpaper back |

Also check:

- [ ] Epic and Legendary reveals cover the whole monitor (taskbar included),
      then the window shrinks back to where it was.
- [ ] The window can be dragged by its title bar, minimized, and closed to the
      tray. Clicking the tray icon brings it back.
- [ ] If rotating is refused (some drivers and VMs do this), the result dialog
      says the curse failed and nothing changed.
- [ ] After a reboot, nothing a curse changed is still there.

Known things to watch: the giant cursor size depends on how Windows scales
custom cursors, and screen rotation depends on the display driver.
