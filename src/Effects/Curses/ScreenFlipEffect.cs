using Gachaos.Gacha;
using Gachaos.Effects.Native;

namespace Gachaos.Effects.Curses;

// TODO: real implementation needs ChangeDisplaySettingsEx + DEVMODE
// (dmDisplayOrientation) — deferred, verify on real Windows. Wired as a
// no-op stub for now so the full 4-tier roster and pull loop work end-to-end.
public class ScreenFlipEffect : IEffect
{
    public string Id => "screen_flip_180";
    public string DisplayName => "Upside-Down World";
    public string Description => "Your screen is flipped 180 degrees.";
    public Rarity Tier => Rarity.Epic;
    public double DurationSeconds => 480;

    public void Apply()
    {
        Godot.GD.Print($"[{Id}] Apply: not yet implemented (needs display-orientation interop)");
    }

    public void Revert()
    {
        Godot.GD.Print($"[{Id}] Revert: not yet implemented (needs display-orientation interop)");
    }
}
