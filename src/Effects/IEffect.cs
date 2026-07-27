using Gachaos.Gacha;

namespace Gachaos.Effects;

public interface IEffect
{
    string Id { get; }
    string DisplayName { get; }
    string Description { get; }
    Rarity Tier { get; }

    // Seconds the effect stays active before EffectScheduler auto-reverts it.
    // -1 means it requires a manual cure (still cleared by the panic hotkey).
    double DurationSeconds { get; }

    void Apply();
    void Revert();
}
