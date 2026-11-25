const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { test } = require('node:test');
const { createQaWorkspace, pruneQaWorkspaces } = require('../scripts/qa-workspace.cjs');

test('QA reclaims abandoned fixtures and preserves active or unowned work', t => {
  const project = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'atlas-qa-test-'));
  t.after(() => fs.rmSync(project, { recursive: true, force: true }));
  const active = createQaWorkspace(project);
  const stale = createQaWorkspace(project);
  const root = path.dirname(active.directory);
  const unowned = fs.mkdtempSync(path.join(root, 'fixture-'));
  const worker = spawnSync(process.execPath, ['-e', 'process.exit(0)'], { windowsHide: true });
  assert.equal(worker.status, 0);
  const old = new Date(Date.now() - 2 * 24 * 60 * 60 * 1000);
  const mark = directory => path.join(directory, '.atlas-qa-owner.json');
  fs.writeFileSync(mark(stale.directory), JSON.stringify({ schema: 1, kind: 'atlas-qa', pids: [worker.pid] }));
  fs.utimesSync(mark(stale.directory), old, old);
  fs.utimesSync(mark(active.directory), old, old);
  assert.equal(pruneQaWorkspaces(project), 1);
  assert.equal(fs.existsSync(stale.directory), false);
  assert.equal(fs.existsSync(unowned), true);
  assert.equal(fs.existsSync(active.directory), true);
  active.close();
  assert.equal(fs.existsSync(active.directory), false);
});
