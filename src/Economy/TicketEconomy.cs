using Godot;
using System;

namespace Gachaos.Economy;

public partial class TicketEconomy : Node
{
    public const int TicketCap = 24;
    public const int RegenIntervalSeconds = 3600; // +1 ticket per hour

    public SaveData Data { get; private set; } = new();

    [Signal]
    public delegate void TicketsChangedEventHandler(int newCount);

    public override void _Ready()
    {
        Data = SaveManager.Load();
        ApplyOfflineRegen();

        var timer = new Timer { WaitTime = 60.0, Autostart = true };
        timer.Timeout += OnRegenTick;
        AddChild(timer);
    }

    // Regen is computed from elapsed wall-clock time (not a running counter),
    // so tickets accrue correctly even while the app isn't open.
    private void ApplyOfflineRegen()
    {
        long now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        long elapsed = now - Data.LastRegenUnixTime;
        if (elapsed <= 0) return;

        int ticksToAdd = (int)(elapsed / RegenIntervalSeconds);
        if (ticksToAdd > 0)
        {
            Data.TicketCount = Math.Min(TicketCap, Data.TicketCount + ticksToAdd);
            Data.LastRegenUnixTime += (long)ticksToAdd * RegenIntervalSeconds;
            SaveManager.Save(Data);
        }
    }

    private void OnRegenTick()
    {
        ApplyOfflineRegen();
        EmitSignal(SignalName.TicketsChanged, Data.TicketCount);
    }

    public bool TrySpendTicket()
    {
        if (Data.TicketCount <= 0) return false;
        Data.TicketCount--;
        SaveManager.Save(Data);
        EmitSignal(SignalName.TicketsChanged, Data.TicketCount);
        return true;
    }

    public double SecondsUntilNextTicket()
    {
        if (Data.TicketCount >= TicketCap) return -1;
        long now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        long nextTick = Data.LastRegenUnixTime + RegenIntervalSeconds;
        return Math.Max(0, nextTick - now);
    }
}
