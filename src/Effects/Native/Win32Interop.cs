using Godot;
using System;
using System.Runtime.InteropServices;

namespace Gachaos.Effects.Native;

// Thin wrapper around the Win32 calls the curse effects need. Every call
// no-ops off Windows so the same build can run under Linux/Xvfb during
// animation development (see docs/superpowers/specs — dev workflow section).
public static class Win32Interop
{
    public static bool IsWindows => OS.GetName() == "Windows";

    private const int SPI_SETCURSORS = 0x0057;
    private const uint SPIF_UPDATEINIFILE = 0x01;
    private const uint SPIF_SENDCHANGE = 0x02;

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool SystemParametersInfo(int uiAction, int uiParam, IntPtr pvParam, uint fWinIni);

    [DllImport("user32.dll", CharSet = CharSet.Auto)]
    private static extern bool SetSystemCursor(IntPtr hcur, uint id);

    [DllImport("user32.dll")]
    private static extern IntPtr LoadCursor(IntPtr hInstance, int lpCursorName);

    // Restores the user's normal OS cursor scheme (undoes SetSystemCursor calls).
    public static void RestoreDefaultCursors()
    {
        if (!IsWindows) { GD.Print("[Win32Interop] RestoreDefaultCursors: no-op (not Windows)"); return; }
        SystemParametersInfo(SPI_SETCURSORS, 0, IntPtr.Zero, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
    }

    // Volume control needs the Core Audio COM API (IAudioEndpointVolume), which
    // doesn't have a simple P/Invoke signature — implemented separately once
    // real-Windows testing is available. Placeholder keeps the effect roster
    // wired end-to-end in the meantime.
    public static void SetMasterMute(bool mute)
    {
        if (!IsWindows) { GD.Print($"[Win32Interop] SetMasterMute({mute}): no-op (not Windows)"); return; }
        GD.PrintErr("[Win32Interop] SetMasterMute not yet implemented — needs Core Audio interop, verify on real Windows.");
    }
}
