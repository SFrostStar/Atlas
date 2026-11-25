const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const http = require('node:http');
const zlib = require('node:zlib');
const { once, EventEmitter } = require('node:events');
const { execFileSync } = require('node:child_process');
const { ReleaseUpdater, pruneUpdates, version, newer, safeUrl, allowedFile, validateManifest, extractArchive, REQUIRED, REPOSITORY, hashFile } = require('../lib/release-updater.cjs');
const { NativeMods } = require('../lib/native-mods.cjs');

const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
function scratch(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-release-test-'));
  t.after(() => {
    assert.equal(path.dirname(path.resolve(root)), path.resolve(os.tmpdir()));
    assert.ok(path.basename(root).startsWith('atlas-release-test-'));
    fs.rmSync(root, { recursive: true, force: true });
  });
  return root;
}
function crc32(bytes) {
  let crc = -1;
  for (const byte of bytes) { crc ^= byte; for (let bit = 0; bit < 8; bit++) crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1; }
  return (crc ^ -1) >>> 0;
}
function zip(files) {
  const local = [], central = []; let offset = 0, centralSize = 0;
  for (const [name, value] of files) {
    const bytes = Buffer.from(value), compressed = zlib.deflateRawSync(bytes), filename = Buffer.from(name), crc = crc32(bytes);
    const header = Buffer.alloc(30); header.writeUInt32LE(0x04034b50, 0); header.writeUInt16LE(20, 4); header.writeUInt16LE(8, 8); header.writeUInt32LE(crc, 14); header.writeUInt32LE(compressed.length, 18); header.writeUInt32LE(bytes.length, 22); header.writeUInt16LE(filename.length, 26);
    local.push(header, filename, compressed);
    const entry = Buffer.alloc(46); entry.writeUInt32LE(0x02014b50, 0); entry.writeUInt16LE(20, 4); entry.writeUInt16LE(20, 6); entry.writeUInt16LE(8, 10); entry.writeUInt32LE(crc, 16); entry.writeUInt32LE(compressed.length, 20); entry.writeUInt32LE(bytes.length, 24); entry.writeUInt16LE(filename.length, 28); entry.writeUInt32LE(offset, 42);
    central.push(entry, filename); centralSize += entry.length + filename.length; offset += header.length + filename.length + compressed.length;
  }
  const end = Buffer.alloc(22); end.writeUInt32LE(0x06054b50, 0); end.writeUInt16LE(files.length, 8); end.writeUInt16LE(files.length, 10); end.writeUInt32LE(centralSize, 12); end.writeUInt32LE(offset, 16);
  return Buffer.concat([...local, ...central, end]);
}
function releaseFixture() {
  const contents = REQUIRED.map(name => [name, `new ${name}`]);
  const archive = zip(contents), archiveName = 'Atlas-1.1.0-Update.zip';
  const base = `https://github.com/${REPOSITORY}/releases/download/atlas-1.1.0`;
  const manifest = { schema: 1, repository: REPOSITORY, platform: 'win32-x64', version: '1.1.0', tag: 'atlas-1.1.0', archive: { name: archiveName, size: archive.length, sha256: digest(archive) }, files: contents.map(([name, bytes]) => ({ path: name, size: Buffer.byteLength(bytes), sha256: digest(bytes) })) };
  const release = { tag_name: manifest.tag, prerelease: false, draft: false, assets: [{ name: 'atlas-update.json', size: Buffer.byteLength(JSON.stringify(manifest)), browser_download_url: `${base}/atlas-update.json` }, { name: archiveName, size: archive.length, browser_download_url: `${base}/${archiveName}` }] };
  return { release, manifest, archive, contents };
}
async function fixtureServer(t, fixture, modify) {
  const requests = [];
  const server = http.createServer((request, response) => {
    const productionUrl = decodeURIComponent(request.url.slice(1)); requests.push(productionUrl);
    if (modify?.(productionUrl, response)) return;
    if (productionUrl.endsWith('/releases/latest')) response.end(JSON.stringify(fixture.release));
    else if (productionUrl.endsWith('/atlas-update.json')) response.end(JSON.stringify(fixture.manifest));
    else if (productionUrl.endsWith('-Update.zip')) response.end(fixture.archive);
    else { response.writeHead(404); response.end(); }
  });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  t.after(() => new Promise(resolve => server.close(resolve)));
  const fetch = (url, options) => globalThis.fetch(`http://127.0.0.1:${server.address().port}/${encodeURIComponent(url)}`, options);
  return { fetch, requests };
}
function updater(root, fetch, extra = {}) {
  return new ReleaseUpdater({ projectRoot: root, dataRoot: path.join(root, 'data'), executable: path.join(root, 'Atlas Launcher.exe'), currentVersion: '1.0.0', enabled: true, fetch, ...extra });
}

test('native source download waits for consent and is shared by builds after restart', {
  skip: process.platform !== 'win32' || process.arch !== 'x64',
}, async t => {
  const root = scratch(t), home = path.join(root, 'instance'), dataRoot = path.join(root, 'data');
  const sourceManifest = path.join(root, 'native-source.json');
  const revision = 'a'.repeat(40), sourceKey = 'b'.repeat(64), version = '1.4.1';
  const archive = zip([
    ['.atlas-source-key', sourceKey], ['ATLAS-UPSTREAM.json', JSON.stringify({ commit: revision })],
    ['Cargo.toml', '[workspace]'], ['Cargo.lock', 'version = 4'],
    ['crates/sample/src/lib.rs', 'pub fn noop() {}'],
  ]);
  fs.writeFileSync(sourceManifest, JSON.stringify({ schema: 1, version, revision, sourceKey, size: archive.length, sha256: digest(archive) }));
  const downloads = [], stages = [];
  let valid = false;
  const toolStop = new Error('sources reached the compiler');
  const create = confirm => new NativeMods({
    project: root, dataRoot, sourceManifest, version, confirm,
    fetch(url) {
      downloads.push(url);
      return new Response(valid ? archive : Buffer.alloc(archive.length));
    },
    progress(stage) { stages.push(stage); },
    tools() { throw toolStop; },
  });
  assert.equal(await create(() => assert.fail('No mods to approve')).prepare(home), null);
  const mods = path.join(home, 'mods');
  fs.mkdirSync(mods, { recursive: true });
  const mod = zip([
    ['atlas-native.json', JSON.stringify({ schema: 1, id: 'sample', version: '1', game: '26.3', atlas: revision, mixins: '5a64af938ec86b00d53d16a408aabc0038fb3e57' })],
    ['mixins/sample/atlas-mixin.toml', '[mod]\nid = "sample"'],
  ]);
  fs.writeFileSync(path.join(mods, 'Sample.hnmod'), mod);
  assert.deepEqual(await create(() => false).prepare(home), { cancelled: true });
  assert.deepEqual(downloads, []);

  const nativeRoot = path.join(dataRoot, 'native-mods');
  const sources = path.join(nativeRoot, `source-${sourceKey}`);
  await assert.rejects(create(() => true).prepare(home), /checksum/i);
  assert.equal(fs.existsSync(sources), false);
  valid = true;
  await assert.rejects(create(() => true).prepare(home), toolStop);
  assert.equal(fs.readFileSync(path.join(sources, 'crates/sample/src/lib.rs'), 'utf8'), 'pub fn noop() {}');
  await assert.rejects(create(() => true).prepare(home), toolStop);
  assert.deepEqual(downloads, Array(2).fill(`https://github.com/${REPOSITORY}/releases/download/atlas-payload-1.4.1/Atlas-1.4.1-Native-Source.zip`));
  assert(stages.includes('source'));

  const iconRoot = path.join(__dirname, '..', 'assets');
  const png = path.join(iconRoot, 'game-icon.png'), ico = path.join(iconRoot, 'game-icon.ico');
  const icons = [digest(fs.readFileSync(fs.existsSync(png) ? png : path.join(iconRoot, 'icon.png'))), fs.existsSync(ico) ? digest(fs.readFileSync(ico)) : null];
  const built = path.join(nativeRoot, 'builds', digest(JSON.stringify([version, sourceKey, [digest(mod)], icons])));
  fs.mkdirSync(built, { recursive: true });
  fs.writeFileSync(path.join(built, 'complete'), path.basename(built));
  fs.writeFileSync(path.join(built, 'atlas-client.exe'), 'compiled test client');
  const state = await create(() => assert.fail('The cached build must skip consent')).prepare(home);
  assert.equal(state.executable, path.join(built, 'atlas-client.exe'));
  assert.equal(downloads.length, 2);
});

test('Atlas semantic versions sort numerically and retain legacy v0.1 support', () => {
  assert.equal(version('v0.1'), '0.1.0'); assert.equal(version('atlas-1.0.0'), '1.0.0');
  assert.equal(newer('atlas-1.10.0', 'atlas-1.9.9'), true);
  assert.equal(newer('atlas-1.0.0', '1.0.0'), false);
  for (const bad of ['nightly-2026-11-05-66f3981', '1.0.0-beta', '01.0.0', '1.2.3.4']) assert.throws(() => version(bad));
});
test('download URLs and installed paths cannot target external hosts or user data', () => {
  assert.match(safeUrl('https://github.com/SFrostStar/Atlas/releases/download/atlas-1.0.0/file.zip', true), /github/);
  for (const bad of ['http://github.com/SFrostStar/Atlas/releases/download/x/a', 'https://github.com/attacker/Atlas/releases/download/x/a', 'https://github.com.evil.test/SFrostStar/Atlas/releases/download/x/a', 'file:///tmp/a', 'https://user:pass@github.com/x']) assert.throws(() => safeUrl(bad, true));
  for (const name of ['data/worlds/a', 'runtime/.local/a', 'bundle/game/game-root/data/a', 'bundle/game/game-root/usercache.json', 'resources/accounts.json', 'launcher.json', 'bundle/game/bad.exe']) assert.equal(allowedFile(name), false);
  for (const name of ['../data/a', '/data/a', 'bundle/game/../data/a', 'resources\\app.asar', 'resources/app.asar:evil', 'resources/NUL.txt']) assert.throws(() => allowedFile(name));
});
test('manifest binds exact stable version, checksum inventory and required Atlas binaries', () => {
  const { manifest, release } = releaseFixture();
  assert.equal(validateManifest(manifest, release).version, '1.1.0');
  for (const change of [value => value.version = '1.2.0', value => value.files.pop(), value => value.files.push(value.files[0]), value => value.files[0].path = 'data/worlds/x', value => value.archive.sha256 = 'bad', value => value.archive.size++]) {
    const value = structuredClone(manifest); change(value); assert.throws(() => validateManifest(value, release));
  }
});
test('real HTTP download extracts verified files and reuses staged version across sessions', async t => {
  const root = scratch(t), fixture = releaseFixture(), server = await fixtureServer(t, fixture);
  const first = updater(root, server.fetch);
  assert.equal((await first.check()).status, 'available');
  assert.equal((await first.download()).status, 'ready');
  assert.equal(fs.existsSync(path.join(first.stageDirectory, 'update.zip')), false);
  for (const [name, content] of fixture.contents) assert.equal(fs.readFileSync(path.join(first.staged, name), 'utf8'), content);
  const second = updater(root, server.fetch);
  assert.equal((await second.check()).status, 'ready');
  assert.equal(server.requests.filter(url => url.endsWith('-Update.zip')).length, 1);
  fs.writeFileSync(path.join(second.staged, 'Atlas Launcher.exe'), 'corrupt');
  assert.equal((await second.check()).status, 'available');
});
test('corrupt and oversized archive never become ready or overwrite live files', async t => {
  const root = scratch(t), fixture = releaseFixture();
  const original = path.join(root, 'Atlas Launcher.exe'); fs.writeFileSync(original, 'old launcher');
  fixture.archive = Buffer.concat([fixture.archive, Buffer.from('tampered')]);
  const server = await fixtureServer(t, fixture), instance = updater(root, server.fetch);
  await instance.check(); const result = await instance.download();
  assert.equal(result.status, 'error'); assert.match(result.error, /size|checksum/);
  assert.equal(fs.readFileSync(original, 'utf8'), 'old launcher');
  assert.equal(fs.existsSync(path.join(instance.root, 'ready.json')), false);
  assert.deepEqual(fs.readdirSync(instance.root), [], 'a failed download removes its whole stage');
});
test('startup prunes obsolete update stages and preserves an in-flight download, helper and retry', async t => {
  const root = scratch(t), data = path.join(root, 'data'), updates = path.join(data, 'updates');
  const stage = (name, files) => {
    const dir = path.join(updates, name);
    fs.mkdirSync(dir, { recursive: true });
    for (const [file, bytes] of Object.entries(files)) fs.writeFileSync(path.join(dir, file), bytes);
    return dir;
  };
  const old = stage('atlas-0.9.0-abc222', { 'update.zip': 'old download' });
  const downloading = stage('atlas-0.9.1-abc333', { '.download-owner.json': JSON.stringify({ pid: process.pid }), 'update.zip.part': 'in flight' });
  const attempt = 'de55de55-12ea-4ced-b83c-013f531184d1';
  const installing = stage('atlas-0.9.2-abc444', {
    [`install-plan-${attempt}.json`]: JSON.stringify({ parentPid: 2147483647 }),
    [`helper-started-${attempt}.json`]: JSON.stringify({ helperPid: process.pid }),
  });
  const oldTime = new Date(Date.now() - 2 * 24 * 60 * 60 * 1000);
  fs.utimesSync(installing, oldTime, oldTime);
  fs.utimesSync(path.join(installing, `install-plan-${attempt}.json`), oldTime, oldTime);
  const retry = stage('atlas-1.1.0-abc555', { 'update.zip': 'previously verified' });
  const other = stage('user-notes', { 'notes.txt': 'keep me' });
  const unrecognized = stage('atlas-0.7.0-abc666', { 'notes.txt': 'keep me' });
  const outside = path.join(root, 'external'); fs.mkdirSync(outside); fs.writeFileSync(path.join(outside, 'world.dat'), 'keep me');
  const linked = path.join(updates, 'atlas-0.8.0-abc777'); fs.symlinkSync(outside, linked, process.platform === 'win32' ? 'junction' : 'dir');
  fs.writeFileSync(path.join(updates, 'ready.json'), JSON.stringify({ tag: 'atlas-1.1.0', directory: path.basename(retry) }));
  await pruneUpdates(data, '1.0.0');
  assert.equal(fs.existsSync(old), false);
  for (const dir of [downloading, installing, retry, other, unrecognized, linked]) assert.equal(fs.existsSync(dir), true, dir);
  assert.equal(fs.readFileSync(path.join(outside, 'world.dat'), 'utf8'), 'keep me');
});
test('a new verified update replaces the superseded staging, while failed refresh keeps it', async t => {
  const root = scratch(t), fixture = releaseFixture();
  const archive = fixture.archive;
  fixture.archive = Buffer.from('corrupt');
  const server = await fixtureServer(t, fixture);
  const instance = updater(root, server.fetch);
  const older = path.join(instance.root, 'atlas-1.0.1-abc888');
  fs.mkdirSync(older, { recursive: true }); fs.writeFileSync(path.join(older, 'update.zip'), 'earlier verified update');
  fs.writeFileSync(path.join(instance.root, 'ready.json'), JSON.stringify({ tag: 'atlas-1.0.1', directory: path.basename(older) }));
  assert.equal((await instance.check()).status, 'available');
  assert.equal((await instance.download()).status, 'error');
  assert.equal(fs.existsSync(older), true);
  fixture.archive = archive;
  assert.equal((await instance.download()).status, 'ready');
  assert.equal(fs.existsSync(older), false);
  assert.ok(fs.existsSync(instance.staged));
});
test('redirects are checked before any request can reach a different service', async t => {
  const root = scratch(t), fixture = releaseFixture();
  const server = await fixtureServer(t, fixture, (url, response) => {
    response.writeHead(302, { Location: 'https://evil.test/payload' }); response.end(); return true;
  });
  const instance = updater(root, server.fetch), result = await instance.check();
  assert.equal(result.status, 'error'); assert.match(result.error, /Untrusted/); assert.equal(server.requests.length, 1);
});
test('ZIP traversal, unlisted files and content hash mismatches fail before installation', async t => {
  const root = scratch(t), bytes = Buffer.from('hello');
  for (const [index, name] of ['../escaped.txt', 'data/worlds/a', 'resources/app.asar'].entries()) {
    const archive = path.join(root, `case-${index}.zip`); fs.writeFileSync(archive, zip([[name, bytes]]));
    const inventory = [{ path: 'resources/app.asar', size: bytes.length, sha256: digest('different') }];
    await assert.rejects(() => extractArchive(archive, path.join(root, `out-${index}`), inventory));
  }
  assert.equal(fs.existsSync(path.join(root, 'escaped.txt')), false);
});
test('linked staging or destination folders are refused', async t => {
  const root = scratch(t), fixture = releaseFixture(), server = await fixtureServer(t, fixture);
  const destination = path.join(root, 'elsewhere'); fs.mkdirSync(destination);
  fs.mkdirSync(path.join(root, 'data')); fs.symlinkSync(destination, path.join(root, 'data', 'updates'), 'junction');
  const instance = updater(root, server.fetch); await instance.check();
  assert.equal((await instance.download()).status, 'error');
  assert.deepEqual(fs.readdirSync(destination), []);
});
test('running game blocks install; dist checkout plans use separate app, runtime and bundle roots', async t => {
  const root = scratch(t), fixture = releaseFixture(), server = await fixtureServer(t, fixture);
  let running = true, invocation;
  const fakeSpawn = (...args) => { invocation = args; const child = new EventEmitter(); child.unref = () => {}; const plan = JSON.parse(fs.readFileSync(args[1][args[1].indexOf('-PlanPath') + 1], 'utf8')); fs.writeFileSync(plan.started, JSON.stringify({ version: plan.version, parentPid: plan.parentPid })); queueMicrotask(() => child.emit('spawn')); return child; };
  const instance = updater(root, server.fetch, { executable: path.join(root, 'dist', 'Atlas Launcher-win32-x64', 'Atlas Launcher.exe'), isGameRunning: () => running, spawnProcess: fakeSpawn });
  await instance.check(); await instance.download();
  await assert.rejects(() => instance.install(), /Close Minecraft/);
  assert.equal(invocation, undefined); running = false;
  await instance.check();
  assert.equal((await instance.install({ restart: false })).status, 'installing');
  assert.equal(invocation[2].windowsHide, true); assert.equal(invocation[2].detached, false);
  const plan = JSON.parse(fs.readFileSync(invocation[1][invocation[1].indexOf('-PlanPath') + 1], 'utf8'));
  assert.equal(plan.appRoot, path.join(root, 'dist', 'Atlas Launcher-win32-x64'));
  assert.equal(plan.projectRoot, root); assert.equal(plan.restart, false);
  assert.ok(plan.files.some(file => file.kind === 'runtime' && file.relative === 'atlas-client.exe'));
  assert.ok(plan.files.every(file => !file.relative.startsWith('data/')));
  assert.ok(plan.files.every(file => file.relative !== 'atlas-portable.json'));
});
test('concurrent installs spawn once and a failed spawn can retry without stale plan collisions', async t => {
  const root = scratch(t), fixture = releaseFixture(), server = await fixtureServer(t, fixture);
  let calls = 0;
  const spawnProcess = (...args) => {
    calls++; const child = new EventEmitter(); child.unref = () => {};
    if (calls > 1) { const plan = JSON.parse(fs.readFileSync(args[1][args[1].indexOf('-PlanPath') + 1], 'utf8')); fs.writeFileSync(plan.started, JSON.stringify({ version: plan.version, parentPid: plan.parentPid })); }
    queueMicrotask(() => child.emit(calls === 1 ? 'error' : 'spawn', new Error('test CreateProcess failed')));
    return child;
  };
  const instance = updater(root, server.fetch, { spawnProcess });
  await instance.check(); await instance.download();
  const first = await Promise.allSettled([instance.install(), instance.install()]);
  assert.ok(first.every(result => result.status === 'rejected')); assert.equal(calls, 1); assert.equal(instance.snapshot().status, 'error');
  assert.equal((await instance.check()).status, 'ready');
  const second = await Promise.all([instance.install(), instance.install()]);
  assert.ok(second.every(result => result.status === 'installing')); assert.equal(calls, 2);
});
test('previous helper error is shown on startup and explicit next check can retry staging', async t => {
  const root = scratch(t), fixture = releaseFixture(), server = await fixtureServer(t, fixture);
  const first = updater(root, server.fetch); await first.check(); await first.download();
  fs.writeFileSync(path.join(first.root, 'last-result.json'), JSON.stringify({ status: 'error', version: '1.1.0', error: 'File is locked.' }));
  const second = updater(root, server.fetch);
  assert.equal((await second.check()).status, 'error'); assert.match(second.snapshot().error, /File is locked/);
  assert.equal((await second.check()).status, 'ready');
  assert.equal(server.requests.filter(url => url.endsWith('-Update.zip')).length, 1);
});

async function helperFixture(t, failAfterFirst) {
  const root = scratch(t), stage = path.join(root, 'data', 'updates', 'test'), payload = path.join(stage, 'payload');
  fs.mkdirSync(payload, { recursive: true });
  const contents = [['resources/app.asar', 'new-app'], ['bundle/game/atlas-client.exe', 'new-game']];
  const files = [];
  for (const [name, content] of contents) {
    const source = path.join(payload, name); fs.mkdirSync(path.dirname(source), { recursive: true }); fs.writeFileSync(source, content);
    files.push({ path: name, relative: name.startsWith('bundle/game/') ? name.slice('bundle/game/'.length) : name, kind: name.startsWith('bundle/game/') ? 'bundle' : 'app', size: content.length, sha256: await hashFile(source) });
  }
  fs.mkdirSync(path.join(root, 'resources')); fs.writeFileSync(path.join(root, 'resources', 'app.asar'), 'old-app');
  fs.mkdirSync(path.join(root, 'bundle', 'game'), { recursive: true });
  if (failAfterFirst) fs.mkdirSync(path.join(root, 'bundle', 'game', 'atlas-client.exe'));
  else fs.writeFileSync(path.join(root, 'bundle', 'game', 'atlas-client.exe'), 'old-game');
  for (const name of ['data/worlds/MyWorld/level.dat', 'data/accounts.vault', 'runtime/.local/atlas-client/26.3/facts.json']) { const target = path.join(root, name); fs.mkdirSync(path.dirname(target), { recursive: true }); fs.writeFileSync(target, 'private untouched'); }
  const plan = { schema: 1, parentPid: 2147483647, projectRoot: root, appRoot: root, executable: path.join(root, 'Atlas Launcher.exe'), version: '1.1.0', payload, backup: path.join(stage, 'backup'), result: path.join(root, 'data', 'updates', 'last-result.json'), restart: false, files };
  const planPath = path.join(stage, 'install-plan.json'); fs.writeFileSync(planPath, JSON.stringify(plan));
  let exitCode = 0;
  try { execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', path.resolve(__dirname, '../lib/install-update.ps1'), '-PlanPath', planPath], { windowsHide: true, timeout: 30000, stdio: 'pipe' }); }
  catch (error) { exitCode = error.status; }
  const result = JSON.parse(fs.readFileSync(plan.result, 'utf8').replace(/^\uFEFF/, ''));
  assert.equal(exitCode, failAfterFirst ? 1 : 0, result.error); assert.equal(result.status, failAfterFirst ? 'error' : 'installed', result.error);
  if (failAfterFirst) assert.match(result.error, /folder occupies/);
  assert.equal(fs.readFileSync(path.join(root, 'resources', 'app.asar'), 'utf8'), failAfterFirst ? 'old-app' : 'new-app');
  for (const name of ['data/worlds/MyWorld/level.dat', 'data/accounts.vault', 'runtime/.local/atlas-client/26.3/facts.json']) assert.equal(fs.readFileSync(path.join(root, name), 'utf8'), 'private untouched');
}
test('Windows helper updates actual app/game files and preserves worlds, credentials and prepared resources', { skip: process.platform !== 'win32' }, async t => helperFixture(t, false));
test('Windows helper rolls back replaced app files when a later replacement fails', { skip: process.platform !== 'win32' }, async t => helperFixture(t, true));

test('production Windows installer runs after its launcher process exits', { skip: process.platform !== 'win32' }, async t => {
  const root = scratch(t), stage = path.join(root, 'data', 'updates', 'exit-test'), payload = path.join(stage, 'payload');
  fs.mkdirSync(path.join(payload, 'resources'), { recursive: true });
  fs.mkdirSync(path.join(root, 'resources'), { recursive: true });
  fs.writeFileSync(path.join(root, 'resources/app.asar'), 'old launcher');
  fs.writeFileSync(path.join(payload, 'resources/app.asar'), 'new launcher');
  const file = { path: 'resources/app.asar', size: 12, sha256: await hashFile(path.join(payload, 'resources/app.asar')) };
  const input = path.join(stage, 'fixture.json');
  fs.writeFileSync(input, JSON.stringify({ root, stage, payload, file }));
  const worker = `
    const fs = require('node:fs'), path = require('node:path');
    const { ReleaseUpdater } = require(process.argv[1]);
    const fixture = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
    const updater = new ReleaseUpdater({ projectRoot: fixture.root, dataRoot: path.join(fixture.root, 'data'), executable: path.join(fixture.root, 'Atlas Launcher.exe'), currentVersion: '1.0.0', enabled: true });
    updater.info = { version: '1.1.0', files: [fixture.file] };
    updater.staged = fixture.payload; updater.stageDirectory = fixture.stage;
    updater.update({ status: 'ready' });
    updater.install({ restart: false }).catch(error => { console.error(error); process.exitCode = 1; });
  `;
  execFileSync(process.execPath, ['-e', worker, path.resolve(__dirname, '../lib/release-updater.cjs'), input], { windowsHide: true, timeout: 10000, stdio: 'pipe' });
  const receipt = path.join(root, 'data/updates/last-result.json');
  for (let i = 0; i < 200 && !fs.existsSync(receipt); i++) await new Promise(resolve => setTimeout(resolve, 100));
  assert.ok(fs.existsSync(receipt), 'Installer must execute after the launcher exits, rather than silently exiting without running its script.');
  const result = JSON.parse(fs.readFileSync(receipt, 'utf8').replace(/^\uFEFF/, ''));
  assert.equal(result.status, 'installed', result.error);
  assert.equal(fs.readFileSync(path.join(root, 'resources/app.asar'), 'utf8'), 'new launcher');
});

test('release builder keeps verified native sources outside the main update', { skip: process.platform !== 'win32' }, async t => {
  const root = scratch(t), portable = path.join(root, 'portable'), output = path.join(root, 'release');
  const fixture = releaseFixture();
  const asarInput = path.join(root, 'asar-input');
  const sourceName = 'Atlas-1.1.0-Native-Source.zip';
  const sourcePath = path.join(root, sourceName);
  const sourceArchive = zip([['Cargo.toml', '[workspace]']]);
  const descriptor = { schema: 1, version: '1.1.0', sourceKey: '1'.repeat(64), revision: '2'.repeat(40), size: sourceArchive.length, sha256: digest(sourceArchive) };
  fs.mkdirSync(asarInput);
  fs.writeFileSync(path.join(asarInput, 'native-source.json'), JSON.stringify(descriptor));
  const { createPackage, extractFile } = require('@electron/asar');
  for (const [name, content] of fixture.contents) {
    const file = path.join(portable, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    if (name === 'resources/app.asar') await createPackage(asarInput, file);
    else fs.writeFileSync(file, content);
  }
  const { buildRelease } = require('../scripts/build-release.cjs');
  await assert.rejects(buildRelease(portable, output, '1.1.0'), /Missing Atlas-1\.1\.0-Native-Source\.zip/);
  fs.writeFileSync(sourcePath, Buffer.alloc(sourceArchive.length));
  await assert.rejects(buildRelease(portable, output, '1.1.0'), /does not match the launcher descriptor/);
  fs.writeFileSync(sourcePath, sourceArchive);
  const built = await buildRelease(portable, output, '1.1.0');
  const release = { ...fixture.release, assets: [{ name: built.manifest.archive.name, size: built.manifest.archive.size, browser_download_url: `https://github.com/${REPOSITORY}/releases/download/atlas-1.1.0/${built.manifest.archive.name}` }] };
  validateManifest(built.manifest, release);
  assert.deepEqual(fs.readFileSync(path.join(output, sourceName)), sourceArchive);
  assert.equal(built.manifest.files.some(file => file.path.endsWith(sourceName)), false);
  await extractArchive(built.archive, path.join(root, 'extracted'), built.manifest.files);
  for (const [name, content] of fixture.contents) {
    const file = path.join(root, 'extracted', name);
    if (name === 'resources/app.asar') assert.deepEqual(JSON.parse(extractFile(file, 'native-source.json').toString('utf8')), descriptor);
    else assert.equal(fs.readFileSync(file, 'utf8'), content);
  }
});
