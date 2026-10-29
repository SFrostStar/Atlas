const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { EventEmitter } = require('node:events');
const { PassThrough } = require('node:stream');
const { LauncherStore } = require('../lib/launcher-store.cjs');
const { GameSessions } = require('../lib/game-sessions.cjs');

function fixture(t, overrides = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-sessions-test-'));
  const store = new LauncherStore(root, { appData: path.join(root, 'appdata') });
  fs.mkdirSync(path.dirname(store.atlasRuntime), { recursive: true });
  fs.writeFileSync(store.atlasRuntime, 'test-runtime');
  fs.writeFileSync(path.join(path.dirname(store.atlasRuntime), 'atlas-ctl.exe'), 'test-helper');
  store.saveSettings({ runtimePath: store.atlasRuntime });
  const resources = path.join(path.dirname(store.atlasRuntime), '.local', 'atlas-client', '26.3');
  for (const name of ['assets', 'data']) fs.mkdirSync(path.join(resources, name), { recursive: true });
  const spawned = [];
  const calls = [];
  const logs = [];
  let pid = 5000;
  let manager;
  const spawn = (executable, args, options) => {
    if (overrides.spawnThrows) throw new Error('native spawn failed');
    const child = new EventEmitter();
    Object.assign(child, { pid: ++pid, stdout: new PassThrough(), stderr: new PassThrough(), exitCode: null });
    child.kill = () => { throw new Error('Must never force-kill an open game'); };
    spawned.push({ executable, args, options, child });
    if (overrides.spawnError) process.nextTick(() => child.emit('error', new Error('asynchronous native spawn failed')));
    else if (!overrides.noSpawn) process.nextTick(() => child.emit('spawn'));
    return child;
  };
  const execFile = async (executable, args, options) => {
    calls.push({ executable, args, options });
    const socket = args[args.indexOf('--socket') + 1];
    const record = [...manager.sessions.values()].find(session => session.socketPath === socket || String(session.child.pid) === args[1]);
    assert.ok(record, 'stop targets one managed session');
    if (overrides.controlFailure && args.includes('--socket')) throw new Error('control unavailable');
    if (!overrides.neverExit) setImmediate(() => { record.child.exitCode = 0; record.child.emit('close', 0); });
    return { stdout: '{"ok":true}\n', stderr: '' };
  };
  manager = new GameSessions(store, { spawn, execFile, prepareAccount: overrides.prepareAccount, onLog: (message, level) => logs.push({message,level}), stopTimeoutMs: 80, startTimeoutMs: 20 });
  t.after(async () => {
    for (const record of manager.sessions.values()) record.child.emit('close', 0);
    await new Promise(resolve => setTimeout(resolve, 20));
    const expectedPrefix = path.join(path.resolve(os.tmpdir()), 'atlas-sessions-test-');
    if (!path.resolve(root).startsWith(expectedPrefix)) throw new Error('Unsafe cleanup');
    fs.rmSync(root, { recursive: true, force: true });
  });
  return { root, store, manager, spawned, calls, logs };
}

test('additional windows get distinct data homes and sockets without copying saved worlds or secrets', async t => {
  const { store, manager, spawned } = fixture(t);
  const base = store.instancePath('atlas');
  fs.writeFileSync(path.join(base, 'options.txt'), 'lang:ru_ru\n');
  fs.writeFileSync(path.join(base, 'atlas-extras.txt'), 'noAi:true\ndynamicLights:true\n');
  fs.writeFileSync(path.join(base, 'accounts.json'), 'secret must not be copied');
  fs.mkdirSync(path.join(base, 'saves', 'My World'));
  fs.writeFileSync(path.join(base, 'resourcepacks', 'pack.zip'), 'pack-data');
  await manager.launch('atlas');
  await manager.launchAnother('atlas');
  const result = manager.snapshot();
  assert.equal(result.count, 2);
  assert.equal(new Set(result.sessions.map(item => item.socketPath)).size, 2);
  assert.equal(new Set(result.sessions.map(item => item.home)).size, 2);
  assert.equal(result.sessions[0].home, base);
  const extra = result.sessions[1];
  assert.equal(extra.isolated, true);
  assert.deepEqual(fs.readdirSync(path.join(extra.home, 'saves')), []);
  assert.equal(fs.existsSync(path.join(extra.home, 'accounts.json')), false);
  assert.equal(fs.readFileSync(path.join(extra.home, 'options.txt'), 'utf8'), 'lang:ru_ru\n');
  assert.match(fs.readFileSync(path.join(extra.home, 'atlas-extras.txt'), 'utf8'), /noAi:true/);
  assert.equal(fs.readFileSync(path.join(extra.home, 'resourcepacks', 'pack.zip'), 'utf8'), 'pack-data');
  for (const spec of spawned) {
    assert.equal(spec.options.env.HORIZON_HOME, spec.options.cwd);
    assert.equal(spec.options.env.ATLAS_HOME, spec.options.cwd);
    assert.ok(spec.args.includes('--control-socket'));
  }
  await assert.rejects(manager.launch('atlas'), /уже открыта/);
});

test('stop closes only the selected session and waits for its exit before restart', async t => {
  const { manager, spawned, calls } = fixture(t);
  await manager.launch('atlas');
  await manager.launchAnother('atlas');
  const [main, extra] = manager.snapshot().sessions;
  await manager.restart(extra.id);
  const sessions = manager.snapshot().sessions;
  assert.equal(spawned.length, 3);
  assert.equal(sessions.length, 2);
  assert.equal(sessions[0].id, main.id);
  assert.equal(sessions[1].home, extra.home);
  assert.notEqual(sessions[1].id, extra.id);
  assert.equal(sessions[1].isolated, true);
  assert.equal(calls[0].args[calls[0].args.indexOf('--socket') + 1], extra.socketPath);
  await manager.stop(main.id);
  assert.equal(manager.snapshot().count, 1);
  assert.equal(manager.snapshot().sessions[0].id, sessions[1].id);
  await assert.rejects(manager.stop(main.id), /уже закрыто/);
});

test('independent base instances can run together and implicit controls choose selected instance', async t => {
  const { manager, store } = fixture(t);
  const second = store.createInstance({ name: 'Second' });
  await manager.launch('atlas');
  await manager.launch(second.id);
  assert.equal(manager.isInstanceRunning('atlas'), true);
  assert.equal(manager.isInstanceRunning(second.id), true);
  await manager.stop();
  assert.equal(manager.isInstanceRunning(second.id), false);
  assert.equal(manager.isInstanceRunning('atlas'), true);
  assert.equal(manager.snapshot().count, 1);
});

test('No AI skips agent commands and normal Windows close never adds force flags', async t => {
  const { manager, store, calls } = fixture(t);
  fs.writeFileSync(path.join(store.instancePath('atlas'), 'atlas-extras.txt'), 'noAi:true\n');
  await manager.launch('atlas');
  if (process.platform === 'win32') {
    await manager.stop();
    assert.equal(calls.length, 1);
    assert.deepEqual(calls[0].args, ['/PID', '5001']);
    assert.match(calls[0].executable, /taskkill\.exe$/i);
    assert.ok(!calls[0].args.some(arg => arg.toLowerCase() === '/f'));
  } else {
    await assert.rejects(manager.stop(), /Не удалось завершить/);
    assert.equal(calls.length, 0);
  }
});

test('a close timeout keeps session tracked and prevents overlapping stop/restart', async t => {
  const { manager } = fixture(t, { neverExit: true });
  await manager.launch('atlas');
  const pending = manager.stop();
  await assert.rejects(manager.restart(), /Дождись/);
  await assert.rejects(pending, /сохраняется|не отвечает/);
  assert.equal(manager.snapshot().count, 1);
  assert.equal(manager.snapshot().sessions[0].status, 'running');
});

test('startup timeout keeps a potentially running game manageable instead of force killing it', async t => {
  const { manager } = fixture(t, { noSpawn: true });
  await assert.rejects(manager.launch('atlas'), /не подтвердил запуск/);
  assert.equal(manager.snapshot().count, 1);
  await manager.stop();
  assert.equal(manager.snapshot().count, 0);
});

test('account preparation uses each session home and passes only session path arguments', async t => {
  const homes = [], cleaned = [];
  const { manager, spawned, store } = fixture(t, { prepareAccount: async home => {
    homes.push(home);
    return { args: ['--session-file', path.join(home, 'sessions', 'once.json')], cleanup: async () => { cleaned.push(home); } };
  } });
  await manager.launch('atlas');
  await manager.launchAnother('atlas');
  assert.equal(homes[0], store.instancePath('atlas'));
  assert.notEqual(homes[1], homes[0]);
  assert.equal(cleaned.length, 0, 'a live process may still read its one-time session');
  for (let index = 0; index < spawned.length; index++) {
    const args = spawned[index].args;
    assert.equal(args[args.indexOf('--session-file') + 1], path.join(homes[index], 'sessions', 'once.json'));
    assert.ok(!args.some(arg => /access_token|refresh_token/i.test(arg)));
  }
  const [main, extra] = manager.snapshot().sessions;
  await manager.stop(extra.id);
  await manager.stop(main.id);
  assert.deepEqual(cleaned, [homes[1], homes[0]]);
});

test('authentication refusal never spawns and releases launch locks for a retry', async t => {
  const { manager, spawned, store } = fixture(t, { prepareAccount: async () => { throw new Error('expired account'); } });
  await assert.rejects(manager.launch('atlas'), /expired account/);
  assert.equal(spawned.length, 0);
  assert.equal(manager.snapshot().count, 0);
  assert.equal(manager.pendingHomes.size, 0);
  assert.equal(manager.isInstanceRunning('atlas'), false);
  assert.equal(store.requireInstance('atlas').lastPlayed, null);
  manager.prepareAccount = async () => ({ args: ['--name', 'AtlasLocal'], cleanup: async () => {} });
  await manager.launch('atlas');
  assert.ok(spawned[0].args.includes('AtlasLocal'));
});

test('synchronous spawn failure cleans up prepared credentials', async t => {
  let cleanups = 0;
  const { manager, spawned } = fixture(t, { spawnThrows: true, prepareAccount: async () => ({ args: [], cleanup: async () => { cleanups++; } }) });
  await assert.rejects(manager.launch('atlas'), /native spawn failed/);
  assert.equal(spawned.length, 0);
  assert.equal(cleanups, 1);
  assert.equal(manager.pendingHomes.size, 0);
  assert.equal(manager.isInstanceRunning('atlas'), false);
});

test('asynchronous spawn error cleans credentials once and keeps the original failure', async t => {
  let cleanups = 0;
  const { manager, logs } = fixture(t, { spawnError: true, prepareAccount: async () => ({ args: [], cleanup: async () => {
    cleanups++;
    throw new Error('credential cleanup failed with sensitive diagnostic');
  } }) });
  await assert.rejects(manager.launch('atlas'), /asynchronous native spawn failed/);
  assert.equal(cleanups, 1);
  assert.equal(manager.snapshot().count, 0);
  assert.equal(manager.isInstanceRunning('atlas'), false);
  assert.ok(logs.some(item => item.message === 'Could not remove an expired account session.'));
  assert.ok(!logs.some(item => /sensitive diagnostic/.test(item.message)));
});

test('credential cleanup failure cannot replace a synchronous spawn error', async t => {
  const { manager } = fixture(t, { spawnThrows: true, prepareAccount: async () => ({ args: [], cleanup: async () => { throw new Error('cleanup failed'); } }) });
  await assert.rejects(manager.launch('atlas'), /native spawn failed/);
});
