const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { createHash } = require('node:crypto');
const { hashFile, zipEntries } = require('../lib/release-updater.cjs');
const { SOURCE_LIMITS } = require('../lib/native-source.cjs');

// Package the actual engine input, including local source edits, without build outputs.
async function copyNativeSource(project, destination) {
  const inventory = execFileSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard', '--', 'engine/atlas'], {
    cwd: project, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024,
  });
  const files = [...new Set(inventory.split('\0').filter(Boolean))].sort();
  const digest = createHash('sha256');
  for (const file of files) {
    const relative = file.slice('engine/atlas/'.length);
    if (!relative || relative.split('/').some(part => part === '..') ||
        /(?:^|\/)(?:target|\.local|saves|auth-sessions|logs|\.git)(?:\/|$)/i.test(relative) ||
        /(?:^|\/)(?:accounts|session|tokens|credentials|launcher)\.json$/i.test(relative) ||
        /\.(?:jar|class|exe|dll|log|mca|mcr)$/i.test(relative)) continue;
    const source = path.join(project, file);
    const entry = await fs.lstat(source).catch(error => {
      if (error.code !== 'ENOENT') throw error;
      return null;
    });
    if (!entry?.isFile() || await fs.realpath(source) !== source) continue;
    const target = path.join(destination, relative);
    const bytes = await fs.readFile(source);
    digest.update(relative + '\0').update(createHash('sha256').update(bytes).digest());
    await fs.mkdir(path.dirname(target), { recursive: true });
    await fs.writeFile(target, bytes);
  }
  for (const name of ['Cargo.toml', 'Cargo.lock', 'ATLAS-UPSTREAM.json']) await fs.access(path.join(destination, name));
  await fs.writeFile(path.join(destination, '.atlas-source-key'), digest.digest('hex') + '\n');
}

async function archiveNativeSource(project, archive, version) {
  const temp = await fs.realpath(os.tmpdir());
  const source = await fs.mkdtemp(path.join(temp, 'atlas-native-source-'));
  try {
    await copyNativeSource(project, source);
    const sourceKey = (await fs.readFile(path.join(source, '.atlas-source-key'), 'utf8')).trim();
    const { commit: revision } = JSON.parse(await fs.readFile(path.join(source, 'ATLAS-UPSTREAM.json'), 'utf8'));
    if (!/^[a-f0-9]{40}$/.test(revision || '')) throw new Error('Invalid Atlas source revision.');
    execFileSync('powershell.exe', [
      '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', path.join(__dirname, 'zip-release.ps1'),
      '-SourceDirectory', source, '-ArchivePath', archive,
    ], { windowsHide: true, stdio: 'inherit' });
    const size = (await fs.stat(archive)).size;
    if (size > SOURCE_LIMITS.archive) throw new Error('Atlas native source archive exceeds the download limit.');
    await zipEntries(archive, null, SOURCE_LIMITS);
    return { schema: 1, version, sourceKey, revision, size, sha256: await hashFile(archive) };
  } finally {
    if (path.dirname(source) === temp) await fs.rm(source, { recursive: true, force: true });
  }
}

module.exports = { copyNativeSource, archiveNativeSource };
if (require.main === module) copyNativeSource(path.join(__dirname, '..'), path.resolve(process.argv[2])).catch(error => {
  console.error(error.message);
  process.exitCode = 1;
});
