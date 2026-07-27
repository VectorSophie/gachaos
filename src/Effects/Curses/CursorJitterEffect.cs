#nullable enable
using System;
using System.Runtime.InteropServices;
using Gachaos.Gacha;
using Gachaos.Effects.Native;

namespace Gachaos.Effects.Curses;

public class CursorJitterEffect : IEffect
{
    public string Id => "cursor_jitter";
    public string DisplayName => "Jitterbug Cursor";
    public string Description => "Your cursor randomly teleports every couple of seconds.";
    public Rarity Tier => Rarity.Rare;
    public double DurationSeconds => 300;

    private System.Threading.Timer? _timer;
    private readonly Random _rng = new();

    [DllImport("user32.dll")]
    private static extern bool SetCursorPos(int x, int y);

    [DllImport("user32.dll")]
    private static extern int GetSystemMetrics(int nIndex);

    private const int SM_CXSCREEN = 0;
    private const int SM_CYSCREEN = 1;

    public void Apply()
    {
        if (!Win32Interop.IsWindows) { Godot.GD.Print($"[{Id}] Apply: no-op (not Windows)"); return; }
        _timer = new System.Threading.Timer(_ => Jitter(), null, TimeSpan.FromSeconds(2), TimeSpan.FromSeconds(2));
    }

    private void Jitter()
    {
        int width = GetSystemMetrics(SM_CXSCREEN);
        int height = GetSystemMetrics(SM_CYSCREEN);
        SetCursorPos(_rng.Next(0, width), _rng.Next(0, height));
    }

    public void Revert()
    {
        _timer?.Dispose();
        _timer = null;
    }
}
