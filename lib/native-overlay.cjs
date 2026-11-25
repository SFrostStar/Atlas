const fs = require('node:fs/promises');
const path = require('node:path');
const { createHash } = require('node:crypto');
const { guard, safeRelative, hashFile } = require('./release-updater.cjs');

const DESCRIPTOR = 'source-overlay.json';
const FILE_LIMIT = 128;
const isHash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const validTarget = target => /^crates\/[a-z0-9-]+\/.+\.(?:rs|json|png)$/.test(safeRelative(target));

async function nativeWorkspace(base, input, mods, ids) {
  const edits = new Map();
  for (const id of ids) {
    const mod = guard(mods, path.join(mods, id));
    const descriptor = await fs.readFile(guard(mod, path.join(mod, DESCRIPTOR)), 'utf8')
      .catch(error => { if (error.code !== 'ENOENT') throw error; });
    if (descriptor === undefined) continue;
    if (Buffer.byteLength(descriptor) > 64 * 1024) throw new Error('Native source overlay manifest is too large.');
    const overlay = JSON.parse(descriptor);
    if (overlay?.schema !== 1 || !Array.isArray(overlay.files) || overlay.files.length > FILE_LIMIT) {
      throw new Error('Invalid native source overlay.');
    }
    for (const entry of overlay.files) {
      if (!entry || !validTarget(entry.path) || (entry.before !== null && !isHash(entry.before)) || !isHash(entry.after)) {
        throw new Error('Invalid file in native source overlay.');
      }
      const key = entry.path.toLowerCase();
      const prior = edits.get(key);
      if (prior && (prior.path !== entry.path || prior.before !== entry.before || prior.after !== entry.after)) {
        throw new Error(`Conflicting native source overlays: ${entry.path}`);
      }
      const replacement = await fs.readFile(guard(mod, path.join(mod, 'source-overlay', entry.path)));
      if (createHash('sha256').update(replacement).digest('hex') !== entry.after) {
        throw new Error(`Native source overlay checksum mismatch: ${entry.path}`);
      }
      edits.set(key, { ...entry, replacement });
    }
  }
  if (!edits.size) return base;
  if (edits.size > FILE_LIMIT) throw new Error('Too many native source overlay files.');

  for (const { path: relative, before } of edits.values()) {
    const original = guard(base, path.join(base, relative));
    const hash = await hashFile(original).catch(error => {
      if (error.code === 'ENOENT') return null;
      throw error;
    });
    if (hash !== before) throw new Error(`Native source overlay does not match this Atlas build: ${relative}`);
  }
  const workspace = guard(input, path.join(input, 'overlay-workspace'));
  await fs.cp(base, workspace, { recursive: true });
  for (const { path: relative, replacement } of edits.values()) {
    const file = guard(workspace, path.join(workspace, relative));
    await fs.mkdir(path.dirname(file), { recursive: true });
    await fs.writeFile(file, replacement);
  }
  return workspace;
}

module.exports = { nativeWorkspace };
