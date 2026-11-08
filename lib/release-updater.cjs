const appFs = require('node:fs');
// Update payloads contain app.asar as an ordinary file. Electron's virtual
// filesystem rejects that destination before it exists, and can cache its header.
const fs = process.versions.electron ? require('original-fs') : appFs;
const path = require('node:path');
const crypto = require('node:crypto');
const zlib = require('node:zlib');
const { Transform } = require('node:stream');
const { pipeline } = require('node:stream/promises');
const { spawn } = require('node:child_process');

const REPOSITORY = 'SFrostStar/Atlas';
const API_URL = `https://api.github.com/repos/${REPOSITORY}/releases/latest`;
const HOSTS = new Set(['api.github.com', 'github.com', 'release-assets.githubusercontent.com', 'objects.githubusercontent.com']);
const LIMITS = { json: 4 * 1024 * 1024, archive: 1024 * 1024 * 1024, file: 512 * 1024 * 1024, total: 2 * 1024 * 1024 * 1024, files: 12000 };
const APP_FILES = new Set(['Atlas Launcher.exe', 'chrome_100_percent.pak', 'chrome_200_percent.pak', 'd3dcompiler_47.dll', 'dxcompiler.dll', 'dxil.dll', 'electron_wer.dll', 'ffmpeg.dll', 'icudtl.dat', 'LICENSE', 'LICENSES.chromium.html', 'resources.pak', 'snapshot_blob.bin', 'v8_context_snapshot.bin', 'version', 'vk_swiftshader_icd.json', 'vk_swiftshader.dll', 'vulkan-1.dll', 'resources/app.asar', 'atlas-portable.json', 'START-HERE.txt']);
const GAME_FILES = new Set(['atlas-client.exe', 'atlas-ctl.exe', 'atlas-prepare.exe', 'atlas-server.exe', 'libc++.dll', 'libunwind.dll', 'libwinpthread-1.dll', 'atlas-runtime.json']);
const REQUIRED = ['Atlas Launcher.exe', 'resources/app.asar', 'atlas-portable.json', 'bundle/game/atlas-client.exe', 'bundle/game/atlas-ctl.exe', 'bundle/game/atlas-prepare.exe', 'bundle/game/libc++.dll', 'bundle/game/libunwind.dll', 'bundle/game/libwinpthread-1.dll', 'bundle/game/atlas-runtime.json', 'bundle/game/game-root/research-lock.json', 'bundle/game/game-root/exporters/manifest.json'];

function version(value) {
  const match = /^(?:atlas-|v)?(0|[1-9]\d*)\.(0|[1-9]\d*)(?:\.(0|[1-9]\d*))?$/.exec(String(value || ''));
  if (!match) throw new Error('Invalid Atlas release version.');
  const parts = match.slice(1).map(n => Number(n || 0));
  if (parts.some(n => !Number.isSafeInteger(n))) throw new Error('Invalid Atlas release version.');
  return parts.join('.');
}
function newer(a, b) {
  const left = version(a).split('.').map(Number), right = version(b).split('.').map(Number);
  for (let i = 0; i < 3; i++) if (left[i] !== right[i]) return left[i] > right[i];
  return false;
}
function safeRelative(value) {
  if (typeof value !== 'string' || !value || value.includes('\\') || /[:\x00-\x1f]/.test(value) || value.split('/').some(n => !n || n === '.' || n === '..' || /[. ]$/.test(n) || /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(n))) throw new Error('Unsafe update file path.');
  return value;
}
function allowedFile(value) {
  safeRelative(value);
  if (APP_FILES.has(value) || /^locales\/[a-zA-Z0-9_-]+\.pak$/.test(value)) return true;
  if (!value.startsWith('bundle/game/')) return false;
  const game = value.slice('bundle/game/'.length);
  return GAME_FILES.has(game) || /^LICENSE[^/]*\.txt$/.test(game) ||
    ['game-root/research-lock.json', 'game-root/ATLAS-UPSTREAM.json', 'game-root/crates/atlas-prepare/src/exporters.rs'].includes(game) ||
    /^game-root\/exporters\/(?:manifest\.json|(?:client|server)\/dev\/atlaslab\/parity\/[A-Za-z0-9_$]+\.class)$/.test(game) ||
    /^game-root\/mods\/atlas\/src\/test\/java\/dev\/atlaslab\/parity\/[A-Za-z0-9_$]+\.java$/.test(game);
}
function guard(root, target) {
  root = path.resolve(root); target = path.resolve(target);
  const relative = path.relative(root, target);
  if (path.isAbsolute(relative) || relative === '..' || relative.startsWith(`..${path.sep}`)) throw new Error('Update path escapes its folder.');
  let current = path.parse(target).root;
  for (const part of target.slice(current.length).split(path.sep).filter(Boolean)) {
    current = path.join(current, part);
    try { if (fs.lstatSync(current).isSymbolicLink()) throw new Error('Linked update folders are not allowed.'); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
  }
  return target;
}
function safeUrl(value, initial = false) {
  const url = new URL(value);
  if (url.protocol !== 'https:' || url.username || url.password || (url.port && url.port !== '443') || !HOSTS.has(url.hostname)) throw new Error('Untrusted Atlas update address.');
  if (initial && !(url.hostname === 'github.com' && url.pathname.startsWith(`/${REPOSITORY}/releases/download/`))) throw new Error('Update asset does not belong to Atlas.');
  return url.href;
}
async function hashFile(file) {
  const hash = crypto.createHash('sha256');
  for await (const bytes of fs.createReadStream(file)) hash.update(bytes);
  return hash.digest('hex');
}
function validateManifest(input, release) {
  const releaseVersion = version(release.tag_name);
  if (!input || input.schema !== 1 || input.repository !== REPOSITORY || input.platform !== 'win32-x64' || input.version !== releaseVersion || input.tag !== release.tag_name || !/^atlas-\d+\.\d+\.\d+$/.test(input.tag)) throw new Error('Update manifest does not match the Atlas release.');
  const asset = release.assets.find(a => a.name === input.archive?.name);
  if (!asset || !/^Atlas-\d+\.\d+\.\d+-Update\.zip$/.test(asset.name) || !/^[a-f0-9]{64}$/.test(input.archive.sha256 || '') || !Number.isSafeInteger(input.archive.size) || input.archive.size < 22 || input.archive.size > LIMITS.archive || asset.size !== input.archive.size) throw new Error('Invalid Atlas update archive.');
  const archive = { ...input.archive, url: safeUrl(asset.browser_download_url, true) };
  if (!Array.isArray(input.files) || !input.files.length || input.files.length > LIMITS.files) throw new Error('Invalid update file inventory.');
  const names = new Set(); let total = 0;
  const files = input.files.map(file => {
    if (!file || !allowedFile(file.path) || !Number.isSafeInteger(file.size) || file.size < 0 || file.size > LIMITS.file || !/^[a-f0-9]{64}$/.test(file.sha256 || '')) throw new Error('Invalid update file inventory.');
    const key = file.path.toLowerCase();
    if (names.has(key)) throw new Error('Duplicate update file path.');
    names.add(key); total += file.size;
    return { path: file.path, size: file.size, sha256: file.sha256 };
  });
  if (total > LIMITS.total || REQUIRED.some(name => !names.has(name.toLowerCase()))) throw new Error('Incomplete or excessive update payload.');
  return { schema: 1, repository: REPOSITORY, platform: 'win32-x64', version: releaseVersion, tag: input.tag, archive, files };
}

// Read the central directory before extraction. Every entry must match the HTTPS-authenticated
// release inventory; no filenames are ever passed to an external unzip program.
async function zipEntries(archive, expected) {
  const handle = await fs.promises.open(archive, 'r');
  try {
    const size = (await handle.stat()).size;
    const tail = Buffer.alloc(Math.min(size, 65557));
    await handle.read(tail, 0, tail.length, size - tail.length);
    let end = -1;
    for (let i = tail.length - 22; i >= 0; i--) if (tail.readUInt32LE(i) === 0x06054b50 && i + 22 + tail.readUInt16LE(i + 20) === tail.length) { end = i; break; }
    if (end < 0) throw new Error('Invalid update ZIP directory.');
    const count = tail.readUInt16LE(end + 10), directorySize = tail.readUInt32LE(end + 12), offset = tail.readUInt32LE(end + 16);
    if (tail.readUInt16LE(end + 4) || tail.readUInt16LE(end + 6) || count !== tail.readUInt16LE(end + 8) || count === 65535 || count > LIMITS.files || directorySize > LIMITS.json || offset + directorySize > size - 22) throw new Error('Unsupported update ZIP directory.');
    const directory = Buffer.alloc(directorySize);
    await handle.read(directory, 0, directorySize, offset);
    const inventory = new Map(expected.map(f => [f.path, f]));
    const entries = []; const names = new Set(); let cursor = 0;
    for (let i = 0; i < count; i++) {
      if (cursor + 46 > directory.length || directory.readUInt32LE(cursor) !== 0x02014b50) throw new Error('Invalid update ZIP entry.');
      const flags = directory.readUInt16LE(cursor + 8), method = directory.readUInt16LE(cursor + 10), compressed = directory.readUInt32LE(cursor + 20), uncompressed = directory.readUInt32LE(cursor + 24), nameLength = directory.readUInt16LE(cursor + 28), extra = directory.readUInt16LE(cursor + 30), comment = directory.readUInt16LE(cursor + 32), attributes = directory.readUInt32LE(cursor + 38), localOffset = directory.readUInt32LE(cursor + 42);
      if (cursor + 46 + nameLength + extra + comment > directory.length) throw new Error('Invalid update ZIP entry.');
      const name = directory.toString('utf8', cursor + 46, cursor + 46 + nameLength);
      cursor += 46 + nameLength + extra + comment;
      const file = inventory.get(name);
      if (!file || !allowedFile(name) || names.has(name.toLowerCase()) || flags & 1 || ![0, 8].includes(method) || (attributes >>> 16 & 0xf000) === 0xa000 || uncompressed !== file.size || compressed > LIMITS.archive || localOffset + 30 > offset) throw new Error('Update ZIP does not match its file inventory.');
      names.add(name.toLowerCase());
      const local = Buffer.alloc(30); await handle.read(local, 0, 30, localOffset);
      if (local.readUInt32LE(0) !== 0x04034b50 || local.readUInt16LE(8) !== method || local.readUInt16LE(6) !== flags) throw new Error('Inconsistent update ZIP entry.');
      const localName = Buffer.alloc(local.readUInt16LE(26)); await handle.read(localName, 0, localName.length, localOffset + 30);
      const dataOffset = localOffset + 30 + localName.length + local.readUInt16LE(28);
      if (localName.toString('utf8') !== name || dataOffset + compressed > offset) throw new Error('Unsafe update ZIP data.');
      entries.push({ ...file, method, compressed, offset: dataOffset });
    }
    if (cursor !== directory.length || entries.length !== expected.length) throw new Error('Update ZIP inventory is incomplete.');
    return entries;
  } finally { await handle.close(); }
}
async function extractArchive(archive, destination, files) {
  const entries = await zipEntries(archive, files);
  guard(destination, destination); await fs.promises.mkdir(destination, { recursive: true });
  for (const entry of entries) {
    const target = guard(destination, path.join(destination, entry.path));
    await fs.promises.mkdir(path.dirname(target), { recursive: true });
    let bytes = 0; const hash = crypto.createHash('sha256');
    const verify = new Transform({ transform(chunk, encoding, callback) {
      bytes += chunk.length;
      if (bytes > entry.size) return callback(new Error('Update ZIP file exceeds its declared size.'));
      hash.update(chunk); callback(null, chunk);
    } });
    const output = fs.createWriteStream(target, { flags: 'wx' });
    if (entry.compressed === 0) {
      output.end(); await new Promise((resolve, reject) => { output.once('finish', resolve); output.once('error', reject); });
    } else {
      const input = fs.createReadStream(archive, { start: entry.offset, end: entry.offset + entry.compressed - 1 });
      await pipeline(...(entry.method === 8 ? [input, zlib.createInflateRaw(), verify, output] : [input, verify, output]));
    }
    if (bytes !== entry.size || hash.digest('hex') !== entry.sha256) throw new Error('Update file checksum mismatch.');
  }
}

class ReleaseUpdater {
  constructor({ projectRoot, dataRoot, executable = process.execPath, currentVersion, onChange = () => {}, isGameRunning = () => false, fetch = globalThis.fetch, spawnProcess = spawn, enabled = process.platform === 'win32', requestTimeoutMs = 300000 } = {}) {
    if (!path.isAbsolute(projectRoot || '') || !path.isAbsolute(dataRoot || '') || !path.isAbsolute(executable || '')) throw new Error('Update folders must be absolute.');
    this.projectRoot = path.resolve(projectRoot); this.dataRoot = path.resolve(dataRoot); this.executable = path.resolve(executable);
    this.root = path.join(this.dataRoot, 'updates'); this.currentVersion = version(currentVersion); this.onChange = onChange; this.isGameRunning = isGameRunning;
    this.fetch = fetch; this.spawn = spawnProcess; this.enabled = enabled; this.requestTimeoutMs = requestTimeoutMs;
    this.state = { status: enabled ? 'idle' : 'disabled', currentVersion: this.currentVersion, version: null, progress: null, bytesDone: 0, bytesTotal: 0, error: null, releaseUrl: null };
    try {
      const result = JSON.parse(fs.readFileSync(guard(this.root, path.join(this.root, 'last-result.json')), 'utf8').replace(/^\uFEFF/, ''));
      if (['installed', 'error'].includes(result.status) && typeof result.version === 'string') {
        version(result.version);
        this.lastInstall = { status: result.status, version: result.version, error: typeof result.error === 'string' ? result.error.slice(0, 2000) : null };
        this.previousInstallError = result.status === 'error' && newer(result.version, this.currentVersion);
      }
    } catch { /* No installation has been attempted yet. */ }
  }
  snapshot() { return { ...this.state, lastInstall: this.lastInstall ? { ...this.lastInstall } : null }; }
  update(patch) { Object.assign(this.state, patch); this.onChange(this.snapshot()); return this.snapshot(); }
  async response(url, signal) {
    for (let redirects = 0; redirects < 6; redirects++) {
      const response = await this.fetch(safeUrl(url), { signal, redirect: 'manual', headers: { 'User-Agent': `Atlas/${this.currentVersion}`, 'Accept': 'application/vnd.github+json' } });
      if ([301,302,303,307,308].includes(response.status)) {
        const location = response.headers.get('location'); await response.body?.cancel();
        if (!location) throw new Error('Atlas update redirect has no address.');
        url = safeUrl(new URL(location, url).href); continue;
      }
      if (!response.ok) { await response.body?.cancel(); throw new Error(`Atlas update request failed (HTTP ${response.status}).`); }
      return response;
    }
    throw new Error('Too many Atlas update redirects.');
  }
  async json(url) {
    const controller = new AbortController(), timer = setTimeout(() => controller.abort(), Math.min(this.requestTimeoutMs, 30000));
    try {
      const response = await this.response(url, controller.signal); const chunks = []; let size = 0;
      for await (const chunk of response.body) { size += chunk.length; if (size > LIMITS.json) { controller.abort(); throw new Error('Atlas update response is too large.'); } chunks.push(chunk); }
      return JSON.parse(Buffer.concat(chunks).toString('utf8'));
    } finally { clearTimeout(timer); }
  }
  async check() {
    if (!this.enabled || this.pending || this.installPending || this.state.status === 'installing') return this.snapshot();
    if (this.previousInstallError) {
      this.previousInstallError = false;
      return this.update({ status: 'error', version: this.lastInstall.version, error: `Previous Atlas update failed: ${this.lastInstall.error || 'Unknown installation error.'} Check for updates to retry.`, progress: null });
    }
    this.pending = this.checkRelease().catch(error => this.update({ status: 'error', error: error.message, progress: null })).finally(() => { this.pending = null; });
    return this.pending;
  }
  async checkRelease() {
    const previous = this.state.status;
    this.update({ status: 'checking', error: null });
    const release = await this.json(API_URL);
    if (release.draft || release.prerelease || !/^atlas-\d+\.\d+\.\d+$/.test(release.tag_name || '')) return this.update({ status: 'upToDate', progress: null });
    if (!newer(release.tag_name, this.currentVersion)) return this.update({ status: 'upToDate', version: this.currentVersion, progress: null });
    if (!Array.isArray(release.assets)) throw new Error('Atlas release has no update assets.');
    const manifest = release.assets.find(asset => asset.name === 'atlas-update.json');
    if (!manifest || manifest.size > LIMITS.json) throw new Error('Atlas release has no update manifest.');
    const info = validateManifest(await this.json(safeUrl(manifest.browser_download_url, true)), release);
    const same = this.info?.tag === info.tag && this.info.archive.sha256 === info.archive.sha256;
    this.info = info;
    const releaseUrl = `https://github.com/${REPOSITORY}/releases/tag/${encodeURIComponent(info.tag)}`;
    if (same && previous === 'ready' && await this.stagedValid()) return this.update({ status: 'ready', version: info.version, releaseUrl, progress: 1 });
    // A previous launcher session may have completed this download already.
    try {
      const receipt = JSON.parse(await fs.promises.readFile(guard(this.root, path.join(this.root, 'ready.json')), 'utf8'));
      const directory = guard(this.root, path.join(this.root, receipt.directory));
      if (receipt.tag === info.tag && receipt.sha256 === info.archive.sha256) {
        this.staged = path.join(directory, 'payload'); this.stageDirectory = directory;
        if (await this.stagedValid()) return this.update({ status: 'ready', version: info.version, releaseUrl, progress: 1 });
      }
    } catch { /* Missing or outdated staging is downloaded again. */ }
    this.staged = null;
    return this.update({ status: 'available', version: info.version, releaseUrl, progress: null });
  }
  async stagedValid() {
    if (!this.staged || !this.info) return false;
    try {
      for (const file of this.info.files) {
        const target = guard(this.staged, path.join(this.staged, file.path));
        if (!fs.statSync(target).isFile() || fs.statSync(target).size !== file.size || await hashFile(target) !== file.sha256) return false;
      }
      return true;
    } catch { return false; }
  }
  async download() {
    if (!this.enabled || this.pending || this.installPending || !this.info || this.state.status === 'installing' || this.state.status === 'ready') return this.snapshot();
    this.pending = this.downloadRelease().catch(error => this.update({ status: 'error', error: error.message, progress: null })).finally(() => { this.pending = null; });
    return this.pending;
  }
  async downloadRelease() {
    guard(this.root, this.root); await fs.promises.mkdir(this.root, { recursive: true });
    this.stageDirectory = await fs.promises.mkdtemp(path.join(this.root, `${this.info.tag}-`));
    this.staged = path.join(this.stageDirectory, 'payload');
    const temporary = path.join(this.stageDirectory, 'update.zip.part'), archive = path.join(this.stageDirectory, 'update.zip');
    const controller = new AbortController(), timer = setTimeout(() => controller.abort(), this.requestTimeoutMs);
    this.update({ status: 'downloading', progress: 0, bytesDone: 0, bytesTotal: this.info.archive.size, error: null });
    let handle;
    try {
      const response = await this.response(this.info.archive.url, controller.signal); const hash = crypto.createHash('sha256'); let bytes = 0, last = 0;
      handle = await fs.promises.open(temporary, 'wx');
      for await (const chunk of response.body) {
        bytes += chunk.length;
        if (bytes > this.info.archive.size) { controller.abort(); throw new Error('Atlas update exceeds its declared size.'); }
        hash.update(chunk); await handle.writeFile(chunk);
        if (Date.now() - last > 150) { this.update({ bytesDone: bytes, progress: bytes / this.info.archive.size }); last = Date.now(); }
      }
      await handle.close(); handle = null;
      if (bytes !== this.info.archive.size || hash.digest('hex') !== this.info.archive.sha256) throw new Error('Atlas update archive checksum mismatch.');
      await fs.promises.rename(temporary, archive);
      await extractArchive(archive, this.staged, this.info.files);
      const receipt = { tag: this.info.tag, sha256: this.info.archive.sha256, directory: path.basename(this.stageDirectory) };
      const receiptTmp = path.join(this.root, `ready-${crypto.randomUUID()}.tmp`);
      await fs.promises.writeFile(receiptTmp, JSON.stringify(receipt), { flag: 'wx' });
      await fs.promises.rename(receiptTmp, guard(this.root, path.join(this.root, 'ready.json')));
      return this.update({ status: 'ready', progress: 1, bytesDone: bytes });
    } finally { clearTimeout(timer); await handle?.close().catch(() => {}); await fs.promises.unlink(temporary).catch(() => {}); }
  }
  async install({ restart = true } = {}) {
    if (this.installPending) return this.installPending;
    this.installPending = this.installRelease({ restart }).catch(error => {
      this.update({ status: 'error', error: error.message });
      throw error;
    }).finally(() => { this.installPending = null; });
    return this.installPending;
  }
  async installRelease({ restart }) {
    if (!this.enabled || this.pending || this.state.status !== 'ready') throw new Error('Atlas update is not ready.');
    if (this.isGameRunning()) throw new Error('Close Minecraft before installing the Atlas update.');
    if (!await this.stagedValid()) { this.update({ status: 'error', error: 'Staged Atlas update failed verification.' }); throw new Error(this.state.error); }
    const appRoot = path.dirname(this.executable), files = [];
    for (const file of this.info.files) {
      if (file.path.startsWith('bundle/game/')) {
        const relative = file.path.slice('bundle/game/'.length);
        guard(path.join(this.projectRoot, 'bundle/game'), path.join(this.projectRoot, 'bundle/game', relative));
        files.push({ ...file, kind: 'bundle', relative });
        if (GAME_FILES.has(relative) || /^LICENSE[^/]*\.txt$/.test(relative)) {
          guard(path.join(this.projectRoot, 'runtime'), path.join(this.projectRoot, 'runtime', relative));
          files.push({ ...file, kind: 'runtime', relative });
        }
      } else {
        // Adding a portable marker inside dist/<app> changes portable-root's data
        // resolution on the next start. Existing checkout installations keep it.
        if (appRoot !== this.projectRoot && ['atlas-portable.json', 'START-HERE.txt'].includes(file.path)) continue;
        guard(appRoot, path.join(appRoot, file.path));
        files.push({ ...file, kind: 'app', relative: file.path });
      }
    }
    const attempt = crypto.randomUUID();
    const started = path.join(this.stageDirectory, `helper-started-${attempt}.json`);
    const plan = { schema: 1, parentPid: process.pid, projectRoot: this.projectRoot, appRoot, executable: this.executable, version: this.info.version, payload: this.staged, backup: path.join(this.stageDirectory, 'backup'), result: path.join(this.root, 'last-result.json'), started, restart: !!restart, files };
    const planPath = guard(this.root, path.join(this.stageDirectory, `install-plan-${attempt}.json`));
    const helper = path.join(this.stageDirectory, `install-update-${attempt}.ps1`);
    await fs.promises.writeFile(planPath, JSON.stringify(plan), { flag: 'wx' });
    // readFile works for scripts stored inside Electron's app.asar too.
    await fs.promises.writeFile(helper, await appFs.promises.readFile(path.join(__dirname, 'install-update.ps1')), { flag: 'wx' });
    // Windows PowerShell can exit successfully without running -File when started
    // with DETACHED_PROCESS. Hidden stdio plus unref still lets it outlive Atlas.
    const child = this.spawn('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', helper, '-PlanPath', planPath, '-Bootstrap'], { detached: false, windowsHide: true, stdio: 'ignore' });
    // Wait for CreateProcess confirmation so a spawn failure never causes the launcher to quit.
    await new Promise((resolve, reject) => { child.once('spawn', resolve); child.once('error', reject); });
    // PowerShell also needs to finish initializing before its parent exits.
    // A successful CreateProcess alone does not mean the script actually started.
    let failed = false;
    child.once('exit', code => { failed = code !== 0; });
    const deadline = Date.now() + 15000;
    let confirmed = false;
    while (Date.now() < deadline) {
      try {
        const receipt = JSON.parse(await fs.promises.readFile(guard(this.root, started), 'utf8'));
        if (receipt.version === plan.version && receipt.parentPid === process.pid) { confirmed = true; break; }
      } catch (error) { if (error.code !== 'ENOENT' && !(error instanceof SyntaxError)) throw error; }
      if (failed) throw new Error('Atlas update installer exited before starting. The launcher stayed open.');
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    if (!confirmed) { child.kill?.(); throw new Error('Atlas update installer did not confirm startup. The launcher stayed open.'); }
    child.unref();
    return this.update({ status: 'installing', error: null });
  }
}

module.exports = { ReleaseUpdater, version, newer, safeUrl, safeRelative, allowedFile, validateManifest, zipEntries, extractArchive, hashFile, guard, REQUIRED, GAME_FILES, LIMITS, REPOSITORY };
