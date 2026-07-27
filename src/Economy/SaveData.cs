using System.Collections.Generic;

namespace Gachaos.Economy;

public class ActiveEffectRecord
{
    public string EffectId { get; set; } = "";
    public long ExpiryUnixTime { get; set; }
}

public class SaveData
{
    public int TicketCount { get; set; } = 0;
    public long LastRegenUnixTime { get; set; } = 0;
    public int PullsSinceEpicPlus { get; set; } = 0;
    public List<ActiveEffectRecord> ActiveEffects { get; set; } = new();
}
