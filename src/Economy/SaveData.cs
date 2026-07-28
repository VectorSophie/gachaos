using System.Collections.Generic;
using Gachaos.Gacha;

namespace Gachaos.Economy;

public class ActiveEffectRecord
{
    public string EffectId { get; set; } = "";
    public long ExpiryUnixTime { get; set; }
}

public class PullRecord
{
    public long Timestamp { get; set; }
    public string EffectId { get; set; } = "";
    public string DisplayName { get; set; } = "";
    public Rarity Tier { get; set; }
}

public class SaveData
{
    public int TicketCount { get; set; } = 0;
    public long LastRegenUnixTime { get; set; } = 0;
    public int PullsSinceEpicPlus { get; set; } = 0;
    public List<ActiveEffectRecord> ActiveEffects { get; set; } = new();

    public int TotalPulls { get; set; } = 0;
    public int[] RarityCounts { get; set; } = new int[4];
    public int BestRarityIndex { get; set; } = -1;
    public string BestPullName { get; set; } = "";

    // Most recent first, capped by EffectScheduler.HistoryCap.
    public List<PullRecord> PullHistory { get; set; } = new();
}
