using Godot;
using System;
using System.Collections.Generic;
using System.Linq;
using Gachaos.Economy;
using Gachaos.Effects.Curses;
using Gachaos.Gacha;

namespace Gachaos.Effects;

public partial class EffectScheduler : Node
{
    public const int HistoryCap = 20;

    private readonly Dictionary<string, IEffect> _registry = new();
    private readonly RandomNumberGenerator _pickRng = new();

    public IReadOnlyDictionary<string, IEffect> Registry => _registry;

    private TicketEconomy Economy => GetNode<TicketEconomy>("/root/TicketEconomy");

    public override void _Ready()
    {
        // TODO(task 5 continuation): giant cursor (needs custom icon
        // generation) and volume mute (needs Core Audio COM interop) both
        // need real Win32 work; ScreenFlip/ScreenRotate90 need
        // ChangeDisplaySettingsEx. All four are no-op stubs for now, verify
        // on real Windows before treating any of them as done.
        Register(new SwapMouseButtonsEffect());
        Register(new CursorJitterEffect());
        Register(new ScreenFlipEffect());
        Register(new ScreenRotate90Effect());

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

    // Picks a random registered effect matching the rolled tier, applies it,
    // and records it into history/stats/pity. Returns the chosen effect so
    // the caller (Pull button handler) can show it.
    public IEffect ApplyPullResult(Rarity tier)
    {
        var candidates = _registry.Values.Where(e => e.Tier == tier).ToList();
        var effect = candidates[_pickRng.RandiRange(0, candidates.Count - 1)];

        effect.Apply();

        var data = Economy.Data;
        data.ActiveEffects.RemoveAll(r => r.EffectId == effect.Id);
        if (effect.DurationSeconds > 0)
        {
            long expiry = DateTimeOffset.UtcNow.ToUnixTimeSeconds() + (long)effect.DurationSeconds;
            data.ActiveEffects.Add(new ActiveEffectRecord { EffectId = effect.Id, ExpiryUnixTime = expiry });
        }

        data.PullHistory.Insert(0, new PullRecord
        {
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            EffectId = effect.Id,
            DisplayName = effect.DisplayName,
            Tier = effect.Tier,
        });
        if (data.PullHistory.Count > HistoryCap)
            data.PullHistory.RemoveRange(HistoryCap, data.PullHistory.Count - HistoryCap);

        data.TotalPulls++;
        data.RarityCounts[(int)effect.Tier]++;
        if (effect.Tier > (Rarity)data.BestRarityIndex)
        {
            data.BestRarityIndex = (int)effect.Tier;
            data.BestPullName = effect.DisplayName;
        }

        data.PullsSinceEpicPlus = RarityRoller.IsEpicOrHigher(effect.Tier) ? 0 : data.PullsSinceEpicPlus + 1;

        SaveManager.Save(data);
        return effect;
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
