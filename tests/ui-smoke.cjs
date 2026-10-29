// Run against the development window started with --remote-debugging-port=9227.
const { chromium } = require('playwright-core');
const assert = require('node:assert/strict');
const path = require('node:path');
const fs = require('node:fs');
(async () => {
  const browser = await chromium.connectOverCDP('http://127.0.0.1:9227');
  try {
    const page = browser.contexts()[0].pages()[0];
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.reload();
    await page.waitForSelector('.hero');
    assert.equal(await page.title(), 'Atlas Launcher');
    assert.ok(await page.locator('svg.lucide').count() > 15);
    assert.equal(await page.evaluate(() => typeof window.require), 'undefined');
    assert.equal(await page.evaluate(() => typeof window.process), 'undefined');
    assert.ok(await page.evaluate(() => document.querySelector('main').getBoundingClientRect().bottom <= innerHeight));
    const initial = await page.evaluate(() => window.atlas.getState());
    if (!initial.instances.some(item => item.name === 'Песочница')) {
      await page.locator('[data-page="instances"]').first().click();
      await page.locator('[data-action="new-instance"]').filter({hasText:'Новая сборка'}).click();
      await page.locator('#instance-name').fill('Песочница');
      await page.locator('input[name="description"]').fill('Место для экспериментов с Atlas');
      await page.locator('#instance-form [type="submit"]').click();
      await page.waitForSelector('#instance-dialog:not([open])', {state:'attached'});
    }
    let state = await page.evaluate(() => window.atlas.getState());
    const sandbox = state.instances.find(item => item.name === 'Песочница');
    assert.ok(sandbox && fs.existsSync(path.join(sandbox.path, 'saves')));
    await page.locator(`.sidebar-instance[data-id="${sandbox.id}"]`).click();
    assert.equal(await page.locator('[data-tab="game"]').count(), 0);
    await page.reload();
    await page.waitForSelector('.hero');
    state = await page.evaluate(() => window.atlas.getState());
    assert.equal(state.selectedId, sandbox.id);
    for (const route of ['workshop','settings','logs','instances','home']) {
      await page.locator(`.nav-item[data-page="${route}"]`).click();
      assert.ok((await page.locator('h1').textContent()).length);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth || document.querySelector('main').getBoundingClientRect().bottom > innerHeight + 1);
      assert.equal(overflow, false, `Overflow on ${route}`);
    }
    await page.locator('.sidebar-instance[data-id="atlas"]').click();
    await page.locator('[data-tab="files"]').click();
    assert.equal(await page.locator('.file-row').count(), 4);
    await page.locator('[data-tab="overview"]').click();
    assert.deepEqual(errors, []);
    console.log('UI PASS: navigation, instance persistence, game settings kept inside the game, files, icons, layout, renderer isolation.');
  } finally { await browser.close(); }
})().catch(error => {console.error(error);process.exitCode=1;});
