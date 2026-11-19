const fs = require('node:fs/promises');
const { constants } = require('node:fs');
const path = require('node:path');
const { randomUUID } = require('node:crypto');
const { guard } = require('./release-updater.cjs');

async function inventory(root) {
  const directories = new Set();
  const files = new Set();
  async function visit(relative) {
    const directory = path.join(root, relative);
    for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
      const name = path.join(relative, entry.name);
      if (entry.isDirectory()) {
        directories.add(name);
        await visit(name);
      } else if (entry.isFile()) {
        files.add(name);
      } else {
        throw new Error(`Linked or unsupported native stage entry: ${name}`);
      }
    }
  }
  await visit('');
  return { directories, files };
}

function contains(root, location) {
  const relative = path.relative(root, location);
  return relative !== '..' && !relative.startsWith('..' + path.sep) && !path.isAbsolute(relative);
}

async function syncNativeStage(source, stage) {
  if (!path.isAbsolute(source) || !path.isAbsolute(stage)) throw new Error('Native stage paths must be absolute.');
  source = guard(source, source);
  stage = guard(stage, stage);
  if (contains(source, stage) || contains(stage, source)) throw new Error('Native stage paths overlap.');
  const incoming = await inventory(source);
  await fs.mkdir(stage, { recursive: true });
  const existing = await inventory(stage);
  const destination = name => guard(stage, path.join(stage, name));

  // Reject links in both trees before changing anything, then remove obsolete entries
  // one at a time so a directory can become a file or vice versa.
  for (const name of existing.files) {
    if (!incoming.files.has(name)) await fs.unlink(destination(name));
  }
  for (const name of [...existing.directories].reverse()) {
    if (!incoming.directories.has(name)) await fs.rmdir(destination(name));
  }
  for (const name of incoming.directories) {
    if (!existing.directories.has(name)) await fs.mkdir(destination(name));
  }
  for (const name of incoming.files) {
    const from = path.join(source, name);
    const to = path.join(stage, name);
    if (existing.files.has(name)) {
      const [previous, updated] = await Promise.all([fs.readFile(to), fs.readFile(from)]);
      if (previous.equals(updated)) continue;
    }
    const target = destination(name);
    const temp = destination(path.join(path.dirname(name), `.atlas-stage-${randomUUID()}.tmp`));
    try {
      await fs.copyFile(from, temp, constants.COPYFILE_EXCL);
      // Cargo checks timestamps, and copies can inherit the input's older timestamp.
      const now = new Date();
      await fs.utimes(temp, now, now);
      await fs.rename(temp, target);
    } finally {
      await fs.unlink(temp).catch(error => {
        if (error.code !== 'ENOENT') throw error;
      });
    }
  }
}

module.exports = { syncNativeStage };
