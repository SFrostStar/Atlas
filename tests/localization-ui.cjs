// Run separately from other ATLAS_GUI_TEST windows (Electron's instance lock).
const launchGuiTest = require('./launch-gui-test.cjs');
const assert = require('node:assert/strict');

(async () => {
  const { app, close } = await launchGuiTest();
  try {
    const page = await app.firstWindow();
    page.setDefaultTimeout(5000);
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.waitForSelector('#boot-splash', { state: 'detached' });
    await page.evaluate(() => invoke(() => window.atlas.saveSettings({ language: 'en', heroStyle: 'minimal', showHeroText: true })));
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
    for (const route of ['home', 'instances', 'mods', 'settings', 'logs']) {
      await page.locator(`.nav-item[data-page="${route}"]`).click();
      await checkLayout(route);
    }
    await page.locator('.nav-item[data-page="home"]').click();
    await page.locator('.nav-item[data-page="instances"]').click();
    assert.equal(await page.locator('[data-instance-tab]').count(), 0);
    assert.equal(await page.locator('.instance-details h2').textContent(), 'Atlas — instance files');
    assert.equal(await page.locator('.file-row').count(), 6);
    await checkLayout('files');
    await page.locator('.nav-item[data-page="home"]').click();
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
    await page.locator('.nav-item[data-page="instances"]').click();
    const tile = page.locator(`[data-instance="${instance.id}"]`);
    assert.equal(await tile.locator('strong').textContent(), 'Настройки');
    await tile.click({ button: 'right' });
    assert.equal(await page.locator('#context-menu').getAttribute('aria-label'), 'Instance actions');
    await page.locator('#context-menu [data-action="delete-instance"]').click();
    assert.equal(await page.locator('#delete-name').textContent(), 'Настройки');
    await checkLayout('delete dialog');
    await page.locator('#delete-dialog [data-action="close-delete"]').first().click();
    await page.locator('.instance-tile-new').click();
    await page.locator('#instance-name').fill('Настройки');
    await page.locator('#instance-form [type="submit"]').click();
    await page.waitForFunction(() => document.getElementById('form-error').textContent.includes('already exists'));
    await page.locator('#instance-dialog [data-action="close-dialog"]').first().click();
    await tile.click();
    await page.locator('.nav-item[data-page="home"]').click();
    // Exercise menus with a synthetic snapshot, without starting a real game.
    await page.evaluate(id => {
      globalThis.savedUiState = state;
      state = { ...state, game: { running: true, count: 2, sessions: [
        { id: 'test-main', instanceId: id, name: 'Настройки', isolated: false, status: 'running' },
        { id: 'test-extra', instanceId: id, name: 'Настройки - 1', isolated: true, status: 'running' },
      ] } };
      render();
    }, instance.id);
    await page.locator('[data-action="game-menu"]').click();
    assert.equal(await page.locator('#context-menu').getAttribute('aria-label'), 'Game controls');
    assert.equal(await page.locator('.context-session').last().textContent(), 'Настройки - 1');
    assert.equal(await page.locator('#context-menu [data-action="stop-game"]').innerText(), 'Stop');
    await page.locator('#context-menu [data-action="another-game"]').click();
    assert.equal(await page.locator('#game-window-title').textContent(), 'Another window');
    assert.equal(await page.locator('#game-window-form input').getAttribute('placeholder'), 'Automatic number');
    await page.keyboard.press('Escape');
    await page.locator('#game-window-dialog').waitFor({state:'hidden'});
    await page.evaluate(() => { state = globalThis.savedUiState; render(); });
    await page.locator('[data-page="settings"]').click();
    await page.locator('[data-setting="heroStyle"]').focus();
    assert.notEqual(await page.locator('[data-setting="heroStyle"]').evaluate(el => getComputedStyle(el).outlineStyle), 'none');
    const readable = await page.evaluate(() => {
      const luminance = (text, backdrop) => {
        const context = document.createElement('canvas').getContext('2d');
        if (backdrop) {
          context.fillStyle = backdrop;
          context.fillRect(0,0,1,1);
        }
        context.fillStyle = text;
        context.fillRect(0,0,1,1);
        const parts = [...context.getImageData(0,0,1,1).data].slice(0,3).map(n => n / 255).map(n => n <= .04045 ? n / 12.92 : ((n + .055) / 1.055) ** 2.4);
        return .2126 * parts[0] + .7152 * parts[1] + .0722 * parts[2];
      };
      const panel = document.querySelector('.discord-preview').closest('.panel');
      const background = luminance(getComputedStyle(panel).backgroundColor,'#fff');
      return ['.discord-preview span', '.discord-preview small'].map(selector => {
        const color = luminance(getComputedStyle(document.querySelector(selector)).color);
        return (color + .05) / (background + .05);
      });
    });
    assert.ok(readable.every(ratio => ratio >= 4.5), `Text contrast ${readable}`);
    await page.setViewportSize({ width: 1050, height: 740 });
    await checkLayout('small settings window');
    await page.reload();
    await page.waitForSelector('#boot-splash', { state: 'detached' });
    assert.equal(await page.locator('html').getAttribute('lang'), 'en');
    await page.locator('[data-page="settings"]').click();
    await page.locator('[data-setting="language"]').selectOption('ru');
    assert.equal(await page.locator('[data-page="instances"]').innerText().then(s => s.replace(/\d+/g, '').trim()), 'Мои сборки');
    await page.locator('[data-page="instances"]').click();
    assert.equal(await tile.locator('strong').textContent(), 'Настройки');
    assert.deepEqual(errors, []);
    console.log('PASS: all launcher pages, files, dialogs, dynamic context menus, custom RU names, form errors, accents, focus, contrast, layout and RU↔EN.');
  } finally { await close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
