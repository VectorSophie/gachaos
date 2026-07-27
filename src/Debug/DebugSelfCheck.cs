using Godot;
using System;
using Gachaos.Gacha;

namespace Gachaos.Debug;

// Autoload, inert unless launched with a "selfcheck" user arg:
//   godot --headless --path . -- selfcheck
public partial class DebugSelfCheck : Node
{
    public override void _Ready()
    {
        string[] args = OS.GetCmdlineUserArgs();
        if (Array.IndexOf(args, "selfcheck") < 0) return;

        bool ok = RarityRollerSelfCheck.Run(out string report);
        GD.Print(report);
        GetTree().Quit(ok ? 0 : 1);
    }
}
