// UI flow test: rolls every tier (as a curse and as a blessing) in Chromium
// against the mocked Tauri bridge, checks the rolled result is the one
// claimed, then covers closing with a curse active and the tray flyout.
// Saves screenshots to tests/ui/out/.
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
  await page.screenshot({ path: path.join(out, 'idle.png'), clip: { x: 0, y: 0, width: 400, height: 500 } });

  const failures = [];
  const dismiss = () => page.evaluate(() => { document.getElementById('dlglayer').textContent = ''; });
  const roll = async (tier, kind, shots) => {
    await page.waitForFunction(() => !GACHAOS.S.busy, null, { timeout: 30000 });
    await page.evaluate(([t, k]) => { GACHAOS.setForce(t); __MOCK__.kind = k; __MOCK__.st.tickets = 24; }, [tier, kind]);
    await dismiss();
    const t0 = Date.now();
    await page.click('#pull');
    for (const [label, ms] of shots) {
      await page.waitForFunction(() => GACHAOS.S.phase === 'result', null, { timeout: 20000 });
      await page.waitForTimeout(ms);
      await page.screenshot({ path: path.join(out, `t${tier}-${kind}-${label}.png`) });
    }
    await page.waitForFunction(() => !GACHAOS.S.busy, null, { timeout: 30000 });
    const secs = ((Date.now() - t0) / 1000).toFixed(1);
    const res = await page.evaluate(() => {
      const last = c => __MOCK__.calls.filter(x => x[0] === c).pop();
      return {
        rolled: __MOCK__.st.history[0].id, claimed: last('claim')[1].id, force: last('pull')[1].force,
        dialog: document.querySelector('#dlglayer .dialog .ttl')?.textContent,
        expands: __MOCK__.calls.filter(c => c[0] === 'fx_expand').length,
        restores: __MOCK__.calls.filter(c => c[0] === 'fx_restore').length,
        appLeft: getComputedStyle(document.getElementById('app')).left,
        appVisible: getComputedStyle(document.getElementById('app')).visibility,
      };
    });
    await page.screenshot({ path: path.join(out, `t${tier}-${kind}-done.png`), clip: { x: 0, y: 0, width: 400, height: 500 } });
    const want = kind === 'blessing' ? 'Blessing' : 'Curse applied';
    const ok = res.rolled === res.claimed && res.force === tier && res.dialog === want && res.expands === res.restores
      && res.appLeft === '0px' && res.appVisible === 'visible';
    console.log(`tier ${tier} ${kind}: ${ok ? 'ok' : 'FAIL'} in ${secs}s`, JSON.stringify(res));
    if (!ok) failures.push(`${tier}-${kind}`);
  };

  for (const tier of [0, 1, 2, 3, 4]) await roll(tier, 'curse', tier >= 3 ? [['reveal', 900]] : []);
  await roll(5, 'curse', [['shatter', 300], ['heartbeat', 1600], ['burst', 2600], ['title', 4400]]);
  await roll(5, 'blessing', [['title', 4600]]);
  await roll(2, 'blessing', []);

  // Closing with a curse active asks first; "Cure and close" cures, then hides.
  await roll(1, 'curse', []);
  await dismiss();
  await page.evaluate(() => GACHAOS.closeFlow());
  const closeTitle = await page.evaluate(() => document.querySelector('#dlglayer .dialog .ttl')?.textContent);
  await page.screenshot({ path: path.join(out, 'close-prompt.png'), clip: { x: 0, y: 0, width: 400, height: 500 } });
  await page.click('#dlglayer .btns .btn >> nth=0');
  await page.waitForTimeout(100);
  const closed = await page.evaluate(() => ({
    cured: __MOCK__.st.active === null,
    hid: __MOCK__.calls.slice(-3).some(c => c[0] === 'hide_to_tray'),
  }));
  const closeOk = closeTitle === 'Close gachaos?' && closed.cured && closed.hid;
  console.log(`close with curse: ${closeOk ? 'ok' : 'FAIL'}`, JSON.stringify({ closeTitle, ...closed }));
  if (!closeOk) failures.push('close');

  // The tray flyout: compact layout, anchored at the tray icon.
  await page.evaluate(() => GACHAOS.setMode('mini', [1270, 790]));
  await page.waitForTimeout(150);
  const mini = await page.evaluate(() => ({
    mini: document.body.classList.contains('mini'),
    width: document.getElementById('app').getBoundingClientRect().width,
    anchor: __MOCK__.calls.filter(c => c[0] === 'fit_window').pop()[1].anchor,
  }));
  await page.screenshot({ path: path.join(out, 'flyout.png'), clip: { x: 0, y: 0, width: 320, height: 420 } });
  const miniOk = mini.mini && mini.width === 300 && JSON.stringify(mini.anchor) === '[1270,790]';
  console.log(`flyout: ${miniOk ? 'ok' : 'FAIL'}`, JSON.stringify(mini));
  if (!miniOk) failures.push('flyout');
  await page.evaluate(() => GACHAOS.setMode('full'));

  // Out of tickets shows an error instead of rolling.
  await page.evaluate(() => { __MOCK__.st.tickets = 0; __MOCK__.st.freePulls = 0; document.getElementById('dlglayer').textContent = ''; });
  await page.click('#pull');
  const noTickets = await page.evaluate(() => document.querySelector('#dlglayer .msg')?.textContent || '');
  const ntOk = noTickets.includes('out of tickets') && !(await page.evaluate(() => GACHAOS.S.busy));
  console.log(`no tickets: ${ntOk ? 'ok' : 'FAIL'}`);
  if (!ntOk) failures.push('no-tickets');

  console.log('page errors:', errors.length ? errors : 'none');
  await browser.close();
  if (failures.length || errors.length) process.exit(1);
})();
