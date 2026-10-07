// Game rules: tickets, odds, pity and the curse catalog. Pure logic, no OS
// calls, so everything here is unit-tested on any platform.
use serde::{Deserialize, Serialize};

use crate::curses::Restore;

pub const TICKET_CAP: u32 = 24;
pub const REGEN_SECS: u64 = 3600; // +1 ticket per hour
pub const PITY: u32 = 20; // the 20th pull without Epic+ is forced Epic+
pub const HISTORY_CAP: usize = 30;
/// Common, Rare, Epic, Legendary, in percent.
pub const TIER_ODDS: [u32; 4] = [60, 30, 8, 2];
pub const TIER_NAMES: [&str; 4] = ["Common", "Rare", "Epic", "Legendary"];

pub struct CurseDef {
    pub id: &'static str,
    pub name: &'static str,
    pub tier: u8,
    pub secs: u64,
    pub desc: &'static str,
}

pub const CURSES: &[CurseDef] = &[
    CurseDef { id: "giant", name: "Giant Cursor", tier: 0, secs: 120, desc: "Your mouse cursor is four times its normal size." },
    CurseDef { id: "wrong", name: "Wrong-Handed", tier: 0, secs: 120, desc: "Your left and right mouse buttons are swapped." },
    CurseDef { id: "jitter", name: "Jitterbug Cursor", tier: 1, secs: 300, desc: "Your cursor keeps drifting away from where you point." },
    CurseDef { id: "wall", name: "Wallpaper Heist", tier: 1, secs: 300, desc: "Someone replaced your wallpaper." },
    CurseDef { id: "upside", name: "Upside-Down World", tier: 2, secs: 480, desc: "Your screen is flipped 180 degrees." },
    CurseDef { id: "beep", name: "Beep Storm", tier: 2, secs: 480, desc: "Your volume goes up and random system beeps play." },
    CurseDef { id: "sideways", name: "Sideways Vision", tier: 3, secs: 600, desc: "Your screen is rotated 90 degrees." },
    CurseDef { id: "disco", name: "Disco Inferno", tier: 3, secs: 600, desc: "Your desktop flashes through disco colours." },
];

pub fn curse(id: &str) -> Option<&'static CurseDef> {
    CURSES.iter().find(|c| c.id == id)
}

/// Odds of one specific curse, as "1 in N".
pub fn one_in(c: &CurseDef) -> u32 {
    let same_tier = CURSES.iter().filter(|d| d.tier == c.tier).count() as f64;
    (100.0 * same_tier / TIER_ODDS[c.tier as usize] as f64).round() as u32
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
    /// Unix time in seconds when it auto-reverts.
    pub until: u64,
    /// Everything needed to undo it, persisted so a crash can still be undone.
    pub restore: Restore,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct State {
    pub tickets: u32,
    pub last_regen: u64,
    pub pity: u32,
    pub pulls: u32,
    pub counts: [u32; 4],
    pub best: i32,
    pub best_name: String,
    pub history: Vec<PullRecord>,
    pub sound: bool,
    pub active: Option<ActiveCurse>,
}

impl Default for State {
    // New installs start full, like most stamina systems.
    fn default() -> Self {
        State {
            tickets: TICKET_CAP,
            last_regen: 0,
            pity: 0,
            pulls: 0,
            counts: [0; 4],
            best: -1,
            best_name: String::new(),
            history: Vec::new(),
            sound: true,
            active: None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PullOutcome {
    pub tier: u8,
    pub id: &'static str,
    pub forced_by_pity: bool,
}

#[derive(Debug, PartialEq)]
pub enum PullError {
    NoTickets,
}

impl State {
    /// Adds tickets from wall-clock time, so they accrue while the app is closed.
    /// Returns true when the state changed.
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

    /// Seconds until the next ticket, or None when full.
    pub fn next_ticket_at(&self) -> Option<u64> {
        (self.tickets < TICKET_CAP).then(|| self.last_regen + REGEN_SECS)
    }

    pub fn pull(&mut self, now: u64, now_ms: u64, rng: &mut fastrand::Rng, force: Option<u8>) -> Result<PullOutcome, PullError> {
        self.regen(now);
        if self.tickets == 0 {
            return Err(PullError::NoTickets);
        }
        if self.tickets >= TICKET_CAP {
            self.last_regen = now; // the regen clock starts when you dip below full
        }
        self.tickets -= 1;

        let forced_by_pity = force.is_none() && self.pity >= PITY - 1;
        let tier = roll_tier(rng, self.pity, force);
        let pool: Vec<&CurseDef> = CURSES.iter().filter(|c| c.tier == tier).collect();
        let c = pool[rng.usize(..pool.len())];

        self.pity = if tier >= 2 { 0 } else { self.pity + 1 };
        self.pulls += 1;
        self.counts[tier as usize] += 1;
        if tier as i32 > self.best {
            self.best = tier as i32;
            self.best_name = c.name.to_string();
        }
        self.history.insert(0, PullRecord { t: now_ms, id: c.id.to_string() });
        self.history.truncate(HISTORY_CAP);
        Ok(PullOutcome { tier, id: c.id, forced_by_pity })
    }
}

pub fn roll_tier(rng: &mut fastrand::Rng, pity: u32, force: Option<u8>) -> u8 {
    if let Some(t) = force {
        return t.min(3);
    }
    if pity >= PITY - 1 {
        // Pity keeps Epic and Legendary at their usual 8:2 ratio.
        return if rng.u32(0..TIER_ODDS[2] + TIER_ODDS[3]) < TIER_ODDS[3] { 3 } else { 2 };
    }
    let mut roll = rng.u32(0..100);
    for tier in (0..4).rev() {
        if roll < TIER_ODDS[tier] {
            return tier as u8;
        }
        roll -= TIER_ODDS[tier];
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odds_are_close_to_the_table_and_pity_caps_the_drought() {
        let mut rng = fastrand::Rng::with_seed(12345);
        let mut st = State { tickets: u32::MAX / 2, ..State::default() };
        let total = 100_000;
        let mut gap = 0;
        let mut max_gap = 0;
        for _ in 0..total {
            let o = st.pull(0, 0, &mut rng, None).unwrap();
            if o.tier >= 2 { gap = 0 } else { gap += 1 }
            max_gap = max_gap.max(gap);
        }
        let pct = |t: usize| st.counts[t] as f64 * 100.0 / total as f64;
        // Pity nudges Epic+ up a little, so Common/Rare sit slightly below 60/30.
        assert!((pct(0) - 60.0).abs() < 2.5, "common {}", pct(0));
        assert!((pct(1) - 30.0).abs() < 2.0, "rare {}", pct(1));
        assert!((pct(2) - 8.0).abs() < 1.5, "epic {}", pct(2));
        assert!((pct(3) - 2.0).abs() < 1.0, "legendary {}", pct(3));
        assert!(max_gap < PITY, "went {max_gap} pulls without Epic+");
    }

    #[test]
    fn pity_forces_epic_or_better_on_the_20th_pull() {
        let mut rng = fastrand::Rng::with_seed(1);
        for _ in 0..200 {
            assert!(roll_tier(&mut rng, PITY - 1, None) >= 2);
        }
    }

    #[test]
    fn force_overrides_odds() {
        let mut rng = fastrand::Rng::with_seed(7);
        let mut st = State::default();
        let o = st.pull(0, 0, &mut rng, Some(3)).unwrap();
        assert_eq!(o.tier, 3);
        assert_eq!(st.best, 3);
        assert_eq!(st.pity, 0);
        assert!(!o.forced_by_pity);
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
        let get = |id| one_in(curse(id).unwrap());
        assert_eq!(get("giant"), 3);
        assert_eq!(get("jitter"), 7);
        assert_eq!(get("upside"), 25);
        assert_eq!(get("sideways"), 100);
        for t in 0..4u8 {
            assert!(CURSES.iter().any(|c| c.tier == t), "tier {t} has no curses");
        }
    }

    #[test]
    fn save_round_trips_and_tolerates_missing_fields() {
        let mut st = State::default();
        st.pity = 7;
        st.active = Some(ActiveCurse { id: "wall".into(), until: 5, restore: Restore { wallpaper: Some("C:\\x.jpg".into()), ..Default::default() } });
        let json = serde_json::to_string(&st).unwrap();
        assert_eq!(serde_json::from_str::<State>(&json).unwrap(), st);
        let old: State = serde_json::from_str(r#"{"tickets":3}"#).unwrap();
        assert_eq!(old.tickets, 3);
        assert_eq!(old.best, -1);
    }
}
