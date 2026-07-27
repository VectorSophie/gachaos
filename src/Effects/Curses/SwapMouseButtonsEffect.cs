using System.Runtime.InteropServices;
using Gachaos.Gacha;
using Gachaos.Effects.Native;

namespace Gachaos.Effects.Curses;

public class SwapMouseButtonsEffect : IEffect
{
    public string Id => "swap_mouse_buttons";
    public string DisplayName => "Wrong-Handed";
    public string Description => "Your left and right mouse buttons are swapped.";
    public Rarity Tier => Rarity.Common;
    public double DurationSeconds => 120;

    [DllImport("user32.dll")]
    private static extern bool SwapMouseButton(bool fSwap);

    public void Apply()
    {
        if (!Win32Interop.IsWindows) { Godot.GD.Print($"[{Id}] Apply: no-op (not Windows)"); return; }
        SwapMouseButton(true);
    }

    public void Revert()
    {
        if (!Win32Interop.IsWindows) { Godot.GD.Print($"[{Id}] Revert: no-op (not Windows)"); return; }
        SwapMouseButton(false);
    }
}
