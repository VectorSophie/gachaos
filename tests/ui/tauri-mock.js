// Stand-in for window.__TAURI__ so ui/index.html can be driven in a plain
// browser by tests. Mirrors the Rust commands closely enough for UI flows;
// the real rules are tested in src/game.rs.
(() => {
  const TIER_NAMES = ['Common', 'Uncommon', 'Rare', 'Epic', 'Legendary', 'Mythic'];
  const ONE_IN = [14, 25, 50, 117, 438, 1750];
  const SECS = [60, 120, 240, 360, 480, 600];
  const NAMES = [
    ['Giant Cursor', 'Wrong-Handed', 'Sloth Mouse', 'Long Tail', 'Pixel Pal', 'Pocket Change', 'Free Spin'],
    ['Turbo Mouse', 'Butterfingers', 'Whisper Mode', 'Window Wobble', 'Rainbow Trail', 'Spare Change', 'Four-Leaf Clover'],
    ['Jitterbug Cursor', 'Wallpaper Heist', 'Teleporter', 'Pixel Snow', 'Fog Bank', 'Lucky Break', 'Umbrella'],
    ['Upside-Down World', 'Beep Storm', 'Negative Space', 'Silent Film', 'Spotlight', 'Windfall', 'Talisman'],
    ['Sideways Vision', 'Disco Inferno', 'Too Close', 'Confetti Storm', 'Ghost Hunt', 'Jackpot', 'Sanctuary'],
    ['The Possession', 'Pandemonium', 'Doom Clock', 'Blood Moon', 'Eclipse', 'Midas Touch', 'Divine Grace'],
  ];
  const CATALOG = NAMES.flatMap((names, tier) => names.map((name, i) => ({
    id: `t${tier}e${i}`, name, tier, kind: i < 5 ? 'curse' : 'blessing', secs: i < 5 ? SECS[tier] : 0,
    desc: `Test entry: ${name}.`, oneIn: ONE_IN[tier],
  })));
  const st = { tickets: 24, pity: 0, pityLegend: 0, pulls: 0, counts: [0, 0, 0, 0, 0, 0], best: -1, bestName: '', history: [], sound: false,
    active: null, pending: null, shields: 0, freePulls: 0 };
  const calls = [];
  const mock = { st, calls, kind: null };
  const snap = () => ({
    tickets: st.tickets, cap: 24, pity: st.pity, pityThreshold: 20, pityLegend: st.pityLegend, pityLegendThreshold: 90,
    pulls: st.pulls, counts: st.counts.slice(), best: st.best, bestName: st.bestName, history: st.history.slice(), sound: st.sound, qa: true,
    nextTicketAtMs: st.tickets < 24 ? Date.now() + 3600e3 : null, active: st.active,
    boons: { freePulls: st.freePulls, luckPulls: 0, midas: 0, shields: st.shields, immuneUntilMs: 0 },
    recovered: null, tiers: TIER_NAMES, catalog: CATALOG,
  });
  const cmds = {
    get_state: () => snap(),
    pull: ({ force }) => {
      if (st.tickets <= 0 && !st.freePulls) throw 'no_tickets';
      st.active = null;
      const free = st.freePulls > 0;
      if (free) st.freePulls--; else st.tickets--;
      const tier = force ?? 0;
      const pool = CATALOG.filter(c => c.tier === tier && (!mock.kind || c.kind === mock.kind));
      const c = pool[Math.floor(Math.random() * pool.length)];
      st.pity = tier >= 3 ? 0 : st.pity + 1; st.pulls++; st.counts[tier]++;
      if (tier > st.best) { st.best = tier; st.bestName = c.name; }
      st.history.unshift({ t: Date.now(), id: c.id }); st.pending = c.id;
      return { tier, id: c.id, forcedByPity: false, free, state: snap() };
    },
    claim: ({ id }) => {
      if (st.pending !== id) throw "that result wasn't rolled";
      st.pending = null;
      const c = CATALOG.find(x => x.id === id);
      if (c.kind === 'blessing') { st.shields++; return { result: 'blessed', message: null, failed: [], state: snap() }; }
      st.active = { id, untilMs: Date.now() + c.secs * 1000, parts: [id] };
      return { result: 'cursed', message: null, failed: [], state: snap() };
    },
    cure_all: () => { st.active = null; return snap(); },
    set_sound: ({ on }) => { st.sound = on; },
    qa_refill: () => { st.tickets = 24; return snap(); },
    qa_reset: () => snap(),
    fit_window: () => {},
    // Pretend the window sat in the middle of a 1280x800 monitor.
    fx_expand: () => [440, 160],
    fx_restore: () => {},
    hide_to_tray: () => {},
    quit: () => {},
    ui_log: ({ msg }) => console.error('[ui_log] ' + msg),
  };
  window.__TAURI__ = {
    core: { invoke: async (cmd, args = {}) => { calls.push([cmd, args]); if (!cmds[cmd]) throw `unknown command ${cmd}`; return cmds[cmd](args); } },
    event: { listen: async () => () => {} },
    window: { getCurrentWindow: () => ({ startDragging() {}, minimize() {}, onFocusChanged: async () => () => {} }) },
  };
  window.__MOCK__ = mock;
})();
