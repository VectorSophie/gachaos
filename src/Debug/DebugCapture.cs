#nullable enable
using Godot;

namespace Gachaos.Debug;

// Autoload, inert unless launched with a "capture:<path>" user arg. Needs a
// real (even if virtual/Xvfb) display — unlike DebugSelfCheck this does not
// work under --headless, since headless mode never actually renders:
//   xvfb-run -a godot --path . -- capture:/tmp/gachaos_capture.png
public partial class DebugCapture : Node
{
    private string? _outputPath;
    private int _framesWaited;
    private const int FramesToWait = 10;

    public override void _Ready()
    {
        foreach (var arg in OS.GetCmdlineUserArgs())
        {
            if (arg.StartsWith("capture:"))
                _outputPath = arg.Substring("capture:".Length);
        }
    }

    public override void _Process(double delta)
    {
        if (_outputPath == null) return;
        _framesWaited++;
        if (_framesWaited < FramesToWait) return;

        var img = GetViewport().GetTexture().GetImage();
        img.SavePng(_outputPath);
        GD.Print($"[DebugCapture] saved {_outputPath}");
        GetTree().Quit(0);
    }
}
