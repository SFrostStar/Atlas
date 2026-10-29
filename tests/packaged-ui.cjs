// Staged release only. This deliberately does not start a game or touch the real data folder.
const { _electron } = require('playwright-core');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { performance } = require('node:perf_hooks');

const root = path.resolve(__dirname, '..');
const fixture = path.join(root, 'work', 'package-fixture');
const executable = path.join(fixture, 'dist', 'Atlas Launcher-win32-x64', 'Atlas Launcher.exe');
const profile = path.join(fixture, `profile-${crypto.randomUUID()}`);

(async () => {
  assert.ok(fs.existsSync(executable), 'Package the staged release first.');
  const started = performance.now();
  const app = await _electron.launch({ executablePath: executable, args: [`--user-data-dir=${profile}`], cwd: fixture });
  try {
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
    await page.locator('[data-setting="theme"]').selectOption('light');
    await page.locator('[data-accent="violet"]').click();
    await page.locator('[data-setting="heroStyle"]').selectOption('sunset');
    await page.locator('#instance-memory').selectOption('2048');
    state = await page.evaluate(() => window.atlas.getState());
    assert.equal(state.settings.language, 'en');
    assert.equal(state.settings.theme, 'light');
    assert.equal(state.settings.accentColor, 'violet');
    assert.equal(state.instances[0].memoryMb, 2048);
    await page.reload();
    await page.waitForSelector('#boot-splash', { state: 'detached' });
    assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
    assert.equal(await page.locator('html').getAttribute('lang'), 'en');
    await page.locator('[data-page="settings"]').click();
    await page.screenshot({ path: path.join(fixture, 'settings-light-en.png') });
    await page.locator('[data-setting="language"]').selectOption('ru');
    await page.locator('[data-setting="theme"]').selectOption('dark');
    assert.equal(await page.locator('h1').textContent(), 'Настройки');
    assert.deepEqual(errors, []);
    assert.equal(fs.existsSync(path.join(fixture, 'data', 'launcher.json')), true);
    const { listPackage } = await import('@electron/asar');
    const packagedFiles = listPackage(path.join(path.dirname(executable), 'resources', 'app.asar')).map(file => file.replace(/\\/g, '/'));
    assert.ok(packagedFiles.some(file => file.startsWith('/node_modules/lucide/')));
    assert.ok(!packagedFiles.some(file => file.startsWith('/node_modules/playwright-core/')));
    assert.ok(!packagedFiles.some(file => file.startsWith('/node_modules/electron/')));
    console.log(JSON.stringify({ passed: true, executable, fixture, readyMs, checks: 'packaged isolation, context isolation, RU/EN, light/dark, accent, background, RAM and persistence, production dependency pruning' }));
  } finally { await app.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
