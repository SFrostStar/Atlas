// Run separately from other ATLAS_GUI_TEST windows (Electron's instance lock).
const { _electron } = require('playwright-core');
const assert = require('node:assert/strict');
const path = require('node:path');
const root = path.resolve(__dirname, '..');

(async () => {
  const app = await _electron.launch({ executablePath: path.join(root, 'node_modules/electron/dist/electron.exe'), args: [root], cwd: root, env: { ...process.env, ATLAS_GUI_TEST: '1' } });
  try {
    const page = await app.firstWindow();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.waitForSelector('#boot-splash', { state: 'detached' });
    await page.evaluate(() => invoke(() => window.atlas.saveSettings({ language: 'en', theme: 'light', heroStyle: 'minimal', showHeroText: true })));
    const untranslated = () => page.evaluate(() => {
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT), found = [];
      let node;
      while ((node = walker.nextNode())) {
        if (node.parentElement.closest('script,style,.path,[data-user-text],.sidebar-instance > span:last-child,.launch-info strong,.instance-body h3,.instance-body p,.section-heading h3,.context-heading,.context-session span,#log-console,dialog:not([open])')) continue;
        if (node.textContent.trim() !== 'Русский' && /[А-Яа-яЁё]/.test(node.textContent)) found.push(node.textContent.trim());
      }
      return found;
    });
    const checkLayout = async label => {
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth || document.querySelector('main').getBoundingClientRect().bottom > innerHeight + 1), false, `Overflow on ${label}`);
      assert.deepEqual(await untranslated(), [], `Untranslated labels on ${label}`);
    };
    for (const route of ['home', 'instances', 'workshop', 'settings', 'logs']) {
      await page.locator(`[data-page="${route}"]`).click();
      await checkLayout(route);
    }
    await page.locator('[data-page="home"]').click();
    await page.locator('[data-tab="files"]').click();
    assert.equal(await page.locator('.file-row').count(), 4);
    await checkLayout('files');
    await page.locator('[data-tab="overview"]').click();
    let snapshot = await page.evaluate(() => window.atlas.getState());
    let instance = snapshot.instances.find(item => item.name === 'Настройки');
    if (!instance) {
      await page.locator('.add-sidebar').click();
      await page.locator('#instance-name').fill('Настройки');
      await page.locator('#instance-form [name="description"]').fill('Твои правила.');
      await page.locator('#instance-form [value="violet"]').check();
      await checkLayout('new instance dialog');
      await page.locator('#instance-form [type="submit"]').click();
      await page.waitForSelector('#instance-dialog:not([open])', { state: 'attached' });
      snapshot = await page.evaluate(() => window.atlas.getState());
      instance = snapshot.instances.find(item => item.name === 'Настройки');
    }
    assert.equal(instance.name, 'Настройки');
    assert.equal(instance.description, 'Твои правила.');
    assert.equal(instance.accent, 'violet');
    await page.locator(`[data-page="instances"]`).click();
    assert.equal(await page.locator(`[data-instance="${instance.id}"] h3`).textContent(), 'Настройки');
    assert.equal(await page.locator(`[data-instance="${instance.id}"] p`).textContent(), 'Твои правила.');
    await page.locator(`[data-instance="${instance.id}"] [data-action="instance-menu"]`).click();
    assert.equal(await page.locator('#context-menu').getAttribute('aria-label'), 'Instance actions');
    await page.locator('#context-menu [data-action="delete-instance"]').click();
    assert.equal(await page.locator('#delete-name').textContent(), 'Настройки');
    await checkLayout('delete dialog');
    await page.locator('#delete-dialog [data-action="close-delete"]').first().click();
    await page.locator('.add-sidebar').click();
    await page.locator('#instance-name').fill('Настройки');
    await page.locator('#instance-form [type="submit"]').click();
    await page.waitForFunction(() => document.getElementById('form-error').textContent.includes('already exists'));
    await page.locator('#instance-dialog [data-action="close-dialog"]').first().click();
    await page.locator(`[data-id="${instance.id}"].sidebar-instance`).click();
    // Exercise menus with a synthetic snapshot, without starting a real game.
    await page.evaluate(id => {
      globalThis.savedUiState = state;
      state = { ...state, game: { running: true, count: 2, sessions: [
        { id: 'test-main', instanceId: id, name: 'Настройки', isolated: false, status: 'running' },
        { id: 'test-extra', instanceId: id, name: 'Настройки · ещё окно', isolated: true, status: 'running' },
      ] } };
      render();
    }, instance.id);
    await page.locator('[data-action="game-menu"]').click();
    assert.equal(await page.locator('#context-menu').getAttribute('aria-label'), 'Game controls');
    assert.ok((await page.locator('.context-session').last().textContent()).includes('another window'));
    assert.equal(await page.locator('#context-menu [data-action="stop-game"]').innerText(), 'Stop');
    await page.keyboard.press('Escape');
    await page.evaluate(() => { state = globalThis.savedUiState; render(); });
    await page.locator('[data-page="settings"]').click();
    await page.locator('[data-setting="theme"]').focus();
    assert.notEqual(await page.locator('[data-setting="theme"]').evaluate(el => getComputedStyle(el).outlineStyle), 'none');
    const readable = await page.evaluate(() => {
      const luminance = text => {
        const context = document.createElement('canvas').getContext('2d');
        context.fillStyle = text;
        context.fillRect(0,0,1,1);
        const parts = [...context.getImageData(0,0,1,1).data].slice(0,3).map(n => n / 255).map(n => n <= .04045 ? n / 12.92 : ((n + .055) / 1.055) ** 2.4);
        return .2126 * parts[0] + .7152 * parts[1] + .0722 * parts[2];
      };
      return ['.discord-preview span', '.discord-preview small', '.account-row .quiet'].map(selector => {
        const color = luminance(getComputedStyle(document.querySelector(selector)).color);
        return (1.05) / (color + .05);
      });
    });
    assert.ok(readable.every(ratio => ratio >= 4.5), `Light text contrast ${readable}`);
    await page.setViewportSize({ width: 1050, height: 740 });
    await checkLayout('small settings window');
    await page.reload();
    await page.waitForSelector('#boot-splash', { state: 'detached' });
    assert.equal(await page.locator('html').getAttribute('lang'), 'en');
    await page.locator('[data-page="settings"]').click();
    await page.locator('[data-setting="language"]').selectOption('ru');
    await page.locator('[data-setting="theme"]').selectOption('dark');
    assert.equal(await page.locator('[data-page="instances"]').innerText().then(s => s.replace(/\d+/g, '').trim()), 'Мои сборки');
    await page.locator('[data-page="instances"]').click();
    assert.equal(await page.locator(`[data-instance="${instance.id}"] h3`).textContent(), 'Настройки');
    assert.deepEqual(errors, []);
    console.log('PASS: all launcher pages, files, dialogs, dynamic context menus, custom RU names unchanged, form errors, accents, focus, light contrast, small window, persistence and RU↔EN.');
  } finally { await app.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
