using Godot;

namespace Gachaos.Gacha;

public enum Rarity
{
    Common,
    Rare,
    Epic,
    Legendary
}

public static class RarityRoller
{
    private const int CommonWeight = 60;
    private const int RareWeight = 30;
    private const int EpicWeight = 8;
    private const int LegendaryWeight = 2;

    // If no Epic+ has landed in this many pulls, the next pull is forced Epic+.
    public const int PityThreshold = 20;

    public static bool IsEpicOrHigher(Rarity r) => r == Rarity.Epic || r == Rarity.Legendary;

    public static Rarity Roll(RandomNumberGenerator rng, int pullsSinceEpicPlus)
    {
        if (pullsSinceEpicPlus >= PityThreshold - 1)
            return RollEpicOrLegendary(rng);

        int roll = rng.RandiRange(1, 100);
        if (roll <= LegendaryWeight) return Rarity.Legendary;
        roll -= LegendaryWeight;
        if (roll <= EpicWeight) return Rarity.Epic;
        roll -= EpicWeight;
        if (roll <= RareWeight) return Rarity.Rare;
        return Rarity.Common;
    }

    private static Rarity RollEpicOrLegendary(RandomNumberGenerator rng)
    {
        int roll = rng.RandiRange(1, EpicWeight + LegendaryWeight);
        return roll <= LegendaryWeight ? Rarity.Legendary : Rarity.Epic;
    }
}
