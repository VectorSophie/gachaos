using Godot;
using System;
using System.Collections.Generic;
using Gachaos.Economy;
using Gachaos.Effects.Curses;

namespace Gachaos.Effects;

public partial class EffectScheduler : Node
{
    private readonly Dictionary<string, IEffect> _registry = new();

    public IReadOnlyDictionary<string, IEffect> Registry => _registry;

    private TicketEconomy Economy => GetNode<TicketEconomy>("/root/TicketEconomy");

    public override void _Ready()
    {
        // TODO(task 5 continuation): giant cursor (needs custom icon
        // generation) and volume mute (needs Core Audio COM interop) — both
        // need a real Windows box to verify, tracked separately.
        Register(new SwapMouseButtonsEffect());
        Register(new CursorJitterEffect());

        RecoverFromCrash();

        var timer = new Timer { WaitTime = 1.0, Autostart = true };
        timer.Timeout += Tick;
        AddChild(timer);
    }

    private void Register(IEffect effect) => _registry[effect.Id] = effect;

    // If the app was killed while an effect was active, its expiry was
    // persisted but Revert() never ran. We can't know how much time was left,
    // so revert immediately rather than leave it stuck.
    private void RecoverFromCrash()
    {
        var data = Economy.Data;
        if (data.ActiveEffects.Count == 0) return;

        foreach (var record in new List<ActiveEffectRecord>(data.ActiveEffects))
        {
            if (_registry.TryGetValue(record.EffectId, out var effect))
                effect.Revert();
        }
        data.ActiveEffects.Clear();
        SaveManager.Save(data);
    }

    public void ApplyPullResult(IEffect effect)
    {
        effect.Apply();

        var data = Economy.Data;
        data.ActiveEffects.RemoveAll(r => r.EffectId == effect.Id);
        if (effect.DurationSeconds > 0)
        {
            long expiry = DateTimeOffset.UtcNow.ToUnixTimeSeconds() + (long)effect.DurationSeconds;
            data.ActiveEffects.Add(new ActiveEffectRecord { EffectId = effect.Id, ExpiryUnixTime = expiry });
        }
        SaveManager.Save(data);
    }

    // Global panic hotkey (Ctrl+Alt+P) reverts everything. Current binding
    // only fires while the app window has focus — reaching it while
    // tray-minimized needs a Win32 RegisterHotKey message-loop hook, tracked
    // as a task 8 follow-up.
    public override void _UnhandledKeyInput(InputEvent @event)
    {
        if (@event is InputEventKey { Pressed: true, CtrlPressed: true, AltPressed: true, Keycode: Key.P })
            PanicRevertAll();
    }

    public void PanicRevertAll()
    {
        var data = Economy.Data;
        foreach (var record in data.ActiveEffects)
        {
            if (_registry.TryGetValue(record.EffectId, out var effect))
                effect.Revert();
        }
        data.ActiveEffects.Clear();
        SaveManager.Save(data);
    }

    private void Tick()
    {
        var data = Economy.Data;
        long now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var expired = data.ActiveEffects.FindAll(r => r.ExpiryUnixTime <= now);
        if (expired.Count == 0) return;

        foreach (var record in expired)
        {
            if (_registry.TryGetValue(record.EffectId, out var effect))
                effect.Revert();
            data.ActiveEffects.Remove(record);
        }
        SaveManager.Save(data);
    }
}
