const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { test } = require('node:test');
const { createHash } = require('node:crypto');
const { syncNativeStage } = require('../lib/native-stage.cjs');
const { guard } = require('../lib/release-updater.cjs');
const { nativeWorkspace } = require('../lib/native-overlay.cjs');

async function fixture(t) {
  const temp = await fs.realpath(os.tmpdir());
  const root = await fs.mkdtemp(path.join(temp, 'atlas-native-stage-test-'));
  t.after(() => fs.rm(guard(temp, root), { recursive: true, force: true }));
  const source = path.join(root, 'fresh');
  const stage = path.join(root, 'stage');
  await fs.mkdir(source);
  return { root, source, stage };
}

test('native stage keeps unchanged timestamps and updates or removes files and directories', async t => {
  const { source, stage } = await fixture(t);
  const write = (name, data) => fs.writeFile(path.join(source, name), data);
  await write('same.rs', 'same');
  await write('change.rs', 'old!');
  await write('to-dir', 'old file');
  await fs.mkdir(path.join(source, 'to-file'));
  await write(path.join('to-file', 'obsolete.rs'), 'old');
  await write('obsolete.rs', 'old');
  await syncNativeStage(source, stage);

  const oldDate = new Date('2020-01-01T00:00:00Z');
  for (const name of ['same.rs', 'change.rs']) await fs.utimes(path.join(stage, name), oldDate, oldDate);
  await write('change.rs', 'new!');
  await fs.utimes(path.join(source, 'change.rs'), oldDate, oldDate);
  await fs.unlink(path.join(source, 'to-dir'));
  await fs.mkdir(path.join(source, 'to-dir'));
  await write(path.join('to-dir', 'new.rs'), 'nested');
  await fs.unlink(path.join(source, 'to-file', 'obsolete.rs'));
  await fs.rmdir(path.join(source, 'to-file'));
  await write('to-file', 'new file');
  await fs.unlink(path.join(source, 'obsolete.rs'));

  await syncNativeStage(source, stage);
  assert.equal((await fs.stat(path.join(stage, 'same.rs'))).mtimeMs, oldDate.getTime());
  assert.ok((await fs.stat(path.join(stage, 'change.rs'))).mtimeMs > oldDate.getTime());
  assert.equal(await fs.readFile(path.join(stage, 'change.rs'), 'utf8'), 'new!');
  assert.equal(await fs.readFile(path.join(stage, 'to-dir', 'new.rs'), 'utf8'), 'nested');
  assert.equal(await fs.readFile(path.join(stage, 'to-file'), 'utf8'), 'new file');
  await assert.rejects(fs.stat(path.join(stage, 'obsolete.rs')), { code: 'ENOENT' });
});

test('native stage refuses overlap and linked directories without touching their contents', async t => {
  const { root, source, stage } = await fixture(t);
  await assert.rejects(syncNativeStage(source, path.join(source, 'nested')), /overlap/i);
  await fs.mkdir(stage);
  const outside = path.join(root, 'outside');
  await fs.mkdir(outside);
  const sentinel = path.join(outside, 'keep.rs');
  await fs.writeFile(sentinel, 'keep');
  const link = path.join(stage, 'linked');
  await fs.symlink(outside, link, process.platform === 'win32' ? 'junction' : 'dir');
  t.after(() => fs.unlink(link).catch(() => {}));
  await assert.rejects(syncNativeStage(source, stage), /linked/i);
  assert.equal(await fs.readFile(sentinel, 'utf8'), 'keep');
});

test('native overlays verify source content, deduplicate compatible changes and only modify a scratch copy', async t => {
  const { root, source, stage } = await fixture(t);
  const mods = path.join(root, 'mods');
  const relative = 'crates/game/src/lib.rs';
  const digest = value => createHash('sha256').update(value).digest('hex');
  const original = path.join(source, relative);
  await fs.mkdir(path.dirname(original), { recursive: true });
  await fs.writeFile(original, 'old');
  await fs.mkdir(stage);
  async function writeMod(id, value, before = digest('old')) {
    const dir = path.join(mods, id);
    const file = path.join(dir, 'source-overlay', relative);
    await fs.mkdir(path.dirname(file), { recursive: true });
    await fs.writeFile(file, value);
    await fs.writeFile(path.join(dir, 'source-overlay.json'), JSON.stringify({
      schema: 1, files: [{ path: relative, before, after: digest(value) }],
    }));
  }
  await writeMod('first', 'new');
  await writeMod('second', 'new');
  const applied = await nativeWorkspace(source, stage, mods, ['first', 'second']);
  assert.equal(await fs.readFile(path.join(applied, relative), 'utf8'), 'new');
  assert.equal(await fs.readFile(original, 'utf8'), 'old');
  await writeMod('second', 'different');
  await assert.rejects(nativeWorkspace(source, stage, mods, ['first', 'second']), /conflicting/i);
  await writeMod('second', 'new', digest('wrong base'));
  await assert.rejects(nativeWorkspace(source, stage, mods, ['second']), /does not match/i);
  assert.equal(await fs.readFile(original, 'utf8'), 'old');
});
