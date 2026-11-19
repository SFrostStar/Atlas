const fs = require('node:fs');
const fsp = fs.promises;
const path = require('node:path');
const { createHash, randomUUID } = require('node:crypto');
const { inflateRawSync } = require('node:zlib');
const { spawn } = require('node:child_process');
const { Readable, Transform } = require('node:stream');
const { pipeline } = require('node:stream/promises');
const { guard, hashFile, zipEntries } = require('./release-updater.cjs');
const { syncNativeStage } = require('./native-stage.cjs');

const MIXINS_REV = '5a64af938ec86b00d53d16a408aabc0038fb3e57';
const LLVM = 'llvm-mingw-20261020-ucrt-x86_64';
const RUST = '1.99.1-x86_64-pc-windows-gnullvm';
const LIBRARIES = ['libc++.dll', 'libunwind.dll', 'libwinpthread-1.dll'];
const GAME_ICONS = ['game-icon.png', 'game-icon.ico'];
const ZIP_LIMITS = { files: 512, json: 512 * 1024, archive: 32 * 1024 * 1024, file: 16 * 1024 * 1024, total: 64 * 1024 * 1024 };
const MOD_ID = /^[a-z0-9_]{1,64}$/;

function cargoProgress(report, log) {
  let remainder = '';
  const line = text => {
    const match = /^\s*Building \[[^\]]*\]\s+(\d+)\/(\d+)/.exec(text);
    if (match) {
      const completed = Number(match[1]), total = Number(match[2]);
      if (Number.isSafeInteger(total) && total > 0 && completed <= total) report(completed, total);
    } else if (text.trim()) {
      log(text + '\n');
    }
  };
  return {
    push(chunk) {
      const lines = (remainder + chunk).split(/[\r\n]+/);
      remainder = lines.pop();
      for (const text of lines) line(text);
      if (remainder.length > 32768) {
        log(remainder);
        remainder = '';
      }
    },
    finish() {
      line(remainder);
      remainder = '';
    },
  };
}

function compileEstimate(report, startedAt, priorTailMs) {
  let initial, tailStartedAt;
  let lastCompleted = -1;
  const tailMs = Number.isFinite(priorTailMs) && priorTailMs > 0 ? priorTailMs : 120_000;
  return {
    update(completed, total, now = Date.now()) {
      if (completed <= lastCompleted) return;
      lastCompleted = completed;
      initial ??= { completed, at: now };
      const packagesLeft = Math.max(0, total - completed - 2);
      if (!packagesLeft) tailStartedAt ??= now;
      const sampled = completed - initial.completed;
      const duration = now - initial.at;
      const milliseconds = tailStartedAt !== undefined
        ? Math.max(0, tailMs - (now - tailStartedAt))
        : sampled > 0 && duration >= 3000
          ? packagesLeft * duration / sampled + tailMs
          : null;
      report({
        startedAt, completed, total,
        etaAt: milliseconds === null ? null : now + Math.min(milliseconds, 4 * 3600_000),
      });
    },
    tail(now = Date.now()) {
      return tailStartedAt === undefined ? null : Math.max(1000, now - tailStartedAt);
    },
  };
}

function readEntry(bytes, entry) {
  const content = bytes.subarray(entry.offset, entry.offset + entry.compressed);
  const value = entry.method === 8 ? inflateRawSync(content, { maxOutputLength: Math.max(1, entry.size) }) : content;
  if (value.length !== entry.size) throw new Error('Invalid native mod file size.');
  return value;
}

function run(command, args, { cwd, env, logfile, compileProgress }) {
  return new Promise((resolve, reject) => {
    const log = fs.openSync(logfile, 'a');
    let child;
    try { child = spawn(command, args, { cwd, env, windowsHide: true, stdio: ['ignore', log, compileProgress ? 'pipe' : log] }); }
    catch (error) { fs.closeSync(log); throw error; }
    if (compileProgress) {
      const output = cargoProgress(compileProgress, text => {
        try { fs.writeSync(log, text); } catch { /* A full disk must not crash the launcher. */ }
      });
      child.stderr.setEncoding('utf8');
      child.stderr.on('data', chunk => output.push(chunk));
      child.stderr.once('end', () => output.finish());
    }
    child.once('error', reject);
    child.once('close', code => {
      fs.closeSync(log);
      if (code === 0) resolve();
      else reject(new Error(`Native build failed (${path.basename(command)}, ${code}). Log: ${logfile}`));
    });
  });
}

async function download(url, target, sha256, maxBytes) {
  if (fs.existsSync(target) && await hashFile(target) === sha256) return;
  const temp = target + '.' + randomUUID();
  const response = await fetch(url, { signal: AbortSignal.timeout(10 * 60_000) });
  if (!response.ok || !response.body) throw new Error(`Native tool download failed (HTTP ${response.status}).`);
  let bytes = 0;
  try {
    await pipeline(Readable.fromWeb(response.body), new Transform({
      transform(chunk, encoding, callback) {
        bytes += chunk.length;
        callback(bytes > maxBytes ? new Error('Native tool download exceeds its size limit.') : null, chunk);
      },
    }), fs.createWriteStream(temp, { flags: 'wx' }));
    if (await hashFile(temp) !== sha256) throw new Error('Native tool checksum mismatch.');
    await fsp.rename(temp, target);
  } finally { await fsp.unlink(temp).catch(() => {}); }
}

async function toolchain(root, project, logfile) {
  const local = path.join(project, 'tools');
  const tools = fs.existsSync(path.join(local, 'cargo', 'bin', 'cargo.exe')) && fs.existsSync(path.join(local, LLVM, 'bin', 'clang.exe'))
    ? local : path.join(root, 'tools');
  const cargoHome = path.join(tools, 'cargo');
  const llvm = path.join(tools, LLVM, 'bin');
  const cargo = path.join(cargoHome, 'bin', 'cargo.exe');
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !/^(?:cargo|rust|gh_token|github_token|atlas_account|horizon_home|atlas_home)/i.test(key)));
  env.CARGO_HOME = cargoHome;
  env.RUSTUP_HOME = path.join(tools, 'rustup');
  env.PATH = [llvm, path.dirname(cargo), env.PATH || env.Path || ''].join(path.delimiter);
  delete env.Path;
  const options = { cwd: root, env, logfile };
  await fsp.mkdir(tools, { recursive: true });
  if (tools !== local) {
    const installLlvm = async () => {
      if (fs.existsSync(path.join(llvm, 'clang.exe'))) return;
      const archive = path.join(tools, LLVM + '.zip');
      await download('https://github.com/mstorsjo/llvm-mingw/releases/download/20261020/' + LLVM + '.zip',
        archive, 'fd5462b7f5d93b2ff981fcd3f9c7e427e47f3203c64ea91c27bcc444eebd74cd', 210_000_000);
      await run(path.join(process.env.SystemRoot || 'C:\\Windows', 'System32', 'tar.exe'), ['-xf', archive, '-C', tools], options);
    };
    const installRust = async () => {
      if (fs.existsSync(path.join(env.RUSTUP_HOME, 'toolchains', RUST, 'bin', 'rustc.exe'))) return;
      const rustup = path.join(tools, 'rustup-init.exe');
      await download('https://static.rust-lang.org/rustup/archive/1.29.2/x86_64-pc-windows-msvc/rustup-init.exe',
        rustup, '00f139da614dced87d3c3d4121c43eebdac3c1bf8a5ee86b9d80dfd71c8d70fd', 24_000_000);
      await run(rustup, ['-y', '--no-modify-path', '--profile', 'minimal', '--default-host', 'x86_64-pc-windows-gnullvm', '--default-toolchain', '1.99.1'], options);
    };
    const installers = await Promise.allSettled([installLlvm(), installRust()]);
    const failed = installers.find(result => result.status === 'rejected');
    if (failed) throw failed.reason;
    env.RUSTUP_TOOLCHAIN = RUST;
  }
  const mixinsDir = path.join(root, 'mixins-tool');
  const tool = path.join(mixinsDir, 'bin', 'horizon-mixins.exe');
  const developmentTool = path.join(local, 'native-mixins', 'horizon-mixins.exe');
  if (fs.existsSync(developmentTool)) return { cargo, tool: developmentTool, llvm, env };
  if (!fs.existsSync(tool)) {
    await run(cargo, ['install', '--locked', '--git', 'https://github.com/horizon-rust/horizon-mixins', '--rev', MIXINS_REV, '--root', mixinsDir, 'horizon-mixins-cli'], options);
  }
  return { cargo, tool, llvm, env };
}

class NativeMods {
  constructor({ project, dataRoot, source, version, confirm, progress = () => {}, tools = toolchain }) {
    this.root = guard(dataRoot, path.join(dataRoot, 'native-mods'));
    Object.assign(this, { project, source, version, confirm, progress, tools });
  }

  async prepare(home, excludedIds = []) {
    const files = [];
    for (const name of ['atlas-mods', 'mods']) {
      const directory = guard(home, path.join(home, name));
      const names = await fsp.readdir(directory).catch(error => {
        if (error.code === 'ENOENT') return [];
        throw error;
      });
      for (const bundle of names.filter(file => /\.hnmod$/i.test(file)).sort()) files.push({ directory, name: bundle });
    }
    if (!files.length) return null;
    if (this.pending) throw new Error('Уже идёт сборка Rust-модов.');
    if (process.platform !== 'win32' || process.arch !== 'x64') throw new Error('Rust-моды пока поддерживаются на Windows x64.');
    if (files.length > 16) throw new Error('At most 16 Rust mods can be built together.');
    this.pending = true;
    try { return await this.prepareSet(files, excludedIds); }
    finally { this.pending = false; this.progress(null); }
  }

  async prepareSet(files, excludedIds) {
    const sourceKey = (await fsp.readFile(path.join(this.source, '.atlas-source-key'), 'utf8')).trim();
    if (!/^[0-9a-f]{64}$/.test(sourceKey)) throw new Error('Invalid Atlas native source build.');
    const revision = JSON.parse(await fsp.readFile(path.join(this.source, 'ATLAS-UPSTREAM.json'), 'utf8')).commit;
    await fsp.mkdir(this.root, { recursive: true });
    const input = await fsp.mkdtemp(path.join(this.root, 'input-'));
    try {
      const bundles = [];
      const ids = new Set();
      for (const [index, { directory, name }] of files.entries()) {
        const from = guard(directory, path.join(directory, name));
        const stat = await fsp.lstat(from);
        if (!stat.isFile() || stat.size > ZIP_LIMITS.archive) throw new Error(`Invalid native mod archive: ${name}`);
        const bytes = await fsp.readFile(from);
        if (bytes.length > ZIP_LIMITS.archive) throw new Error(`Invalid native mod archive: ${name}`);
        const copy = path.join(input, String(index) + '.hnmod');
        await fsp.writeFile(copy, bytes, { flag: 'wx' });
        const entries = await zipEntries(copy, null, ZIP_LIMITS);
        const header = entries.find(entry => entry.path === 'atlas-native.json');
        if (!header || header.size > 16 * 1024) throw new Error(`Missing native mod manifest: ${name}`);
        const manifest = JSON.parse(readEntry(bytes, header).toString('utf8'));
        const { id, version, mixins, atlas, schema, game } = manifest;
        if (excludedIds.includes(id)) continue;
        if (schema !== 1 || typeof id !== 'string' || !MOD_ID.test(id) || typeof version !== 'string' || !/^[\w.+-]{1,64}$/.test(version) ||
            game !== '26.3' || atlas !== revision || mixins !== MIXINS_REV || ids.has(id)) {
          throw new Error(`Incompatible or duplicate Rust mod: ${name}`);
        }
        const prefix = `mixins/${id}/`;
        if (!entries.some(entry => entry.path === prefix + 'atlas-mixin.toml') ||
            entries.some(entry => entry !== header && !entry.path.startsWith(prefix))) {
          throw new Error(`Invalid native mod bundle: ${name}`);
        }
        ids.add(id);
        bundles.push({ id, name, version, prefix, entries, header, bytes, hash: createHash('sha256').update(bytes).digest('hex') });
      }
      if (!bundles.length) return null;
      const iconAssets = path.join(__dirname, '..', 'assets');
      const icons = await Promise.all(GAME_ICONS.map(name => fsp.readFile(path.join(iconAssets, name)).catch(error => {
        if (error.code !== 'ENOENT') throw error;
        return null;
      })));
      icons[0] ??= await fsp.readFile(path.join(iconAssets, 'icon.png'));
      const iconHashes = icons.map(bytes => bytes && createHash('sha256').update(bytes).digest('hex'));
      const key = createHash('sha256').update(JSON.stringify([this.version, sourceKey, bundles.map(bundle => bundle.hash), iconHashes])).digest('hex');
      const output = guard(this.root, path.join(this.root, 'builds', key));
      const executable = guard(output, path.join(output, 'atlas-client.exe'));
      const result = { executable, mods: ids };
      if (fs.existsSync(path.join(output, 'complete')) && fs.existsSync(executable)) return result;
      if (!await this.confirm(bundles.map(({ id, name, version }) => ({ id, name, version })))) return { cancelled: true };
      const logfile = guard(this.root, path.join(this.root, 'build.log'));
      await fsp.writeFile(logfile, '');
      this.progress('tools');
      const compiler = await this.tools(this.root, this.project, logfile);
      const options = { cwd: this.root, env: { ...compiler.env, ATLAS_VERSION: this.version }, logfile };
      const base = guard(this.root, path.join(this.root, `source-${sourceKey}`));
      if (!fs.existsSync(path.join(base, '.horizon-revision'))) {
        await fsp.cp(this.source, base, { recursive: true });
        await fsp.writeFile(path.join(base, '.horizon-revision'), revision + '\n');
      }
      const modRoot = path.join(input, 'mods');
      for (const bundle of bundles) {
        for (const entry of bundle.entries) {
          if (entry === bundle.header) continue;
          let name = entry.path.slice(bundle.prefix.length);
          let bytes = readEntry(bundle.bytes, entry);
          if (name === 'atlas-mixin.toml') {
            name = 'horizon-mixin.toml';
            bytes = bytes.toString('utf8').replace(/^(\s*)atlas_revision\s*=/m, '$1horizon_revision =')
              .replace(/^(\s*)hooks\s*=.*$/m, '');
          }
          const target = guard(modRoot, path.join(modRoot, bundle.id, name));
          await fsp.mkdir(path.dirname(target), { recursive: true });
          await fsp.writeFile(target, bytes, { flag: 'wx' });
        }
      }
      const stage = guard(this.root, path.join(this.root, 'stage'));
      const generated = path.join(input, 'stage');
      this.progress('apply');
      await run(compiler.tool, ['apply', '--workspace', base, '--mods', modRoot, '--out', generated], options);
      const iconDir = guard(generated, path.join(generated, '.local', 'atlas-icons'));
      await fsp.mkdir(iconDir, { recursive: true });
      for (const [index, name] of GAME_ICONS.entries()) {
        if (icons[index]) await fsp.writeFile(path.join(iconDir, name), icons[index]);
      }
      await syncNativeStage(generated, stage);
      const target = guard(this.root, path.join(this.root, 'target'));
      const history = guard(this.root, path.join(this.root, 'compile-estimate.json'));
      const priorTailMs = await fsp.readFile(history, 'utf8').then(Number, () => null);
      const startedAt = Date.now();
      this.progress('compile', { startedAt });
      const tracker = compileEstimate(value => this.progress('compile', value), startedAt, priorTailMs);
      const cargoEnv = { ...options.env, CARGO_TERM_PROGRESS_WHEN: 'always', CARGO_TERM_PROGRESS_WIDTH: '90',
        CARGO_TERM_PROGRESS_TERM_INTEGRATION: 'false', CARGO_TERM_COLOR: 'never', CARGO_TERM_UNICODE: 'false' };
      await run(compiler.cargo, ['build', '--manifest-path', path.join(stage, 'Cargo.toml'), '--target-dir', target, '--locked',
        '--profile', 'fast', '--config', 'profile.fast.package.atlas-client.incremental=true', '-p', 'atlas-client', '--bin', 'atlas-client'],
      { ...options, env: cargoEnv, cwd: stage, compileProgress: tracker.update });
      await fsp.mkdir(output, { recursive: true });
      for (const library of LIBRARIES) await fsp.copyFile(path.join(compiler.llvm, library), guard(output, path.join(output, library)));
      await fsp.copyFile(path.join(target, 'fast', 'atlas-client.exe'), executable);
      await fsp.writeFile(guard(output, path.join(output, 'complete')), key);
      const tail = tracker.tail();
      if (tail !== null && tail >= 10_000) await fsp.writeFile(history, String(tail)).catch(() => {});
      return result;
    } finally { await fsp.rm(guard(this.root, input), { recursive: true, force: true }); }
  }
}

module.exports = { NativeMods, toolchain, cargoProgress, compileEstimate };
