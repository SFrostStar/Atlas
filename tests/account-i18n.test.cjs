const { test } = require('node:test');
const assert = require('node:assert/strict');
const i18n = require('../src/i18n.js');

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
