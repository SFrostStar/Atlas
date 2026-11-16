const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { resolvePortableRoot, resolveLauncherLayout } = require('../lib/portable-root.cjs');
const { GameInstallManager } = require('../lib/game-install.cjs');

function removeFixture(root) {
  const absolute = path.resolve(root);
  assert.equal(path.dirname(absolute), path.resolve(os.tmpdir()));
  assert.ok(path.basename(absolute).startsWith('atlas-portable-'));
  fs.rmSync(absolute, { recursive: true, force: true });
}

test('flat portable bundle stores new-player data alongside the executable', t => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-portable-'));
  t.after(() => removeFixture(root));
  const gameRoot = path.join(root, 'bundle/game/game-root');
  fs.mkdirSync(gameRoot, { recursive: true });
  fs.writeFileSync(path.join(gameRoot, 'research-lock.json'), '{}');
  assert.equal(resolvePortableRoot(path.join(root, 'Atlas Launcher.exe')), root);
});

test('portable marker keeps data in place even if the payload is incomplete', t => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-portable-'));
  t.after(() => removeFixture(root));
  fs.writeFileSync(path.join(root, 'atlas-portable.json'), '{}');
  assert.equal(resolvePortableRoot(path.join(root, 'Atlas Launcher.exe')), root);
});

test('existing dist installation preserves its current project and data folder', () => {
  const root = path.resolve(__dirname, '..');
  assert.equal(resolvePortableRoot(path.join(root, 'dist', 'Atlas Launcher-win32-x64', 'Atlas Launcher.exe')), root);
});

test('bootstrap releases share data and prepared runtime across versioned launcher directories', t => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-portable-'));
  t.after(() => removeFixture(root));
  fs.writeFileSync(path.join(root, 'atlas-bootstrap-version.txt'), '1.4.0\n');
  for (const version of ['1.3.0', '1.4.0']) {
    const directory = path.join(root, 'versions', version);
    const bundle = path.join(directory, 'bundle', 'game');
    const gameRoot = path.join(bundle, 'game-root');
    fs.mkdirSync(gameRoot, { recursive: true });
    fs.writeFileSync(path.join(gameRoot, 'research-lock.json'), JSON.stringify({ minecraft: { version: '26.3' } }));
    const layout = resolveLauncherLayout(path.join(directory, 'Atlas Launcher.exe'));
    assert.deepEqual(layout, { projectRoot: root, bundleDirectory: bundle, bootstrapManaged: true });
    const manager = new GameInstallManager({ projectRoot: layout.projectRoot, dataRoot: path.join(root, 'data'), bundleDirectory: layout.bundleDirectory });
    assert.equal(manager.context().source, gameRoot);
    assert.equal(manager.runtime, path.join(root, 'runtime'));
    assert.equal(manager.root, path.join(root, 'data', 'install'));
  }
});
