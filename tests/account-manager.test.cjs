const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { AccountManager, protectSessionDirectory } = require('../lib/account-manager.cjs');

const CLIENT = '12345678-1234-1234-1234-123456789abc';
const UUID = '00112233445566778899aabbccddeeff';
const secureStorage = {
  isEncryptionAvailable: () => true,
  encryptString: text => Buffer.from([...Buffer.from(text)].map(value => value ^ 123)),
  decryptString: buffer => Buffer.from([...buffer].map(value => value ^ 123)).toString(),
};
function fixture(t, options = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-accounts-test-'));
  const manager = new AccountManager({ dataRoot: root, clientId: CLIENT, safeStorage: secureStorage, ...options });
  t.after(() => {
    manager.dispose();
    if (path.dirname(root) !== os.tmpdir() || !path.basename(root).startsWith('atlas-accounts-test-')) throw new Error('Unsafe cleanup');
    fs.rmSync(root, { recursive: true, force: true });
  });
  return manager;
}
const reply = (value, status = 200) => new Response(JSON.stringify(value), { status, headers: { 'Content-Type': 'application/json' } });
function service({ pending = [], noOwnership = false, approval = false } = {}) {
  const requests = [];
  let refresh = false;
  const fetch = async (url, options) => {
    requests.push({ url, options });
    if (url.endsWith('/devicecode')) return reply({ device_code: 'private-device-secret', user_code: 'ABCD-EFGH', expires_in: 900, interval: 1, verification_uri: 'https://www.microsoft.com/link' });
    if (url.endsWith('/token')) {
      const form = new URLSearchParams(options.body);
      if (form.get('grant_type') === 'refresh_token') refresh = true;
      if (!refresh && pending.length) return reply({ error: pending.shift() }, 400);
      return reply({ access_token: 'microsoft-access-secret', refresh_token: refresh ? 'rotated-refresh-secret' : 'microsoft-refresh-secret' });
    }
    if (url.endsWith('/user/authenticate')) return reply({ Token: 'xbl-secret' });
    if (url.endsWith('/xsts/authorize')) return reply({ Token: 'xsts-secret', DisplayClaims: { xui: [{ uhs: 'hash', xid: '12345' }] } });
    if (url.endsWith('/authentication/login_with_xbox')) return approval ? reply({ error: 'private-service-details' }, 403) : reply({ access_token: 'minecraft-access-secret', expires_in: 3600 });
    if (url.endsWith('/entitlements/mcstore')) return reply({ items: noOwnership ? [] : [{ name: 'game_minecraft' }] });
    if (url.endsWith('/minecraft/profile')) return reply({ id: UUID, name: 'AtlasTester' });
    throw new Error(`Unexpected endpoint ${url}`);
  };
  return { fetch, requests };
}
async function signIn(manager) {
  await manager.startSignIn();
  if (manager.flow?.task) await manager.flow.task;
  return manager.snapshot();
}

test('local accounts persist, select and launch without network or secure storage', async t => {
  let calls = 0;
  const manager = fixture(t, { safeStorage: null, fetch: async () => { calls++; throw new Error(); } });
  manager.addLocal('Atlas_Name');
  await assert.rejects(async () => manager.addLocal('../unsafe'), error => error.code === 'invalid_name');
  assert.deepEqual((await manager.prepareLaunch(manager.root)).args, ['--name', 'Atlas_Name']);
  const reloaded = new AccountManager({ dataRoot: manager.root });
  assert.equal(reloaded.snapshot().accounts.find(a => a.id === reloaded.snapshot().selectedId).name, 'Atlas_Name');
  manager.remove(manager.snapshot().selectedId);
  assert.equal(calls, 0);
});

test('sign-in requires own configured client and secure storage, with no plaintext fallback', async t => {
  const manager = fixture(t, { clientId: '' });
  await assert.rejects(manager.startSignIn(), error => error.code === 'not_configured');
  manager.clientId = CLIENT;
  manager.safeStorage = { isEncryptionAvailable: () => false };
  await assert.rejects(manager.startSignIn(), error => error.code === 'storage_unavailable');
  assert.equal(manager.snapshot().accounts.length, 1);
});

test('device flow respects pending and slow_down, stores only encrypted tokens and exposes no secrets', async t => {
  const api = service({ pending: ['authorization_pending', 'slow_down'] });
  const intervals = [];
  const opened = [];
  const snapshots = [];
  const manager = fixture(t, { fetch: api.fetch, sleep: async ms => { intervals.push(ms); }, openExternal: async url => opened.push(url), onChange: state => snapshots.push(state) });
  const state = await signIn(manager);
  assert.equal(state.status, 'idle');
  assert.equal(state.accounts.at(-1).name, 'AtlasTester');
  assert.deepEqual(intervals, [1000, 1000, 6000]);
  assert.deepEqual(opened, ['https://www.microsoft.com/link']);
  const stored = fs.readFileSync(manager.file, 'utf8');
  const visible = JSON.stringify(snapshots);
  for (const secret of ['private-device-secret', 'microsoft-access-secret', 'microsoft-refresh-secret', 'minecraft-access-secret', 'xsts-secret']) {
    assert.equal(stored.includes(secret), false, `${secret} leaked to file`);
    assert.equal(visible.includes(secret), false, `${secret} leaked to renderer`);
  }
  const tokenRequest = api.requests.find(request => request.url.endsWith('/token'));
  assert.equal(new URLSearchParams(tokenRequest.options.body).get('client_id'), CLIENT);
  assert.equal(api.requests.find(r => r.url.endsWith('/user/authenticate')).options.body.includes('d=microsoft-access-secret'), true);
});

test('a game session is private, has Rust-compatible schema, and cleanup never removes its parent', async t => {
  const api = service();
  const protectedFolders = [];
  const manager = fixture(t, { fetch: api.fetch, sleep: async () => {}, protectDirectory: async folder => protectedFolders.push(folder) });
  await signIn(manager);
  const launch = await manager.prepareLaunch(manager.root);
  assert.equal(launch.args[0], '--session-file');
  const file = launch.args[1];
  assert.deepEqual(protectedFolders, [path.dirname(file)]);
  const body = JSON.parse(fs.readFileSync(file, 'utf8'));
  assert.deepEqual(body, { name: 'AtlasTester', uuid: UUID, access_token: 'minecraft-access-secret', user_type: 'msa', xuid: '12345' });
  assert.equal(launch.args.some(arg => arg.includes('minecraft-access-secret')), false);
  await launch.cleanup();
  await launch.cleanup();
  assert.equal(fs.existsSync(file), false);
  assert.equal(fs.existsSync(path.join(manager.root, 'auth-sessions')), true);
});

test('parallel launches coalesce token refresh and use separate one-time files', async t => {
  const api = service();
  let time = 100000;
  const manager = fixture(t, { fetch: api.fetch, now: () => time, sleep: async () => {}, protectDirectory: async () => {} });
  await signIn(manager);
  time += 3600000;
  const launches = await Promise.all([manager.prepareLaunch(manager.root), manager.prepareLaunch(manager.root)]);
  const refreshes = api.requests.filter(r => r.url.endsWith('/token') && new URLSearchParams(r.options.body).get('grant_type') === 'refresh_token');
  assert.equal(refreshes.length, 1);
  assert.notEqual(launches[0].args[1], launches[1].args[1]);
  const account = manager.data.accounts.find(a => a.type === 'microsoft');
  assert.equal(manager.decrypt(account).refreshToken, 'rotated-refresh-secret');
  await Promise.all(launches.map(launch => launch.cleanup()));
});

test('cancelled device flow cannot save a late response or select a Microsoft account', async t => {
  const api = service();
  let wake;
  const manager = fixture(t, { fetch: api.fetch, sleep: () => new Promise(resolve => { wake = resolve; }) });
  await manager.startSignIn();
  const pending = manager.flow.task;
  manager.cancelSignIn(); wake(); await pending;
  assert.equal(manager.snapshot().status, 'idle');
  assert.equal(manager.snapshot().device, null);
  assert.equal(manager.snapshot().accounts.length, 1);
  assert.equal(api.requests.filter(r => r.url.endsWith('/token')).length, 0);
});

test('expired and declined codes end polling and expose sanitized errors', async t => {
  let time = 0;
  const expired = fixture(t, { fetch: service().fetch, now: () => time, sleep: async () => { time += 1000000; } });
  assert.equal((await signIn(expired)).error.code, 'expired_token');
  const declined = fixture(t, { fetch: service({ pending: ['authorization_declined'] }).fetch, sleep: async () => {} });
  assert.equal((await signIn(declined)).error.code, 'authorization_declined');
});

test('refused app registration and missing ownership do not create Microsoft accounts', async t => {
  const unapproved = fixture(t, { fetch: service({ approval: true }).fetch, sleep: async () => {} });
  assert.equal((await signIn(unapproved)).error.code, 'app_not_approved');
  assert.equal(unapproved.snapshot().accounts.length, 1);
  const unowned = fixture(t, { fetch: service({ noOwnership: true }).fetch, sleep: async () => {} });
  assert.equal((await signIn(unowned)).error.code, 'no_ownership');
  assert.equal(unowned.snapshot().accounts.length, 1);
});

test('unknown network errors and unsafe verification links cannot leak data to the UI', async t => {
  const offline = fixture(t, { fetch: async () => { throw new Error('https://secret.invalid/?token=SECRET'); } });
  await assert.rejects(offline.startSignIn(), error => error.code === 'network' && !error.message.includes('SECRET'));
  const wrongLink = fixture(t, { fetch: async () => reply({ device_code: 'secret', user_code: 'CODE', expires_in: 60, verification_uri: 'https://phishing.invalid/' }) });
  await assert.rejects(wrongLink.startSignIn(), error => error.code === 'invalid_response');
});

test('a locked vault or failed private-directory setup prevents online launch', async t => {
  const manager = fixture(t, { fetch: service().fetch, sleep: async () => {}, protectDirectory: async () => { throw new Error('ACL failed'); } });
  await signIn(manager);
  await assert.rejects(manager.prepareLaunch(manager.root), error => error.code === 'session_security');
  assert.deepEqual(fs.readdirSync(path.join(manager.root, 'auth-sessions')), []);
  manager.safeStorage = { isEncryptionAvailable: () => true, decryptString: () => { throw new Error('raw private details'); } };
  await assert.rejects(manager.prepareLaunch(manager.root), error => error.code === 'storage_locked');
});

test('the actual platform session directory protection works on a fresh test directory', async t => {
  const manager = fixture(t);
  const directory = fs.mkdtempSync(path.join(manager.root, 'private-'));
  await protectSessionDirectory(directory);
  fs.writeFileSync(path.join(directory, 'probe.txt'), 'no token');
  assert.equal(fs.readFileSync(path.join(directory, 'probe.txt'), 'utf8'), 'no token');
});

test('malformed account files remain untouched', t => {
  const manager = fixture(t);
  fs.writeFileSync(manager.file, '{broken');
  assert.throws(() => new AccountManager({ dataRoot: manager.root }), /preserved/);
  assert.equal(fs.readFileSync(manager.file, 'utf8'), '{broken');
});

test('abandoned expired one-time files are removed on restart while unknown files survive', t => {
  const manager = fixture(t);
  const sessions = path.join(manager.root, 'auth-sessions');
  fs.mkdirSync(sessions);
  const old = fs.mkdtempSync(path.join(sessions, 'session-'));
  const preserved = fs.mkdtempSync(path.join(sessions, 'session-'));
  fs.writeFileSync(path.join(old, 'session.json'), 'expired-token');
  fs.writeFileSync(path.join(preserved, 'unrelated.txt'), 'keep');
  const past = new Date(Date.now() - 180000);
  fs.utimesSync(old, past, past); fs.utimesSync(preserved, past, past);
  new AccountManager({ dataRoot: manager.root });
  assert.equal(fs.existsSync(old), false);
  assert.equal(fs.readFileSync(path.join(preserved, 'unrelated.txt'), 'utf8'), 'keep');
});

test('timeout of an account service is sanitized and stops sign-in', async t => {
  const manager = fixture(t, { requestTimeoutMs: 10, fetch: async (_url, options) => new Promise((_resolve, reject) => options.signal.addEventListener('abort', () => reject(new Error('secret upstream token')), { once: true })) });
  // AbortSignal.timeout alone intentionally does not keep Node alive.
  const keepAlive = setTimeout(() => {}, 1000);
  try { await assert.rejects(manager.startSignIn(), error => error.code === 'network' && !error.message.includes('secret')); }
  finally { clearTimeout(keepAlive); }
  assert.equal(manager.snapshot().status, 'error');
});
