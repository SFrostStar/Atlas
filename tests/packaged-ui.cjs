// Staged release only. This deliberately does not start a game or touch the real data folder.
const { _electron } = require('playwright-core');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { performance } = require('node:perf_hooks');
const { createQaWorkspace } = require('../scripts/qa-workspace.cjs');

const root = path.resolve(__dirname, '..');
const appFolder = path.join('dist', 'Atlas Launcher-win32-x64');
const staged = path.resolve(process.argv[2] || path.join(root, 'work', 'package-fixture', appFolder));

(async () => {
  assert.ok(fs.existsSync(path.join(staged, 'Atlas Launcher.exe')), 'Package the staged release first.');
  const scratch = createQaWorkspace(root);
  const fixture = scratch.directory;
  const executable = path.join(fixture, appFolder, 'Atlas Launcher.exe');
  let app;
  try {
    fs.cpSync(staged, path.dirname(executable), { recursive: true });
    const started = performance.now();
    app = await _electron.launch({
      executablePath: executable,
      args: [`--user-data-dir=${path.join(fixture, 'profile')}`],
      cwd: fixture,
      env: { ...process.env, ATLAS_NO_AUDIO: '1', ATLAS_NO_DISCORD: '1', ATLAS_LAUNCHER_NO_SELF_UPDATE: '1' },
    });
    scratch.trackChild(app.process().pid);
    const page = await app.firstWindow();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.waitForSelector('#boot-splash', { state: 'detached' });
    const readyMs = Math.round(performance.now() - started);
    let state = await page.evaluate(() => window.atlas.getState());
    assert.equal(state.projectRoot, fixture);
    assert.equal(state.dataRoot, path.join(fixture, 'data'));
    assert.equal(state.runtimePath, path.join(fixture, 'runtime', 'atlas-client.exe'));
    assert.equal(state.runtimeAvailable, false);
    assert.equal(state.sourceAvailable, false);
    assert.equal(await page.title(), 'Atlas Launcher');
    assert.equal(await page.evaluate(() => typeof window.require), 'undefined');
    assert.equal(await page.evaluate(() => typeof window.process), 'undefined');
    assert.ok(!state.accounts.configured);
    await page.locator('[data-page="settings"]').click();
    await page.locator('[data-setting="language"]').selectOption('en');
    assert.equal(await page.locator('h1').textContent(), 'Settings');
    assert.equal(await page.locator('[data-setting="theme"]').count(), 0);
    await page.locator('[data-accent="violet"]').click();
    await page.locator('[data-setting="heroStyle"]').selectOption('minimal');
    await page.locator('#instance-memory').focus();
    await page.locator('#instance-memory').press('Home');
    await page.locator('#instance-memory').press('ArrowRight');
    await page.waitForFunction(async()=>{const state=await window.atlas.getState();return state.instances.find(i=>i.id===state.selectedId).memoryMb===2048;});
    state = await page.evaluate(() => window.atlas.getState());
    assert.equal(state.settings.language, 'en');
    assert.equal(state.settings.theme, 'dark');
    assert.equal(state.settings.accentColor, 'violet');
    assert.equal(state.instances[0].memoryMb, 2048);
    await page.reload();
    await page.waitForSelector('#boot-splash', { state: 'detached' });
    assert.equal(await page.locator('html').getAttribute('data-hero'), 'minimal');
    assert.equal(await page.locator('html').getAttribute('lang'), 'en');
    await page.locator('[data-page="settings"]').click();
    await page.screenshot({ path: path.join(fixture, 'settings-dark-en.png') });
    await page.locator('[data-setting="language"]').selectOption('ru');
    assert.equal(await page.locator('h1').textContent(), 'Настройки');
    assert.deepEqual(errors, []);
    assert.equal(fs.existsSync(path.join(fixture, 'data', 'launcher.json')), true);
    const { listPackage } = await import('@electron/asar');
    const packagedFiles = listPackage(path.join(path.dirname(executable), 'resources', 'app.asar')).map(file => file.replace(/\\/g, '/'));
    assert.ok(packagedFiles.some(file => file.startsWith('/node_modules/lucide/')));
    assert.ok(!packagedFiles.some(file => file.startsWith('/node_modules/playwright-core/')));
    assert.ok(!packagedFiles.some(file => file.startsWith('/node_modules/electron/')));
    console.log(JSON.stringify({ passed: true, readyMs, checks: 'packaged isolation, context isolation, RU/EN, appearance, RAM and persistence, production dependency pruning' }));
  } finally {
    await app?.close();
    scratch.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
