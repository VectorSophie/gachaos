using Godot;
using System;
using System.Text.Json;

namespace Gachaos.Economy;

public static class SaveManager
{
    private const string SavePath = "user://save.json";

    public static SaveData Load()
    {
        if (!Godot.FileAccess.FileExists(SavePath))
            return NewSaveData();

        using var file = Godot.FileAccess.Open(SavePath, Godot.FileAccess.ModeFlags.Read);
        string json = file.GetAsText();
        try
        {
            return JsonSerializer.Deserialize<SaveData>(json) ?? NewSaveData();
        }
        catch (JsonException)
        {
            GD.PrintErr("gachaos: save file corrupt, starting fresh");
            return NewSaveData();
        }
    }

    // New installs start fully charged, like most energy/stamina systems —
    // otherwise a first-time user hits a 1-hour wait before their first pull.
    private static SaveData NewSaveData() => new()
    {
        TicketCount = TicketEconomy.TicketCap,
        LastRegenUnixTime = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
    };

    public static void Save(SaveData data)
    {
        using var file = Godot.FileAccess.Open(SavePath, Godot.FileAccess.ModeFlags.Write);
        if (file == null)
        {
            GD.PrintErr($"gachaos: failed to open {SavePath} for writing, error={Godot.FileAccess.GetOpenError()}");
            return;
        }
        file.StoreString(JsonSerializer.Serialize(data));
    }
}
