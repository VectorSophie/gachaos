// Game rules: tickets, odds, pity, the catalog of curses and blessings, and
// what claiming a result does. Pure logic, no OS calls, so everything here is
// unit-tested on any platform.
use serde::{Deserialize, Serialize};

use crate::curses::Restore;

pub const TICKET_CAP: u32 = 24;
pub const REGEN_SECS: u64 = 3600; // +1 ticket per hour
pub const PITY: u32 = 20; // the 20th pull without Epic+ is forced Epic+
pub const PITY_LEGEND: u32 = 90; // the 90th pull without Legendary+ is forced Legendary+
pub const HISTORY_CAP: usize = 30;
pub const TIERS: usize = 6;
/// Common, Uncommon, Rare, Epic, Legendary, Mythic, per 1000 pulls.
pub const TIER_ODDS: [u32; TIERS] = [500, 280, 140, 60, 16, 4];
pub const TIER_NAMES: [&str; TIERS] = ["Common", "Uncommon", "Rare", "Epic", "Legendary", "Mythic"];
/// How long a curse of each tier lasts.
pub const TIER_SECS: [u64; TIERS] = [60, 120, 240, 360, 480, 600];
pub const SAVE_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Boon {
    Tickets(u32),
    FreePulls(u32),
    /// Doubles the odds of Rare or better for this many pulls.
    Luck(u32),
    /// Blocks this many curses.
    Shield(u32),
    /// No curses for this many seconds.
    Immunity(u64),
    /// This many free pulls, each guaranteed Epic or better.
    Midas(u32),
    /// Tickets back to full and two hours of immunity.
    Grace,
}

pub struct Entry {
    pub id: &'static str,
    pub name: &'static str,
    pub tier: u8,
    pub desc: &'static str,
    /// None for curses.
    pub boon: Option<Boon>,
}

impl Entry {
    pub fn is_curse(&self) -> bool {
        self.boon.is_none()
    }
}

const fn curse(id: &'static str, name: &'static str, tier: u8, desc: &'static str) -> Entry {
    Entry { id, name, tier, desc, boon: None }
}
const fn bless(id: &'static str, name: &'static str, tier: u8, desc: &'static str, boon: Boon) -> Entry {
    Entry { id, name, tier, desc, boon: Some(boon) }
}

/// Five curses and two blessings per tier.
pub const CATALOG: &[Entry] = &[
    curse("giant", "Giant Cursor", 0, "Your mouse cursor is four times its normal size."),
    curse("wrong", "Wrong-Handed", 0, "Your left and right mouse buttons are swapped."),
    curse("sloth", "Sloth Mouse", 0, "Your mouse pointer crawls at its slowest speed."),
    curse("trails", "Long Tail", 0, "Your pointer drags a long trail of ghost cursors."),
    curse("pal", "Pixel Pal", 0, "A small pixel creature wanders along the bottom of your screen."),
    bless("pocket", "Pocket Change", 0, "+1 ticket.", Boon::Tickets(1)),
    bless("freespin", "Free Spin", 0, "Your next pull costs nothing.", Boon::FreePulls(1)),
    curse("turbo", "Turbo Mouse", 1, "Your mouse pointer moves at its fastest speed."),
    curse("butter", "Butterfingers", 1, "Double-clicks have to be lightning fast to count."),
    curse("whisper", "Whisper Mode", 1, "Your volume drops to 10%."),
    curse("wobble", "Window Wobble", 1, "Whatever window you're using won't sit still."),
    curse("trail", "Rainbow Trail", 1, "A pixel rainbow follows your cursor everywhere."),
    bless("spare", "Spare Change", 1, "+2 tickets.", Boon::Tickets(2)),
    bless("clover", "Four-Leaf Clover", 1, "Your next 3 pulls have double the odds of Rare or better.", Boon::Luck(3)),
    curse("jitter", "Jitterbug Cursor", 2, "Your cursor keeps drifting away from where you point."),
    curse("wall", "Wallpaper Heist", 2, "Someone replaced your wallpaper."),
    curse("teleport", "Teleporter", 2, "Your cursor jumps to a random spot every few seconds."),
    curse("snow", "Pixel Snow", 2, "Snow falls across your screen and piles up at the bottom."),
    curse("fog", "Fog Bank", 2, "Dithered fog creeps in from the edges of your screen."),
    bless("lucky", "Lucky Break", 2, "+4 tickets.", Boon::Tickets(4)),
    bless("umbrella", "Umbrella", 2, "Blocks the next curse you roll.", Boon::Shield(1)),
    curse("upside", "Upside-Down World", 3, "Your screen is flipped 180 degrees."),
    curse("beep", "Beep Storm", 3, "Your volume goes up and random system beeps play."),
    curse("negative", "Negative Space", 3, "Every colour on your screen is inverted."),
    curse("silent", "Silent Film", 3, "Your whole screen turns black and white."),
    curse("spotlight", "Spotlight", 3, "Everything goes dark except a small circle around your cursor."),
    bless("windfall", "Windfall", 3, "+8 tickets.", Boon::Tickets(8)),
    bless("talisman", "Talisman", 3, "Blocks the next 2 curses you roll.", Boon::Shield(2)),
    curse("sideways", "Sideways Vision", 4, "Your screen is rotated 90 degrees."),
    curse("disco", "Disco Inferno", 4, "Your desktop flashes through disco colours."),
    curse("tooclose", "Too Close", 4, "Your screen is zoomed in and follows your cursor around."),
    curse("confetti", "Confetti Storm", 4, "Confetti pours over your desktop without stopping."),
    curse("ghost", "Ghost Hunt", 4, "A pixel ghost chases your cursor. Don't let it catch you."),
    bless("jackpot", "Jackpot", 4, "+16 tickets, even past the cap.", Boon::Tickets(16)),
    bless("sanctuary", "Sanctuary", 4, "No curse can touch you for the next 30 minutes.", Boon::Immunity(1800)),
    curse("possession", "The Possession", 5, "Upside-Down World, Negative Space and Jitterbug Cursor, all at once."),
    curse("pandemonium", "Pandemonium", 5, "One random curse from every tier below Mythic, all at once."),
    curse("doom", "Doom Clock", 5, "A giant countdown takes over your screen while Beep Storm and Teleporter run."),
    curse("bloodmoon", "Blood Moon", 5, "Your screen turns blood red and only a spotlight is left."),
    curse("eclipse", "Eclipse", 5, "Your screen tips sideways while the light around your cursor closes in."),
    bless("midas", "Midas Touch", 5, "Your next 5 pulls are free and guaranteed Epic or better.", Boon::Midas(5)),
    bless("grace", "Divine Grace", 5, "Tickets refilled to full and no curses for 2 hours.", Boon::Grace),
];

pub fn entry(id: &str) -> Option<&'static Entry> {
    CATALOG.iter().find(|e| e.id == id)
}

/// The building blocks curses are made of. Each part changes one thing, its
/// slot; parts sharing a slot can't run at once (two display rotations, two
/// screen colour effects). Overlay parts are drawn by gachaos's own
/// click-through window, so they work everywhere and vanish with the app.
pub struct Part {
    pub id: &'static str,
    pub slot: &'static str,
    pub overlay: bool,
}

const fn os(id: &'static str, slot: &'static str) -> Part {
    Part { id, slot, overlay: false }
}
const fn ov(id: &'static str, slot: &'static str) -> Part {
    Part { id, slot, overlay: true }
}

pub const PARTS: &[Part] = &[
    os("giant", "cursor-art"),
    os("wrong", "buttons"),
    os("sloth", "speed"),
    os("trails", "trails"),
    ov("pal", "pal"),
    os("turbo", "speed"),
    os("butter", "dblclick"),
    os("whisper", "volume"),
    os("wobble", "window"),
    ov("trail", "trail"),
    os("jitter", "cursor-move"),
    os("wall", "wallpaper"),
    os("teleport", "cursor-move"),
    ov("snow", "snow"),
    ov("fog", "fog"),
    os("upside", "display"),
    os("beep", "volume"),
    os("negative", "colour"),
    os("silent", "colour"),
    ov("spotlight", "spot"),
    os("sideways", "display"),
    os("disco", "wallpaper"),
    os("tooclose", "colour"),
    ov("confetti", "confetti"),
    ov("ghost", "ghost"),
    // Mythic-only parts.
    os("bloodred", "colour"),
    ov("redpulse", "pulse"),
    ov("doomclock", "clock"),
    ov("eclipse", "spot"),
];

pub fn part(id: &str) -> Option<&'static Part> {
    PARTS.iter().find(|p| p.id == id)
}

/// Whether this platform can run a part: everything on Windows, overlays elsewhere.
pub fn part_supported(id: &str) -> bool {
    part(id).is_some_and(|p| cfg!(windows) || p.overlay)
}

/// The fixed parts of a curse (Pandemonium is chosen at claim time).
fn fixed_parts(id: &str) -> &'static [&'static str] {
    match id {
        "possession" => &["upside", "negative", "jitter", "redpulse"],
        "doom" => &["doomclock", "beep", "teleport"],
        "bloodmoon" => &["bloodred", "spotlight", "redpulse"],
        "eclipse" => &["sideways", "eclipse", "snow"],
        _ => &[],
    }
}

/// Whether an entry can be rolled on this platform.
pub fn available(e: &Entry) -> bool {
    match (e.boon, e.id) {
        (Some(_), _) => true,
        (None, "pandemonium") => (0..5).all(|t| CATALOG.iter().any(|c| c.tier == t && c.is_curse() && part_supported(c.id))),
        (None, id) if !fixed_parts(id).is_empty() => fixed_parts(id).iter().all(|p| part_supported(p)),
        (None, id) => part_supported(id),
    }
}

/// The parts a curse applies. Pandemonium picks one curse from each lower
/// tier, skipping any whose slot is already taken.
pub fn curse_parts(id: &str, rng: &mut fastrand::Rng) -> Vec<&'static str> {
    if id == "pandemonium" {
        let mut out: Vec<&'static str> = vec!["redpulse"];
        for t in 0..5u8 {
            let mut pool: Vec<&'static str> = CATALOG
                .iter()
                .filter(|c| c.tier == t && c.is_curse() && part_supported(c.id))
                .map(|c| c.id)
                .filter(|p| !out.iter().any(|o| part(o).unwrap().slot == part(p).unwrap().slot))
                .collect();
            rng.shuffle(&mut pool);
            if let Some(p) = pool.first() {
                out.push(p);
            }
        }
        return out;
    }
    let fixed = fixed_parts(id);
    if fixed.is_empty() { entry(id).map(|e| vec![e.id]).unwrap_or_default() } else { fixed.to_vec() }
}

/// Odds of one specific entry, as "1 in N".
pub fn one_in(e: &Entry) -> u32 {
    let same_tier = CATALOG.iter().filter(|d| d.tier == e.tier && available(d)).count() as f64;
    (1000.0 * same_tier / TIER_ODDS[e.tier as usize] as f64).round() as u32
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PullRecord {
    /// Unix time in milliseconds.
    pub t: u64,
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveCurse {
    pub id: String,
    /// The parts actually applied (Pandemonium differs every time).
    #[serde(default)]
    pub parts: Vec<String>,
    /// Unix time in seconds when it auto-reverts.
    pub until: u64,
    /// Everything needed to undo it, persisted so a crash can still be undone.
    pub restore: Restore,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct State {
    /// Missing in version-1 saves, so it reads as 0 there and `migrate` runs.
    #[serde(default)]
    pub version: u32,
    pub tickets: u32,
    pub last_regen: u64,
    /// Pulls since the last Epic or better.
    pub pity: u32,
    /// Pulls since the last Legendary or better.
    pub pity_legend: u32,
    pub pulls: u32,
    pub counts: Vec<u32>,
    pub best: i32,
    pub best_name: String,
    pub history: Vec<PullRecord>,
    pub sound: bool,
    pub active: Option<ActiveCurse>,
    // Blessings in effect.
    pub free_pulls: u32,
    pub luck_pulls: u32,
    pub midas: u32,
    pub shields: u32,
    pub immune_until: u64,
}

impl Default for State {
    // New installs start full, like most stamina systems.
    fn default() -> Self {
        State {
            version: SAVE_VERSION,
            tickets: TICKET_CAP,
            last_regen: 0,
            pity: 0,
            pity_legend: 0,
            pulls: 0,
            counts: vec![0; TIERS],
            best: -1,
            best_name: String::new(),
            history: Vec::new(),
            sound: true,
            active: None,
            free_pulls: 0,
            luck_pulls: 0,
            midas: 0,
            shields: 0,
            immune_until: 0,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PullOutcome {
    pub tier: u8,
    pub id: &'static str,
    pub forced_by_pity: bool,
    pub free: bool,
}

#[derive(Debug, PartialEq)]
pub enum PullError {
    NoTickets,
}

/// What claiming a revealed result did.
#[derive(Debug, PartialEq)]
pub enum Claim {
    /// A blessing took effect.
    Blessed,
    /// A curse was rolled but a shield or immunity stopped it.
    Blocked(String),
    /// Apply these curse parts.
    Curse(Vec<&'static str>),
}

impl State {
    /// Brings a save from an older version up to date. Version 1 had four
    /// tiers (Common, Rare, Epic, Legendary).
    pub fn migrate(&mut self) {
        if self.version < 2 {
            let old = std::mem::take(&mut self.counts);
            let get = |i: usize| old.get(i).copied().unwrap_or(0);
            self.counts = vec![get(0), 0, get(1), get(2), get(3), 0];
            self.best = match self.best {
                0 => 0,
                1 => 2,
                2 => 3,
                3 => 4,
                b => b,
            };
            self.version = 2;
        }
        self.counts.resize(TIERS, 0);
    }

    /// Adds tickets from wall-clock time, so they accrue while the app is closed.
    /// Returns true when the ticket count changed.
    pub fn regen(&mut self, now: u64) -> bool {
        if self.tickets >= TICKET_CAP {
            self.last_regen = now;
            return false;
        }
        let n = now.saturating_sub(self.last_regen) / REGEN_SECS;
        if n == 0 {
            return false;
        }
        self.tickets = (self.tickets as u64 + n).min(TICKET_CAP as u64) as u32;
        self.last_regen += n * REGEN_SECS;
        true
    }

    /// Unix seconds of the next ticket, or None when full.
    pub fn next_ticket_at(&self) -> Option<u64> {
        (self.tickets < TICKET_CAP).then(|| self.last_regen + REGEN_SECS)
    }

    pub fn pull(&mut self, now: u64, now_ms: u64, rng: &mut fastrand::Rng, force: Option<u8>) -> Result<PullOutcome, PullError> {
        self.regen(now);
        let free = self.free_pulls > 0;
        if free {
            self.free_pulls -= 1;
        } else {
            if self.tickets == 0 {
                return Err(PullError::NoTickets);
            }
            if self.tickets >= TICKET_CAP {
                self.last_regen = now; // the regen clock starts when you dip below full
            }
            self.tickets -= 1;
        }

        let lucky = self.luck_pulls > 0;
        self.luck_pulls = self.luck_pulls.saturating_sub(1);
        let midas = self.midas > 0;
        self.midas = self.midas.saturating_sub(1);
        let forced_by_pity = force.is_none() && !midas && (self.pity_legend + 1 >= PITY_LEGEND || self.pity + 1 >= PITY);
        let floor = if self.pity_legend + 1 >= PITY_LEGEND { 4 } else if midas || self.pity + 1 >= PITY { 3 } else { 0 };
        let tier = match force {
            Some(t) => t.min(TIERS as u8 - 1),
            None => roll_tier(rng, floor, lucky),
        };
        let pool: Vec<&'static Entry> = CATALOG.iter().filter(|e| e.tier == tier && available(e)).collect();
        let e = pool[rng.usize(..pool.len())];

        self.pity = if tier >= 3 { 0 } else { self.pity + 1 };
        self.pity_legend = if tier >= 4 { 0 } else { self.pity_legend + 1 };
        self.pulls += 1;
        self.counts.resize(TIERS, 0);
        self.counts[tier as usize] += 1;
        if tier as i32 > self.best {
            self.best = tier as i32;
            self.best_name = e.name.to_string();
        }
        self.history.insert(0, PullRecord { t: now_ms, id: e.id.to_string() });
        self.history.truncate(HISTORY_CAP);
        Ok(PullOutcome { tier, id: e.id, forced_by_pity, free })
    }

    /// Applies a revealed result: blessings take effect here; curses come back
    /// as parts to apply unless a shield or immunity stops them.
    pub fn claim(&mut self, id: &str, now: u64, rng: &mut fastrand::Rng) -> Option<Claim> {
        let e = entry(id)?;
        if let Some(boon) = e.boon {
            match boon {
                Boon::Tickets(n) => self.tickets += n,
                Boon::FreePulls(n) => self.free_pulls += n,
                Boon::Luck(n) => self.luck_pulls += n,
                Boon::Shield(n) => self.shields += n,
                Boon::Immunity(s) => self.immune_until = self.immune_until.max(now) + s,
                Boon::Midas(n) => {
                    self.free_pulls += n;
                    self.midas += n;
                }
                Boon::Grace => {
                    self.tickets = self.tickets.max(TICKET_CAP);
                    self.immune_until = self.immune_until.max(now) + 7200;
                }
            }
            return Some(Claim::Blessed);
        }
        if self.immune_until > now {
            let left = self.immune_until - now;
            return Some(Claim::Blocked(format!("You're immune for another {}:{:02}.", left / 60, left % 60)));
        }
        if self.shields > 0 {
            self.shields -= 1;
            return Some(Claim::Blocked(match self.shields {
                0 => "Your shield blocked it. That was your last one.".into(),
                n => format!("Your shield blocked it. {n} left."),
            }));
        }
        Some(Claim::Curse(curse_parts(id, rng)))
    }
}

/// Rolls a tier no lower than `floor`. Luck doubles Rare and above, paid for out of Common.
pub fn roll_tier(rng: &mut fastrand::Rng, floor: u8, lucky: bool) -> u8 {
    let mut w = TIER_ODDS;
    if lucky {
        for t in 2..TIERS {
            w[0] -= w[t];
            w[t] *= 2;
        }
    }
    for t in w.iter_mut().take(floor as usize) {
        *t = 0;
    }
    let total: u32 = w.iter().sum();
    let mut roll = rng.u32(0..total);
    for (t, weight) in w.iter().enumerate() {
        if roll < *weight {
            return t as u8;
        }
        roll -= weight;
    }
    floor
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_has_five_curses_and_two_blessings_per_tier() {
        for t in 0..TIERS as u8 {
            let curses = CATALOG.iter().filter(|e| e.tier == t && e.is_curse()).count();
            let blessings = CATALOG.iter().filter(|e| e.tier == t && !e.is_curse()).count();
            assert_eq!((curses, blessings), (5, 2), "tier {t}");
        }
        let ids: HashSet<_> = CATALOG.iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), CATALOG.len(), "duplicate ids");
    }

    #[test]
    fn every_curse_resolves_to_known_parts_with_distinct_slots() {
        let mut rng = fastrand::Rng::with_seed(4);
        for e in CATALOG.iter().filter(|e| e.is_curse()) {
            for _ in 0..50 {
                let parts = curse_parts(e.id, &mut rng);
                assert!(!parts.is_empty(), "{} has no parts", e.id);
                let slots: HashSet<_> = parts.iter().map(|p| part(p).unwrap_or_else(|| panic!("unknown part {p}")).slot).collect();
                assert_eq!(slots.len(), parts.len(), "{} stacks two parts in one slot: {parts:?}", e.id);
            }
        }
    }

    #[test]
    fn pandemonium_takes_one_curse_from_each_lower_tier() {
        if !cfg!(windows) {
            return; // off Windows only overlay parts exist, so some tiers have nothing to pick
        }
        let mut rng = fastrand::Rng::with_seed(8);
        let parts = curse_parts("pandemonium", &mut rng);
        let tiers: HashSet<_> = parts.iter().filter_map(|p| entry(p)).map(|e| e.tier).collect();
        assert_eq!(tiers, (0..5).collect());
    }

    #[test]
    fn odds_are_close_to_the_table_and_both_pities_cap_the_drought() {
        let mut rng = fastrand::Rng::with_seed(12345);
        let mut st = State { tickets: u32::MAX / 2, ..State::default() };
        let total = 400_000;
        let (mut gap, mut max_gap, mut lgap, mut max_lgap) = (0, 0, 0, 0);
        for _ in 0..total {
            let o = st.pull(0, 0, &mut rng, None).unwrap();
            if o.tier >= 3 { gap = 0 } else { gap += 1 }
            if o.tier >= 4 { lgap = 0 } else { lgap += 1 }
            max_gap = max_gap.max(gap);
            max_lgap = max_lgap.max(lgap);
        }
        let pct = |t: usize| st.counts[t] as f64 * 1000.0 / total as f64;
        // Pity nudges the top tiers up a little, so allow for that.
        for (t, (want, tol)) in [(500.0, 30.0), (280.0, 15.0), (140.0, 10.0), (60.0, 15.0), (16.0, 8.0), (4.0, 3.0)].iter().enumerate() {
            assert!((pct(t) - want).abs() < *tol, "tier {t}: {} per mille, want {want}", pct(t));
        }
        assert!(max_gap < PITY, "went {max_gap} pulls without Epic+");
        assert!(max_lgap < PITY_LEGEND, "went {max_lgap} pulls without Legendary+");
    }

    #[test]
    fn floors_and_luck() {
        let mut rng = fastrand::Rng::with_seed(1);
        for _ in 0..500 {
            assert!(roll_tier(&mut rng, 3, false) >= 3);
            assert!(roll_tier(&mut rng, 4, false) >= 4);
        }
        let (mut plain, mut lucky) = (0, 0);
        for _ in 0..100_000 {
            plain += (roll_tier(&mut rng, 0, false) >= 2) as u32;
            lucky += (roll_tier(&mut rng, 0, true) >= 2) as u32;
        }
        let ratio = lucky as f64 / plain as f64;
        assert!((ratio - 2.0).abs() < 0.1, "luck ratio {ratio}");
    }

    #[test]
    fn force_overrides_odds() {
        let mut rng = fastrand::Rng::with_seed(7);
        let mut st = State::default();
        let o = st.pull(0, 0, &mut rng, Some(5)).unwrap();
        assert_eq!(o.tier, 5);
        assert_eq!(st.best, 5);
        assert_eq!((st.pity, st.pity_legend), (0, 0));
    }

    #[test]
    fn tickets_run_out_and_regen_hourly_up_to_the_cap() {
        let mut rng = fastrand::Rng::with_seed(3);
        let mut st = State::default();
        let t0 = 1_000_000;
        for _ in 0..TICKET_CAP {
            st.pull(t0, 0, &mut rng, None).unwrap();
        }
        assert_eq!(st.pull(t0, 0, &mut rng, None), Err(PullError::NoTickets));
        assert_eq!(st.next_ticket_at(), Some(t0 + REGEN_SECS));
        st.regen(t0 + REGEN_SECS * 3 + 10);
        assert_eq!(st.tickets, 3);
        st.regen(t0 + REGEN_SECS * 100);
        assert_eq!(st.tickets, TICKET_CAP);
        assert_eq!(st.next_ticket_at(), None);
    }

    #[test]
    fn blessings_take_effect() {
        let mut rng = fastrand::Rng::with_seed(5);
        let mut st = State { tickets: 0, ..State::default() };
        let now = 1000;
        assert_eq!(st.claim("jackpot", now, &mut rng), Some(Claim::Blessed));
        assert_eq!(st.tickets, 16);

        // Free pulls don't spend tickets.
        st.claim("freespin", now, &mut rng);
        assert!(st.pull(now, 0, &mut rng, None).unwrap().free);
        assert_eq!(st.tickets, 16);

        // Shields block curses one at a time, then curses land again.
        st.claim("talisman", now, &mut rng);
        assert!(matches!(st.claim("upside", now, &mut rng), Some(Claim::Blocked(_))));
        assert!(matches!(st.claim("upside", now, &mut rng), Some(Claim::Blocked(_))));
        assert_eq!(st.claim("upside", now, &mut rng), Some(Claim::Curse(vec!["upside"])));

        // Immunity blocks without spending shields, until it runs out.
        st.claim("umbrella", now, &mut rng);
        st.claim("sanctuary", now, &mut rng);
        assert!(matches!(st.claim("giant", now + 10, &mut rng), Some(Claim::Blocked(_))));
        assert_eq!(st.shields, 1);
        assert!(matches!(st.claim("giant", now + 1800, &mut rng), Some(Claim::Blocked(_))));
        assert_eq!(st.shields, 0);

        // Midas: free and Epic or better.
        let mut st = State { tickets: 0, ..State::default() };
        st.claim("midas", now, &mut rng);
        for _ in 0..5 {
            let o = st.pull(now, 0, &mut rng, None).unwrap();
            assert!(o.free && o.tier >= 3, "{o:?}");
        }
        assert_eq!(st.pull(now, 0, &mut rng, None), Err(PullError::NoTickets));

        // Grace refills and protects.
        let mut st = State { tickets: 2, ..State::default() };
        st.claim("grace", now, &mut rng);
        assert_eq!(st.tickets, TICKET_CAP);
        assert_eq!(st.immune_until, now + 7200);
    }

    #[test]
    fn history_is_newest_first_and_capped() {
        let mut rng = fastrand::Rng::with_seed(9);
        let mut st = State { tickets: 100, ..State::default() };
        for i in 0..40 {
            st.pull(0, i, &mut rng, None).unwrap();
        }
        assert_eq!(st.history.len(), HISTORY_CAP);
        assert_eq!(st.history[0].t, 39);
    }

    #[test]
    fn one_in_matches_the_chances_table() {
        if !cfg!(windows) {
            return; // off Windows, tiers shrink to the overlay curses
        }
        let get = |id| one_in(entry(id).unwrap());
        assert_eq!([get("giant"), get("turbo"), get("jitter"), get("upside"), get("sideways"), get("eclipse")], [14, 25, 50, 117, 438, 1750]);
    }

    #[test]
    fn every_tier_has_something_to_roll_on_this_platform() {
        for t in 0..TIERS as u8 {
            assert!(CATALOG.iter().any(|e| e.tier == t && e.is_curse() && available(e)), "tier {t} has no curses here");
        }
    }

    #[test]
    fn version_1_saves_migrate_to_six_tiers() {
        let mut old: State = serde_json::from_str(r#"{"tickets":3,"counts":[10,5,2,1],"best":3,"bestName":"Sideways Vision"}"#).unwrap();
        assert_eq!(old.version, 0);
        old.migrate();
        assert_eq!(old.version, SAVE_VERSION);
        assert_eq!(old.counts, vec![10, 0, 5, 2, 1, 0]);
        assert_eq!(old.best, 4);
        assert_eq!(old.tickets, 3);
    }

    #[test]
    fn save_round_trips() {
        let mut st = State { pity: 7, shields: 2, ..State::default() };
        st.active = Some(ActiveCurse {
            id: "wall".into(),
            parts: vec!["wall".into()],
            until: 5,
            restore: Restore { wallpaper: Some("C:\\x.jpg".into()), ..Default::default() },
        });
        let json = serde_json::to_string(&st).unwrap();
        assert_eq!(serde_json::from_str::<State>(&json).unwrap(), st);
    }
}
