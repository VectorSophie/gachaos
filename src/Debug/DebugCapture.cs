#nullable enable
using Godot;

namespace Gachaos.Debug;

// Autoload, inert unless launched with a "capture:<path>" user arg. Needs a
// real (even if virtual/Xvfb) display — unlike DebugSelfCheck this does not
// work under --headless, since headless mode never actually renders:
//   xvfb-run -a godot --path . -- capture:/tmp/gachaos_capture.png simulatepulls:5
public partial class DebugCapture : Node
{
    private string? _outputPath;
    private int _simulatePulls;
    private int _framesWaited;
    private const int FramesToWait = 10;
    private const int SimulateAtFrame = 3; // after the main scene autoload-inits

    public override void _Ready()
    {
        foreach (var arg in OS.GetCmdlineUserArgs())
        {
            if (arg.StartsWith("capture:"))
                _outputPath = arg.Substring("capture:".Length);
            else if (arg.StartsWith("simulatepulls:"))
                int.TryParse(arg.Substring("simulatepulls:".Length), out _simulatePulls);
        }
    }

    public override void _Process(double delta)
    {
        if (_outputPath == null) return;
        _framesWaited++;

        if (_framesWaited == SimulateAtFrame && _simulatePulls > 0)
        {
            var mainWindow = GetTree().Root.GetNode<Gachaos.UI.MainWindow>("Main");
            for (int i = 0; i < _simulatePulls; i++)
                mainWindow.DebugSimulatePull();
        }

        if (_framesWaited < FramesToWait) return;

        var img = GetViewport().GetTexture().GetImage();
        img.SavePng(_outputPath);
        GD.Print($"[DebugCapture] saved {_outputPath}");
        GetTree().Quit(0);
    }
}
