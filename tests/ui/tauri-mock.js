// Stand-in for window.__TAURI__ so ui/index.html can be driven in a plain
// browser by tests. Mirrors the Rust commands closely enough for UI flows;
// the real rules are tested in src/game.rs.
(() => {
  const CATALOG = [
    { id: 'giant', name: 'Giant Cursor', tier: 0, secs: 120, desc: 'Your mouse cursor is four times its normal size.', oneIn: 3 },
    { id: 'wrong', name: 'Wrong-Handed', tier: 0, secs: 120, desc: 'Your left and right mouse buttons are swapped.', oneIn: 3 },
    { id: 'jitter', name: 'Jitterbug Cursor', tier: 1, secs: 300, desc: 'Your cursor keeps drifting away from where you point.', oneIn: 7 },
    { id: 'wall', name: 'Wallpaper Heist', tier: 1, secs: 300, desc: 'Someone replaced your wallpaper.', oneIn: 7 },
    { id: 'upside', name: 'Upside-Down World', tier: 2, secs: 480, desc: 'Your screen is flipped 180 degrees.', oneIn: 25 },
    { id: 'beep', name: 'Beep Storm', tier: 2, secs: 480, desc: 'Your volume goes up and random system beeps play.', oneIn: 25 },
    { id: 'sideways', name: 'Sideways Vision', tier: 3, secs: 600, desc: 'Your screen is rotated 90 degrees.', oneIn: 100 },
    { id: 'disco', name: 'Disco Inferno', tier: 3, secs: 600, desc: 'Your desktop flashes through disco colours.', oneIn: 100 },
  ];
  const st = { tickets: 24, pity: 0, pulls: 0, counts: [0, 0, 0, 0], best: -1, bestName: '', history: [], sound: false, active: null, pending: null };
  const calls = [];
  const snap = () => ({
    tickets: st.tickets, cap: 24, pity: st.pity, pityThreshold: 20, pulls: st.pulls, counts: st.counts.slice(), best: st.best,
    bestName: st.bestName, history: st.history.slice(), sound: st.sound, qa: true,
    nextTicketAtMs: st.tickets < 24 ? Date.now() + 3600e3 : null, active: st.active, recovered: null,
    tiers: ['Common', 'Rare', 'Epic', 'Legendary'], catalog: CATALOG,
  });
  const cmds = {
    get_state: () => snap(),
    pull: ({ force }) => {
      if (st.tickets <= 0) throw 'no_tickets';
      st.active = null; st.tickets--;
      const tier = force ?? 0;
      const pool = CATALOG.filter(c => c.tier === tier);
      const c = pool[Math.floor(Math.random() * pool.length)];
      st.pity = tier >= 2 ? 0 : st.pity + 1; st.pulls++; st.counts[tier]++;
      if (tier > st.best) { st.best = tier; st.bestName = c.name; }
      st.history.unshift({ t: Date.now(), id: c.id }); st.pending = c.id;
      return { tier, curseId: c.id, forcedByPity: false, state: snap() };
    },
    apply_curse: ({ id }) => {
      if (st.pending !== id) throw "that curse wasn't rolled";
      st.pending = null;
      const c = CATALOG.find(x => x.id === id);
      st.active = { id, untilMs: Date.now() + c.secs * 1000 };
      return snap();
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
  };
  window.__TAURI__ = {
    core: { invoke: async (cmd, args = {}) => { calls.push([cmd, args]); if (!cmds[cmd]) throw `unknown command ${cmd}`; return cmds[cmd](args); } },
    event: { listen: async () => () => {} },
    window: { getCurrentWindow: () => ({ startDragging() {}, minimize() {} }) },
  };
  window.__MOCK__ = { st, calls };
})();
