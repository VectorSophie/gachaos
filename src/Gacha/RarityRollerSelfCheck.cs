using Godot;
using System;

namespace Gachaos.Gacha;

// ponytail: assert-based self-check in lieu of a full test framework/project.
// Run via: godot --headless --path . -- selfcheck
public static class RarityRollerSelfCheck
{
    public static bool Run(out string report)
    {
        var rng = new RandomNumberGenerator();
        rng.Seed = 12345;

        const int total = 100_000;
        int[] counts = new int[4];
        int pullsSinceEpicPlus = 0;
        int gap = 0;
        int maxGapObserved = 0;

        for (int i = 0; i < total; i++)
        {
            bool forcedByPity = pullsSinceEpicPlus >= RarityRoller.PityThreshold - 1;
            var result = RarityRoller.Roll(rng, pullsSinceEpicPlus);
            counts[(int)result]++;

            if (RarityRoller.IsEpicOrHigher(result))
            {
                gap = 0;
                pullsSinceEpicPlus = 0;
            }
            else
            {
                if (forcedByPity)
                {
                    report = $"FAIL: pity should have forced Epic+ at gap {pullsSinceEpicPlus} but rolled {result}";
                    return false;
                }
                gap++;
                pullsSinceEpicPlus++;
            }
            maxGapObserved = Math.Max(maxGapObserved, gap);
        }

        double commonPct = counts[(int)Rarity.Common] * 100.0 / total;
        double rarePct = counts[(int)Rarity.Rare] * 100.0 / total;
        double epicPct = counts[(int)Rarity.Epic] * 100.0 / total;
        double legendaryPct = counts[(int)Rarity.Legendary] * 100.0 / total;

        report = $"rolls={total} common={commonPct:F2}% rare={rarePct:F2}% epic={epicPct:F2}% " +
                 $"legendary={legendaryPct:F2}% maxGapWithoutEpicPlus={maxGapObserved} (threshold={RarityRoller.PityThreshold})";

        bool withinTolerance =
            Math.Abs(commonPct - 60) < 2 &&
            Math.Abs(rarePct - 30) < 2 &&
            Math.Abs(epicPct - 8) < 1.5 &&
            Math.Abs(legendaryPct - 2) < 1;

        if (!withinTolerance)
        {
            report = "FAIL: distribution out of tolerance. " + report;
            return false;
        }

        if (maxGapObserved > RarityRoller.PityThreshold)
        {
            report = "FAIL: pity did not cap the gap. " + report;
            return false;
        }

        report = "PASS: " + report;
        return true;
    }
}
