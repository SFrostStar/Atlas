const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { resolvePortableRoot } = require('../lib/portable-root.cjs');

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
