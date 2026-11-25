const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { getRawHeader, extractFile } = require('@electron/asar');
const { allowedFile, hashFile, guard, version, REQUIRED, REPOSITORY, LIMITS } = require('../lib/release-updater.cjs');
const { SOURCE_LIMITS } = require('../lib/native-source.cjs');

const USAGE = 'Usage: node scripts/build-release.cjs <flat-portable-directory> <new-output-directory> [--bootstrap]';
async function nativeSourceForRelease(portable, releaseVersion) {
  const app = path.join(portable, 'resources', 'app.asar');
  const { header } = getRawHeader(app);
  if (!Object.hasOwn(header.files, 'native-source.json')) return null;
  if (Object.hasOwn(header.files, 'native-source')) throw new Error('The launcher still bundles the native source tree.');
  const descriptor = JSON.parse(extractFile(app, 'native-source.json').toString('utf8'));
  if (descriptor?.schema !== 1 || descriptor.version !== releaseVersion ||
      !/^[a-f0-9]{64}$/.test(descriptor.sourceKey || '') || !/^[a-f0-9]{40}$/.test(descriptor.revision || '') ||
      !Number.isSafeInteger(descriptor.size) || descriptor.size < 22 || descriptor.size > SOURCE_LIMITS.archive ||
      !/^[a-f0-9]{64}$/.test(descriptor.sha256 || '')) throw new Error('Invalid launcher native source descriptor.');
  const parent = path.dirname(portable);
  const name = `Atlas-${releaseVersion}-Native-Source.zip`;
  const archive = guard(parent, path.join(parent, name));
  const stat = await fs.promises.stat(archive).catch(error => {
    if (error.code !== 'ENOENT') throw error;
    throw new Error(`Missing ${name} alongside the portable directory.`);
  });
  if (!stat.isFile() || stat.size !== descriptor.size || await hashFile(archive) !== descriptor.sha256) {
    throw new Error('Native source archive does not match the launcher descriptor.');
  }
  return { archive, name, sha256: descriptor.sha256 };
}

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
  const nativeSource = await nativeSourceForRelease(portable, releaseVersion);
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
  const assets = [archive, manifestFile];
  if (nativeSource) {
    const target = path.join(payload, nativeSource.name);
    await fs.promises.copyFile(nativeSource.archive, target, fs.constants.COPYFILE_EXCL);
    assets.push(target);
  }
  if (!bootstrap) {
    const checksums = [`${manifest.archive.sha256}  ${archiveName}`];
    if (nativeSource) checksums.push(`${nativeSource.sha256}  ${nativeSource.name}`);
    await fs.promises.writeFile(path.join(output, 'SHA256SUMS.txt'), checksums.join('\n') + '\n', { flag: 'wx' });
    return { output, archive, manifest };
  }
  return {
    output, archive, manifest,
    releases: {
      stable: { tag: stableTag, assets: [path.join(stable, 'Atlas-Launcher.exe')] },
      payload: { tag, prerelease: true, assets },
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
