const fs = require('node:fs');
const path = require('node:path');

if (process.versions.electron) {
  const { app } = require('electron');
  const fixture = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  fs.mkdirSync(fixture.profile, { recursive: true });
  app.setPath('userData', fixture.profile);
  app.setPath('sessionData', fixture.profile);
  app.whenReady().then(async () => {
    const assert = require('node:assert/strict');
    const { extractArchive, hashFile } = require('../lib/release-updater.cjs');
    await extractArchive(fixture.archive, fixture.destination, fixture.files);
    for (const file of fixture.files) {
      assert.equal(await hashFile(path.join(fixture.destination, file.path)), file.sha256);
    }
    app.exit(0);
  }).catch(error => { console.error(error.stack); app.exit(1); });
} else {
  const test = require('node:test');
  const assert = require('node:assert/strict');
  const { execFileSync } = require('node:child_process');
  const { hashFile } = require('../lib/release-updater.cjs');
  test('Electron extracts and hashes app.asar as a real update file, outside its virtual archive filesystem', { skip: process.platform !== 'win32' }, async () => {
    const root = path.resolve(__dirname, '..');
    const fixtureRoot = fs.mkdtempSync(path.join(root, 'work', 'update-asar-test-'));
    const source = path.join(fixtureRoot, 'app-source');
    const portable = path.join(fixtureRoot, 'portable');
    fs.mkdirSync(source, { recursive: true });
    fs.mkdirSync(path.join(portable, 'resources'), { recursive: true });
    fs.writeFileSync(path.join(source, 'package.json'), JSON.stringify({ name: 'update-fixture', version: '1.0.0', main: 'main.cjs' }));
    fs.writeFileSync(path.join(source, 'main.cjs'), '// Original Atlas test fixture.\n');
    const asar = await import('@electron/asar');
    const packagePath = path.join(portable, 'resources', 'app.asar');
    await asar.createPackage(source, packagePath);
    const archive = path.join(fixtureRoot, 'update.zip');
    execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', path.join(root, 'scripts', 'zip-release.ps1'), '-SourceDirectory', portable, '-ArchivePath', archive], { windowsHide: true, stdio: 'pipe' });
    const fixture = { archive, destination: path.join(fixtureRoot, 'extracted'), profile: path.join(fixtureRoot, 'profile'), files: [{ path: 'resources/app.asar', size: fs.statSync(packagePath).size, sha256: await hashFile(packagePath) }] };
    const input = path.join(fixtureRoot, 'fixture.json');
    fs.writeFileSync(input, JSON.stringify(fixture));
    const env = { ...process.env, APPDATA: path.join(fixtureRoot, 'roaming'), LOCALAPPDATA: path.join(fixtureRoot, 'local'), ATLAS_LAUNCHER_NO_SELF_UPDATE: '1' };
    delete env.ELECTRON_RUN_AS_NODE;
    const output = execFileSync(require('electron'), [__filename, input], { env, windowsHide: true, timeout: 30000, encoding: 'utf8', stdio: 'pipe' });
    assert.ok(fs.existsSync(path.join(fixture.destination, 'resources', 'app.asar')), output);
  });
}
