using Gachaos.Gacha;
using Gachaos.Effects.Native;

namespace Gachaos.Effects.Curses;

// TODO: real implementation needs ChangeDisplaySettingsEx + DEVMODE
// (dmDisplayOrientation, with width/height swap) — deferred, verify on real
// Windows. Wired as a no-op stub for now so the full 4-tier roster and pull
// loop work end-to-end.
public class ScreenRotate90Effect : IEffect
{
    public string Id => "screen_rotate_90";
    public string DisplayName => "Sideways Vision";
    public string Description => "Your screen is rotated 90 degrees.";
    public Rarity Tier => Rarity.Legendary;
    public double DurationSeconds => 600;

    public void Apply()
    {
        Godot.GD.Print($"[{Id}] Apply: not yet implemented (needs display-orientation interop)");
    }

    public void Revert()
    {
        Godot.GD.Print($"[{Id}] Revert: not yet implemented (needs display-orientation interop)");
    }
}
