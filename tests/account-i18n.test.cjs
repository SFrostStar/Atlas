const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const i18n = require('../i18n.js');

test('account error codes and original English IPC messages localize in either language', () => {
  const english = 'Your Microsoft sign-in expired. Sign in again.';
  const russian = 'Сессия Microsoft истекла. Войди заново.';
  assert.equal(i18n.accountError({ code: 'invalid_grant', message: english }, 'ru'), russian);
  assert.equal(i18n.accountError({ code: 'invalid_grant', message: english }, 'en'), english);
  assert.equal(i18n.t(english, 'ru'), russian);
  assert.equal(i18n.t(russian, 'en'), english);
  assert.equal(i18n.accountError({ message: 'Unknown issue' }, 'ru'), 'Unknown issue');
  assert.equal(i18n.t('AtlasCustomName', 'en'), 'AtlasCustomName');
});

test('a pending account flow cannot be started again and can be cancelled during verification', () => {
  const context = vm.createContext({
    AtlasI18n: i18n,
    state: { settings: { language: 'en' }, accounts: { accounts: [], selectedId: null, configured: true, storageAvailable: true, status: 'verifying', device: null, error: { code: 'invalid_grant' } } },
    document: { addEventListener() {} },
    icon: () => '', escapeHtml: text => String(text),
    button: (action, label, _icon, _style, attributes = '') => `<button data-action="${action}" ${attributes}>${label}</button>`,
  });
  vm.runInContext(fs.readFileSync(path.join(__dirname, '..', 'app-settings.js'), 'utf8'), context);
  const panel = vm.runInContext('accountSettingsPanel()', context);
  assert.match(panel, /data-action="signin-microsoft" disabled/);
  assert.match(panel, /data-action="cancel-signin"/);
  assert.match(panel, /Your Microsoft sign-in expired/);
  context.state.accounts.status = 'idle';
  const idle = vm.runInContext('accountSettingsPanel()', context);
  assert.doesNotMatch(idle, /data-action="signin-microsoft" disabled/);
});
