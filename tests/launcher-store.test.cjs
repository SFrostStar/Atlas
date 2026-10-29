const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { LauncherStore, buildLaunchSpec } = require('../lib/launcher-store.cjs');

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-launcher-test-'));
  t.after(() => {
    if (!path.resolve(root).startsWith(`${path.resolve(os.tmpdir())}${path.sep}atlas-launcher-test-`)) throw new Error('Unsafe test cleanup path');
    fs.rmSync(root, { recursive: true, force: true });
  });
  return new LauncherStore(root, { appData: path.join(root, 'appdata') });
}

test('instances persist and display names cannot redirect instance folders', (t) => {
  const store = fixture(t);
  const instance = store.createInstance({ name: '../../Мой мир', description: 'Отдельные сохранения', accent: 'blue' });
  const state = new LauncherStore(store.projectRoot).getState();
  assert.equal(state.selectedId, instance.id);
  assert.equal(state.instances.length, 2);
  const folder = store.instancePath(instance.id);
  assert.equal(path.dirname(folder), path.join(store.dataRoot, 'instances'));
  assert.ok(fs.existsSync(path.join(folder, 'saves')));
  assert.throws(() => store.folder('instance', '../atlas'), /не найдена/);
  assert.throws(() => store.createInstance({ name: '../../мой мир' }), /уже есть/);
});

test('an explicit data root isolates GUI test data without changing project runtime/source paths', (t) => {
  const store = fixture(t);
  const projectLegacy = path.join(store.projectRoot, 'horizon-extras.txt');
  fs.writeFileSync(projectLegacy, 'unknown:keep-real-project-settings\n');
  const dataRoot = path.join(store.projectRoot, 'work', 'gui-test-data');
  const isolated = new LauncherStore(store.projectRoot, { dataRoot });
  assert.equal(fs.readFileSync(projectLegacy, 'utf8'), 'unknown:keep-real-project-settings\n');
  assert.equal(fs.existsSync(path.join(store.projectRoot, 'atlas-extras.txt')), false);
  isolated.createInstance({ name: 'GUI-only instance' });
  assert.equal(isolated.dataRoot, dataRoot);
  assert.equal(isolated.getState().projectRoot, store.projectRoot);
  assert.equal(isolated.getState().sourceRoot, path.join(store.projectRoot, 'engine', 'atlas'));
  assert.equal(isolated.atlasRuntime, path.join(store.projectRoot, 'runtime', 'atlas-client.exe'));
  assert.equal(fs.existsSync(path.join(dataRoot, 'launcher.json')), true);
  assert.equal(new LauncherStore(store.projectRoot).getState().instances.length, 1);
  assert.ok(isolated.instancePath(isolated.config.selectedId).startsWith(dataRoot));
  assert.throws(() => new LauncherStore(store.projectRoot, { dataRoot: 'relative' }), /полный путь/);
});

test('launch specification isolates HORIZON_HOME and preserves argument boundaries', (t) => {
  const store = fixture(t);
  const runtime = path.join(store.projectRoot, 'custom build', 'horizon-client.exe');
  fs.mkdirSync(path.dirname(runtime), { recursive: true });
  fs.writeFileSync(runtime, 'test fixture');
  const resources = path.join(path.dirname(runtime), '.local', 'horizon-client', '26.3');
  for (const name of ['assets', 'data']) fs.mkdirSync(path.join(resources, name), { recursive: true });
  store.saveSettings({ runtimePath: runtime });
  const instance = store.createInstance({ name: 'Dev' });
  const spec = buildLaunchSpec(store, instance.id);
  assert.equal(spec.executable, runtime);
  assert.deepEqual(spec.args, ['--resources', resources]);
  assert.equal(spec.cwd, store.instancePath(instance.id));
  assert.equal(spec.env.HORIZON_HOME, spec.cwd);
  assert.notEqual(spec.env.HORIZON_HOME, process.env.HORIZON_HOME);
});

test('missing runtime and malformed settings produce actionable errors', (t) => {
  const store = fixture(t);
  assert.throws(() => buildLaunchSpec(store, 'atlas'), /Клиент не найден/);
  assert.throws(() => store.saveSettings({ runtimePath: 'relative.exe' }), /полный путь/);
  assert.throws(() => store.saveSettings({ closeOnLaunch: 'yes' }), /Некорректная/);
  assert.throws(() => store.createInstance({ name: ' ' }), /название сборки/);
});

test('previous default source path migrates to Atlas without changing custom paths', (t) => {
  const store = fixture(t);
  const oldPath = path.join(store.projectRoot, 'engine', 'horizon');
  store.saveSettings({ sourceRoot: oldPath });
  const migrated = new LauncherStore(store.projectRoot);
  assert.equal(migrated.getState().sourceRoot, path.join(store.projectRoot, 'engine', 'atlas'));
  const custom = path.join(store.projectRoot, 'my-custom-source');
  migrated.saveSettings({ sourceRoot: custom });
  const preserved = new LauncherStore(store.projectRoot);
  assert.equal(preserved.getState().sourceRoot, custom);
  assert.equal(preserved.folder('guide'), path.join(store.projectRoot, 'docs', 'EDITING-ATLAS.md'));
});

test('game options stay inside their instance and preserve unrecognized settings', (t) => {
  const store = fixture(t);
  const second = store.createInstance({ name: 'Second' });
  assert.equal(store.gameOptions(second.id).mouseTweaksWheel, true);
  const extras = path.join(store.instancePath('atlas'), 'atlas-extras.txt');
  fs.writeFileSync(extras, 'unknown:preserve-me\nfullbright:false\n');
  store.setGameOptions('atlas', { fullbright: true, dynamicLights: true });
  assert.equal(store.gameOptions('atlas').fullbright, true);
  assert.equal(store.gameOptions(second.id).fullbright, false);
  assert.match(fs.readFileSync(extras, 'utf8'), /unknown:preserve-me/);
  assert.throws(() => store.setGameOptions('atlas', { unexpected: true }), /Неизвестная/);
});

test('a completed Atlas build replaces only the original default client', (t) => {
  const store = fixture(t);
  assert.equal(store.getState().runtimePath, store.defaultRuntime);
  fs.mkdirSync(path.dirname(store.atlasRuntime), { recursive: true });
  fs.writeFileSync(store.atlasRuntime, 'atlas fixture');
  assert.equal(store.getState().runtimePath, store.atlasRuntime);
  const persisted = JSON.parse(fs.readFileSync(store.file, 'utf8'));
  assert.equal(persisted.settings.runtimePath, store.atlasRuntime);

  const custom = path.join(store.projectRoot, 'custom', 'my-client.exe');
  store.saveSettings({ runtimePath: custom });
  assert.equal(store.getState().runtimePath, custom);
  const reopened = new LauncherStore(store.projectRoot, { appData: path.join(store.projectRoot, 'appdata') });
  assert.equal(reopened.getState().runtimePath, custom);
});

test('a new installation uses Atlas runtime and its own prepared resources', (t) => {
  const store = fixture(t);
  fs.mkdirSync(path.dirname(store.atlasRuntime), { recursive: true });
  fs.writeFileSync(store.atlasRuntime, 'atlas fixture');
  const resources = path.join(path.dirname(store.defaultRuntime), '.local', 'atlas-client', '26.3');
  for (const name of ['assets', 'data']) fs.mkdirSync(path.join(resources, name), { recursive: true });
  const restarted = new LauncherStore(store.projectRoot, { appData: path.join(store.projectRoot, 'appdata') });
  assert.equal(restarted.getState().runtimePath, store.atlasRuntime);
  const spec = buildLaunchSpec(restarted, 'atlas');
  assert.equal(spec.executable, store.atlasRuntime);
  assert.deepEqual(spec.args, ['--resources', resources]);
});
