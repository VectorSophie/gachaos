using Godot;
using Gachaos.Economy;
using Gachaos.Effects;
using Gachaos.Gacha;

namespace Gachaos.UI;

public partial class MainWindow : Control
{
    private TicketEconomy _economy = null!;
    private EffectScheduler _scheduler = null!;
    private readonly RandomNumberGenerator _rng = new();

    private Label _ticketLabel = null!;
    private Label _bestLabel = null!;
    private VBoxContainer _historyList = null!;
    private Button _pullButton = null!;

    public override void _Ready()
    {
        _economy = GetNode<TicketEconomy>("/root/TicketEconomy");
        _scheduler = GetNode<EffectScheduler>("/root/EffectScheduler");

        _ticketLabel = GetNode<Label>("%TicketLabel");
        _bestLabel = GetNode<Label>("%PersonalBestLabel");
        _historyList = GetNode<VBoxContainer>("%HistoryList");
        _pullButton = GetNode<Button>("%PullButton");

        _pullButton.Pressed += OnPullPressed;
        _economy.TicketsChanged += _ => RefreshTicketLabel();

        RefreshAll();
    }

    private void OnPullPressed()
    {
        if (!_economy.TrySpendTicket()) return;

        var tier = RarityRoller.Roll(_rng, _economy.Data.PullsSinceEpicPlus);
        _scheduler.ApplyPullResult(tier);

        RefreshAll();
    }

    // Exercises the exact same path a real click takes — used by
    // DebugCapture to populate history/stats for visual verification.
    public void DebugSimulatePull() => OnPullPressed();

    private void RefreshAll()
    {
        RefreshTicketLabel();
        RefreshBestLabel();
        RefreshHistoryList();
    }

    private void RefreshTicketLabel()
    {
        var data = _economy.Data;
        _ticketLabel.Text = $"Tickets: {data.TicketCount} / {TicketEconomy.TicketCap}";
        _pullButton.Disabled = data.TicketCount <= 0;
    }

    private void RefreshBestLabel()
    {
        var data = _economy.Data;
        _bestLabel.Text = data.BestRarityIndex < 0
            ? "Personal best: none yet"
            : $"Personal best: {(Rarity)data.BestRarityIndex} — {data.BestPullName}";
    }

    private void RefreshHistoryList()
    {
        foreach (Node child in _historyList.GetChildren())
            child.QueueFree();

        var data = _economy.Data;
        if (data.PullHistory.Count == 0)
        {
            _historyList.AddChild(new Label { Text = "No pulls yet." });
            return;
        }

        foreach (var record in data.PullHistory)
        {
            var row = new Label { Text = $"[{record.Tier}] {record.DisplayName}" };
            row.AddThemeColorOverride("font_color", RarityColors.Get(record.Tier));
            _historyList.AddChild(row);
        }
    }
}
