const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { allowedFile, hashFile, version, REQUIRED, REPOSITORY, LIMITS } = require('../lib/release-updater.cjs');

// Usage: node scripts/build-release.cjs <flat-portable-directory> <new-output-directory>
// Feed this only the output of Build-Portable.ps1, never a running installation.
async function buildRelease(portable, output, releaseVersion = require('../package.json').version) {
  portable = path.resolve(portable); output = path.resolve(output); releaseVersion = version(releaseVersion);
  if (fs.existsSync(output)) throw new Error('Release output already exists; use a fresh output directory.');
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
  await fs.promises.mkdir(output, { recursive: true });
  const archiveName = `Atlas-${releaseVersion}-Update.zip`, archive = path.join(output, archiveName);
  // Arguments are passed separately, never interpolated into PowerShell source.
  execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', path.join(__dirname, 'zip-release.ps1'), '-SourceDirectory', portable, '-ArchivePath', archive], { windowsHide: true, stdio: 'inherit' });
  const size = (await fs.promises.stat(archive)).size;
  if (size > LIMITS.archive) throw new Error('Release archive exceeds the updater download limit.');
  const manifest = { schema: 1, repository: REPOSITORY, platform: 'win32-x64', version: releaseVersion, tag: `atlas-${releaseVersion}`, archive: { name: archiveName, size, sha256: await hashFile(archive) }, files };
  await fs.promises.writeFile(path.join(output, 'atlas-update.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  await fs.promises.writeFile(path.join(output, 'SHA256SUMS.txt'), `${manifest.archive.sha256}  ${archiveName}\n`, { flag: 'wx' });
  return { output, archive, manifest };
}
if (require.main === module) {
  if (!process.argv[2] || !process.argv[3]) { console.error('Usage: node scripts/build-release.cjs <flat-portable-directory> <new-output-directory>'); process.exitCode = 1; }
  else buildRelease(process.argv[2], process.argv[3]).then(result => console.log(result.output)).catch(error => { console.error(error.message); process.exitCode = 1; });
}
module.exports = { buildRelease };
