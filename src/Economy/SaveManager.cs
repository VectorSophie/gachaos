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
            return new SaveData { LastRegenUnixTime = DateTimeOffset.UtcNow.ToUnixTimeSeconds() };

        using var file = Godot.FileAccess.Open(SavePath, Godot.FileAccess.ModeFlags.Read);
        string json = file.GetAsText();
        try
        {
            return JsonSerializer.Deserialize<SaveData>(json) ?? new SaveData();
        }
        catch (JsonException)
        {
            GD.PrintErr("gachaos: save file corrupt, starting fresh");
            return new SaveData { LastRegenUnixTime = DateTimeOffset.UtcNow.ToUnixTimeSeconds() };
        }
    }

    public static void Save(SaveData data)
    {
        using var file = Godot.FileAccess.Open(SavePath, Godot.FileAccess.ModeFlags.Write);
        file.StoreString(JsonSerializer.Serialize(data));
    }
}
