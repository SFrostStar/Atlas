const fs = require('node:fs');
const path = require('node:path');
const { guard } = require('../lib/release-updater.cjs');

const OWNER = '.atlas-qa-owner.json';
const PREFIX = 'fixture-';
const ORPHAN_AGE = 24 * 60 * 60 * 1000;
const project = path.resolve(__dirname, '..');

function qaRoot(workspace) {
  return guard(workspace, path.join(workspace, 'work', 'qa'));
}

function alive(pid) {
  try { process.kill(pid, 0); return true; }
  catch (error) { return error.code !== 'ESRCH'; }
}

function removeFixture(root, directory) {
  const target = guard(root, directory);
  if (path.dirname(target) !== root || !/^fixture-[A-Za-z0-9]{6}$/.test(path.basename(target))) {
    throw new Error('Unsafe Atlas QA fixture path.');
  }
  if (!fs.lstatSync(target).isDirectory() || fs.realpathSync(target) !== path.join(fs.realpathSync(root), path.basename(target))) {
    throw new Error('Linked Atlas QA fixtures cannot be removed.');
  }
  fs.rmSync(target, { recursive: true, maxRetries: 3, retryDelay: 150 });
}

function pruneQaWorkspaces(workspace = project, now = Date.now()) {
  const root = qaRoot(workspace);
  if (!fs.existsSync(root)) return 0;
  let count = 0;
  for (const item of fs.readdirSync(root, { withFileTypes: true })) {
    if (!item.isDirectory() || !/^fixture-[A-Za-z0-9]{6}$/.test(item.name)) continue;
    const directory = path.join(root, item.name);
    try {
      const marker = guard(root, path.join(directory, OWNER));
      const stat = fs.lstatSync(marker);
      if (!stat.isFile() || stat.size > 1024 || now - stat.mtimeMs < ORPHAN_AGE) continue;
      const owner = JSON.parse(fs.readFileSync(marker, 'utf8'));
      if (owner.schema !== 1 || owner.kind !== 'atlas-qa' || !Array.isArray(owner.pids)
          || owner.pids.length < 1 || owner.pids.length > 2
          || owner.pids.some(pid => !Number.isSafeInteger(pid) || pid < 1 || alive(pid))) continue;
      removeFixture(root, directory);
      count++;
    } catch {
      // A live, linked or unreadable path can wait for the next run.
    }
  }
  return count;
}

function createQaWorkspace(workspace = project) {
  pruneQaWorkspaces(workspace);
  const root = qaRoot(workspace);
  fs.mkdirSync(root, { recursive: true });
  const directory = fs.mkdtempSync(path.join(root, PREFIX));
  const marker = guard(root, path.join(directory, OWNER));
  const writeOwner = (pids, flag) => fs.writeFileSync(marker, JSON.stringify({ schema: 1, kind: 'atlas-qa', pids }), { flag });
  writeOwner([process.pid], 'wx');
  return {
    directory,
    trackChild(pid) { writeOwner([process.pid, pid], 'w'); },
    close() { removeFixture(root, directory); },
  };
}

if (require.main === module) console.log(`Atlas QA: removed ${pruneQaWorkspaces()} abandoned fixtures.`);
module.exports = { createQaWorkspace, pruneQaWorkspaces };
