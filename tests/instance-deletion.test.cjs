const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const crypto = require('node:crypto');
const { LauncherStore } = require('../lib/launcher-store.cjs');
const { GameSessions } = require('../lib/game-sessions.cjs');

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-deletion-test-'));
  t.after(() => {
    if (!path.resolve(root).startsWith(`${path.resolve(os.tmpdir())}${path.sep}atlas-deletion-test-`)) throw new Error('Unsafe fixture cleanup');
    fs.rmSync(root, { recursive: true, force: true });
  });
  const store = new LauncherStore(root, { appData: path.join(root, 'appdata') });
  return { root, store };
}

function addSession(store, instanceId) {
  const directory = path.join(store.dataRoot, 'sessions', crypto.randomUUID());
  const home = path.join(directory, 'home');
  fs.mkdirSync(path.join(home, 'saves'), { recursive: true });
  fs.writeFileSync(path.join(directory, 'session.json'), JSON.stringify({ instanceId, home }));
  fs.writeFileSync(path.join(home, 'saves', 'world.txt'), 'test world');
  return directory;
}

test('deleting a selected instance trashes its worlds and related windows together, preserving other instances', async t => {
  const { root, store } = fixture(t);
  const instance = store.createInstance({ name: 'Delete me' });
  fs.writeFileSync(path.join(store.instancePath(instance.id), 'saves', 'base-world.txt'), 'base world');
  const related = addSession(store, instance.id);
  const other = addSession(store, 'atlas');
  const originalConfig = JSON.parse(fs.readFileSync(store.file, 'utf8'));
  let trashCalls = 0;
  await store.deleteInstance(instance.id, { trashItem: async directory => {
    trashCalls++;
    assert.ok(path.relative(store.dataRoot, directory).startsWith(`.delete-staging${path.sep}`));
    assert.equal(fs.readFileSync(path.join(directory, 'instance', 'saves', 'base-world.txt'), 'utf8'), 'base world');
    assert.equal(fs.existsSync(path.join(directory, `window-${path.basename(related)}`, 'home', 'saves', 'world.txt')), true);
    assert.deepEqual(JSON.parse(fs.readFileSync(store.file, 'utf8')), originalConfig, 'configuration is kept until trash succeeds');
    fs.renameSync(directory, path.join(root, 'recycle-bin'));
  }});
  assert.equal(trashCalls, 1);
  assert.equal(store.config.instances.length, 1);
  assert.equal(store.config.selectedId, 'atlas');
  assert.equal(new LauncherStore(root).config.selectedId, 'atlas');
  assert.ok(fs.existsSync(other));
  assert.ok(!fs.existsSync(related));
  assert.equal(fs.readFileSync(path.join(root, 'recycle-bin', 'instance', 'saves', 'base-world.txt'), 'utf8'), 'base world');
});

test('trash failure restores all folders and leaves selected instance and config unchanged', async t => {
  const { store } = fixture(t);
  const instance = store.createInstance({ name: 'Keep me' });
  const related = addSession(store, instance.id);
  const home = store.instancePath(instance.id);
  fs.writeFileSync(path.join(home, 'saves', 'base-world.txt'), 'base world');
  const before = fs.readFileSync(store.file, 'utf8');
  await assert.rejects(store.deleteInstance(instance.id, { trashItem: async () => { throw new Error('Recycle Bin unavailable'); } }), /Recycle Bin unavailable/);
  assert.equal(fs.readFileSync(store.file, 'utf8'), before);
  assert.equal(store.config.selectedId, instance.id);
  assert.equal(fs.readFileSync(path.join(home, 'saves', 'base-world.txt'), 'utf8'), 'base world');
  assert.equal(fs.readFileSync(path.join(related, 'home', 'saves', 'world.txt'), 'utf8'), 'test world');
  assert.equal(store.deletingInstances.size, 0);
});

test('last, active and copying instances cannot be deleted; launching cannot race an accepted deletion', async t => {
  const { root, store } = fixture(t);
  let calls = 0;
  const trashItem = async () => { calls++; };
  await assert.rejects(store.deleteInstance('atlas', { trashItem }), /Последнюю/);
  const instance = store.createInstance({ name: 'Second' });
  await assert.rejects(store.deleteInstance(instance.id, { trashItem, isRunning: () => true }), /Закрой все окна/);
  const games = new GameSessions(store);
  const launch = games.launchAnother(instance.id);
  assert.equal(games.isInstanceRunning(instance.id), true);
  await assert.rejects(store.deleteInstance(instance.id, { trashItem, isRunning: id => games.isInstanceRunning(id) }), /Закрой все окна/);
  await assert.rejects(launch, /Клиент не найден/);
  assert.equal(games.isInstanceRunning(instance.id), false);
  assert.equal(calls, 0);
  let release;
  const deleting = store.deleteInstance(instance.id, { trashItem: directory => new Promise(resolve => { release = () => { fs.renameSync(directory, path.join(root, 'recycle-bin')); resolve(); }; }) });
  await assert.rejects(games.launch(instance.id), /Сборка удаляется/);
  await assert.rejects(store.deleteInstance('atlas', { trashItem }), /Дождись удаления/);
  release();
  await deleting;
});

test('deletion rejects junctions and inconsistent session metadata instead of following outside data', async t => {
  const { root, store } = fixture(t);
  const instance = store.createInstance({ name: 'Unsafe' });
  const home = store.instancePath(instance.id);
  const outside = path.join(root, 'outside-data');
  fs.mkdirSync(outside);
  fs.writeFileSync(path.join(outside, 'keep.txt'), 'outside');
  fs.renameSync(home, path.join(root, 'original-instance'));
  fs.symlinkSync(outside, home, process.platform === 'win32' ? 'junction' : 'dir');
  let calls = 0;
  await assert.rejects(store.deleteInstance(instance.id, { trashItem: async () => { calls++; } }), /папок-ссылок/);
  assert.equal(fs.readFileSync(path.join(outside, 'keep.txt'), 'utf8'), 'outside');
  assert.equal(calls, 0);
  fs.unlinkSync(home);
  fs.renameSync(path.join(root, 'original-instance'), home);
  const session = addSession(store, instance.id);
  fs.writeFileSync(path.join(session, 'session.json'), JSON.stringify({ instanceId: instance.id, home: outside }));
  await assert.rejects(store.deleteInstance(instance.id, { trashItem: async () => { calls++; } }), /не соответствует/);
  assert.equal(calls, 0);
  assert.equal(store.config.instances.length, 2);
});

test('Discord settings validate public IDs and keep missing opt-in unspecified; launcher version is independent', t => {
  const { store } = fixture(t);
  const initial = store.getState();
  assert.equal(initial.version, '0.1');
  assert.equal(initial.engineVersion, 'v0.2.93');
  assert.equal(Object.hasOwn(initial.settings, 'discordRichPresence'), false);
  assert.equal(initial.settings.discordApplicationId, '');
  assert.equal(initial.settings.discordAssetKey, 'atlas');
  store.saveSettings({ discordRichPresence: true, discordApplicationId: '123456789012345678', discordAssetKey: 'atlas_logo' });
  assert.equal(store.getState().settings.discordRichPresence, true);
  assert.equal(new LauncherStore(store.projectRoot).getState().settings.discordApplicationId, '123456789012345678');
  assert.throws(() => store.saveSettings({ discordApplicationId: 'secret-token' }), /Application ID/);
  assert.throws(() => store.saveSettings({ discordAssetKey: 'path/logo.png' }), /изображения/);
  assert.throws(() => store.saveSettings({ discordRichPresence: 'true' }), /Presence/);
});
