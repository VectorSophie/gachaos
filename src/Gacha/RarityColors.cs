using Godot;

namespace Gachaos.Gacha;

public static class RarityColors
{
    public static Color Get(Rarity r) => r switch
    {
        Rarity.Common => new Color(0.2f, 0.2f, 0.2f),
        Rarity.Rare => new Color(0.11f, 0.35f, 0.85f),
        Rarity.Epic => new Color(0.55f, 0.15f, 0.75f),
        Rarity.Legendary => new Color(0.85f, 0.62f, 0.05f),
        _ => Colors.Black,
    };
}
