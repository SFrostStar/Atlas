const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { randomUUID } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const { guard } = require('../lib/release-updater.cjs');
const { NativeMods, trimCargoTarget } = require('../lib/native-mods.cjs');

async function scratch(t) {
  const parent = path.resolve(os.tmpdir());
  const root = await fs.mkdtemp(path.join(parent, 'atlas-native-cache-'));
  t.after(() => fs.rm(guard(parent, root), { recursive: true, force: true }));
  return root;
}

test('native maintenance preserves live build claims, current sources and compiled clients', async t => {
  const dataRoot = await scratch(t);
  const version = '1.4.1';
  const sourceKey = 'b'.repeat(64), oldKey = 'a'.repeat(64), unknownKey = 'c'.repeat(64);
  const sourceManifest = path.join(dataRoot, 'native-source.json');
  await fs.writeFile(sourceManifest, JSON.stringify({ schema: 1, version, revision: 'd'.repeat(40), sourceKey, size: 22, sha256: 'a'.repeat(64) }));
  const manager = new NativeMods({ dataRoot, version, sourceManifest });
  async function put(relative, bytes) {
    const file = path.join(manager.root, relative);
    await fs.mkdir(path.dirname(file), { recursive: true });
    await fs.writeFile(file, bytes);
    return file;
  }
  const current = await put(`source-${sourceKey}/.atlas-source-key`, sourceKey);
  const old = await put(`source-${oldKey}/.atlas-source-key`, oldKey);
  const unknown = await put(`source-${unknownKey}/.atlas-source-key`, 'foreign');
  const interrupted = await put('input-a1b2c3/stage/Cargo.toml', 'old build');
  const recent = await put('input-z3y2x1/stage/Cargo.toml', 'recent build');
  const ready = await put(`builds/${sourceKey}/atlas-client.exe`, 'compiled executable');
  const unrelated = await put('tools/cargo/cargo.exe', 'compiler');
  const age = new Date('2020-01-01');
  await fs.utimes(path.join(manager.root, 'input-a1b2c3'), age, age);

  const exited = spawnSync(process.execPath, ['-e', ''], { windowsHide: true });
  assert.equal(exited.status, 0);
  const claim = await put(`cache-${exited.pid}-${randomUUID()}.lock`, String(process.pid));
  await manager.maintain();
  assert.equal(await fs.readFile(interrupted, 'utf8'), 'old build');
  assert.equal(await fs.readFile(old, 'utf8'), oldKey);

  await fs.unlink(claim);
  await manager.maintain();
  await assert.rejects(fs.access(interrupted), { code: 'ENOENT' });
  await assert.rejects(fs.access(old), { code: 'ENOENT' });
  assert.deepEqual(await Promise.all([current, unknown, recent, ready, unrelated].map(file => fs.readFile(file, 'utf8'))),
    [sourceKey, 'foreign', 'recent build', 'compiled executable', 'compiler']);
});

test('trim keeps usable Cargo artifacts within budget and leaves separate final builds', async t => {
  const root = await scratch(t);
  const target = path.join(root, 'target');
  const incremental = path.join(target, 'fast', 'incremental');
  const dependencies = path.join(target, 'fast', 'deps');
  const ready = path.join(root, 'builds', 'atlas-client.exe');
  for (const directory of [incremental, dependencies, path.dirname(ready)]) await fs.mkdir(directory, { recursive: true });
  await fs.writeFile(ready, 'compiled executable');
  await fs.writeFile(path.join(incremental, 'partial'), Buffer.alloc(650));
  await fs.writeFile(path.join(dependencies, 'library.rlib'), Buffer.alloc(90));

  await trimCargoTarget(root, 512);
  await fs.access(incremental);
  await fs.writeFile(path.join(target, 'CACHEDIR.TAG'), 'Signature: 8a477f597d28d172789f06886806bc55\n');
  await trimCargoTarget(root, 512);
  await assert.rejects(fs.access(incremental), { code: 'ENOENT' });
  await fs.access(dependencies);

  await fs.writeFile(path.join(dependencies, 'library.rlib'), Buffer.alloc(640));
  await trimCargoTarget(root, 512);
  await assert.rejects(fs.access(target), { code: 'ENOENT' });
  assert.equal(await fs.readFile(ready, 'utf8'), 'compiled executable');
});
