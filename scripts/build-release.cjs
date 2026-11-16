const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { allowedFile, hashFile, version, REQUIRED, REPOSITORY, LIMITS } = require('../lib/release-updater.cjs');

const USAGE = 'Usage: node scripts/build-release.cjs <flat-portable-directory> <new-output-directory> [--bootstrap]';
// Feed this only the output of Build-Portable.ps1, never a running installation.
async function buildRelease(portable, output, releaseVersion = require('../package.json').version, { bootstrap = false } = {}) {
  portable = path.resolve(portable); output = path.resolve(output); releaseVersion = version(releaseVersion);
  if (fs.existsSync(output)) throw new Error('Release output already exists; use a fresh output directory.');
  const relative = path.relative(portable, output);
  if (!path.isAbsolute(relative) && relative !== '..' && !relative.startsWith(`..${path.sep}`)) {
    throw new Error('Release output must be outside the portable directory.');
  }
  const files = [];
  async function inventory(directory) {
    for (const entry of await fs.promises.readdir(directory, { withFileTypes: true })) {
      const source = path.join(directory, entry.name), name = path.relative(portable, source).replaceAll('\\', '/');
      if (entry.isSymbolicLink()) throw new Error(`Linked release file: ${name}`);
      if (entry.isDirectory()) { await inventory(source); continue; }
      if (!entry.isFile() || !allowedFile(name)) throw new Error(`Unexpected release file: ${name}`);
      const size = (await fs.promises.stat(source)).size;
      if (size > LIMITS.file) throw new Error(`Release file is too large: ${name}`);
      files.push({ path: name, size, sha256: await hashFile(source) });
    }
  }
  await inventory(portable);
  if (REQUIRED.some(name => !files.some(file => file.path === name))) throw new Error('Portable payload is incomplete.');
  if (files.length > LIMITS.files || files.reduce((total, file) => total + file.size, 0) > LIMITS.total) throw new Error('Release payload exceeds update limits.');
  files.sort((a, b) => a.path.localeCompare(b.path));
  const stableTag = `atlas-${releaseVersion}`;
  const tag = bootstrap ? `atlas-payload-${releaseVersion}` : stableTag;
  const payload = bootstrap ? path.join(output, 'payload') : output;
  const stable = path.join(output, 'stable');
  if (bootstrap) {
    if (releaseVersion !== require('../package.json').version) throw new Error('Atlas loader version must match package.json.');
    for (const name of ['atlas-portable.json', 'bundle/game/atlas-runtime.json']) {
      const config = JSON.parse((await fs.promises.readFile(path.join(portable, name), 'utf8')).replace(/^\uFEFF/, ''));
      if (config.version !== releaseVersion) throw new Error(`Release version does not match ${name}.`);
    }
    execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', path.join(__dirname, 'Build-Bootstrap.ps1'), '-OutputDirectory', stable], { windowsHide: true, stdio: 'inherit' });
  }
  await fs.promises.mkdir(payload, { recursive: true });
  const archiveName = `Atlas-${releaseVersion}-Update.zip`, archive = path.join(payload, archiveName);
  // Arguments are passed separately, never interpolated into PowerShell source.
  execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', path.join(__dirname, 'zip-release.ps1'), '-SourceDirectory', portable, '-ArchivePath', archive], { windowsHide: true, stdio: 'inherit' });
  const size = (await fs.promises.stat(archive)).size;
  if (size > LIMITS.archive) throw new Error('Release archive exceeds the updater download limit.');
  const manifest = { schema: 1, repository: REPOSITORY, platform: 'win32-x64', version: releaseVersion, tag, archive: { name: archiveName, size, sha256: await hashFile(archive) }, files };
  const manifestFile = path.join(payload, 'atlas-update.json');
  await fs.promises.writeFile(manifestFile, JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  if (!bootstrap) {
    await fs.promises.writeFile(path.join(output, 'SHA256SUMS.txt'), `${manifest.archive.sha256}  ${archiveName}\n`, { flag: 'wx' });
    return { output, archive, manifest };
  }
  return {
    output, archive, manifest,
    releases: {
      stable: { tag: stableTag, assets: [path.join(stable, 'Atlas-Launcher.exe')] },
      payload: { tag, prerelease: true, assets: [archive, manifestFile] },
    },
  };
}
if (require.main === module) {
  const [portable, output, mode, extra] = process.argv.slice(2);
  if (!portable || !output || (mode !== undefined && mode !== '--bootstrap') || extra !== undefined) {
    console.error(USAGE);
    process.exitCode = 1;
  } else {
    buildRelease(portable, output, undefined, { bootstrap: mode === '--bootstrap' }).then(result => {
      console.log(result.releases ? JSON.stringify(result.releases, null, 2) : result.output);
    }).catch(error => { console.error(error.message); process.exitCode = 1; });
  }
}
module.exports = { buildRelease };
