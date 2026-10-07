// UI flow test: rolls every tier in Chromium against the mocked Tauri bridge,
// checks the rolled curse is the one applied, and saves screenshots.
//   node tests/ui/run.js            (needs the `playwright` package)
const path = require('path');
const fs = require('fs');
const { chromium } = require('playwright');

const root = path.resolve(__dirname, '../..');
const out = path.join(__dirname, 'out');
fs.mkdirSync(out, { recursive: true });

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  await page.addInitScript({ path: path.join(__dirname, 'tauri-mock.js') });
  await page.goto('file://' + path.join(root, 'ui/index.html'));
  await page.waitForFunction(() => window.GACHAOS);
  await page.screenshot({ path: path.join(out, 'idle.png'), clip: { x: 0, y: 0, width: 400, height: 470 } });

  const failures = [];
  for (const tier of [0, 1, 2, 3]) {
    await page.evaluate(t => { GACHAOS.setForce(t); document.getElementById('dlglayer').textContent = ''; }, tier);
    const t0 = Date.now();
    await page.click('#pull');
    // Mid-reveal frame for the tiers that break out of the window.
    if (tier >= 2) {
      await page.waitForFunction(() => GACHAOS.O.title, null, { timeout: 15000 });
      await page.waitForTimeout(900);
      await page.screenshot({ path: path.join(out, `tier${tier}-reveal.png`) });
    }
    await page.waitForFunction(() => !GACHAOS.S.busy, null, { timeout: 20000 });
    const secs = ((Date.now() - t0) / 1000).toFixed(1);
    const res = await page.evaluate(() => {
      const pulls = __MOCK__.calls.filter(c => c[0] === 'pull');
      const lastPull = pulls[pulls.length - 1];
      const applies = __MOCK__.calls.filter(c => c[0] === 'apply_curse');
      const restored = __MOCK__.calls.filter(c => c[0] === 'fx_restore').length;
      const expands = __MOCK__.calls.filter(c => c[0] === 'fx_expand').length;
      return {
        rolled: __MOCK__.st.history[0].id, applied: applies[applies.length - 1][1].id, force: lastPull[1].force,
        dialog: document.querySelector('#dlglayer .dialog .ttl')?.textContent, active: __MOCK__.st.active?.id,
        expands, restored, appLeft: getComputedStyle(document.getElementById('app')).left,
      };
    });
    await page.screenshot({ path: path.join(out, `tier${tier}-done.png`), clip: { x: 0, y: 0, width: 400, height: 470 } });
    const ok = res.rolled === res.applied && res.active === res.rolled && res.force === tier && res.dialog === 'Curse applied'
      && res.expands === res.restored && res.appLeft === '0px';
    console.log(`tier ${tier}: ${ok ? 'ok' : 'FAIL'} in ${secs}s`, JSON.stringify(res));
    if (!ok) failures.push(tier);
  }

  // Out of tickets shows an error instead of rolling.
  await page.evaluate(() => { __MOCK__.st.tickets = 0; document.getElementById('dlglayer').textContent = ''; });
  await page.click('#pull');
  const noTickets = await page.evaluate(() => document.querySelector('#dlglayer .msg')?.textContent || '');
  const ntOk = noTickets.includes("out of tickets") && !(await page.evaluate(() => GACHAOS.S.busy));
  console.log(`no tickets: ${ntOk ? 'ok' : 'FAIL'}`);
  if (!ntOk) failures.push('no-tickets');

  console.log('page errors:', errors.length ? errors : 'none');
  await browser.close();
  if (failures.length || errors.length) process.exit(1);
})();
