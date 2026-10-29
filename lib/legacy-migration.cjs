const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

// Compatibility names belong here, rather than in the current game's configuration paths.
const NAMES = Object.freeze({
  'horizon-extras.txt': 'atlas-extras.txt',
  'horizon-graphics.txt': 'atlas-graphics.txt',
  'horizon-player.txt': 'atlas-player.txt',
  'horizon-shaders.toml': 'atlas-shaders.toml',
  'horizon-mods': 'atlas-mods',
});
function contained(root, target, includeRoot = false) {
  const relative = path.relative(root, target);
  return (!!relative || includeRoot) && relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}
function guard(root, target, includeRoot = false) {
  if (!contained(root, target, includeRoot)) throw new Error('A migration path escapes its data directory.');
  let current = root;
  for (const part of ['', ...path.relative(root, target).split(path.sep).filter(Boolean)]) {
    if (part) current = path.join(current, part);
    let currentStat;
    try { currentStat = fs.lstatSync(current); } catch (error) { if (error.code === 'ENOENT') continue; throw error; }
    if (currentStat.isSymbolicLink()) throw new Error('Migration does not follow symbolic links.');
    if (!contained(fs.realpathSync(root), fs.realpathSync(current), true)) throw new Error('A migration path points outside its data directory.');
  }
}
function fileHash(file) {
  const hash = crypto.createHash('sha256');
  const handle = fs.openSync(file, 'r');
  try {
    const buffer = Buffer.allocUnsafe(64 * 1024);
    for (;;) { const count = fs.readSync(handle, buffer, 0, buffer.length, null); if (!count) break; hash.update(buffer.subarray(0, count)); }
  } finally { fs.closeSync(handle); }
  return hash.digest('hex');
}
function inspectTree(root, target) {
  guard(root, target);
  const stat = fs.lstatSync(target);
  if (stat.isSymbolicLink()) throw new Error('Migration does not follow symbolic links.');
  if (stat.isFile()) return { type: 'file', size: stat.size, hash: fileHash(target) };
  if (!stat.isDirectory()) throw new Error('Migration supports only ordinary files and directories.');
  const entries = Object.create(null);
  for (const entry of fs.readdirSync(target, { withFileTypes: true }).sort((a,b) => a.name.localeCompare(b.name))) entries[entry.name] = inspectTree(root, path.join(target, entry.name));
  return { type: 'directory', entries };
}
function copyTree(sourceRoot, source, destinationRoot, destination) {
  guard(sourceRoot, source); guard(destinationRoot, destination);
  const stat = fs.lstatSync(source);
  if (stat.isDirectory()) {
    fs.mkdirSync(destination);
    for (const entry of fs.readdirSync(source)) copyTree(sourceRoot, path.join(source, entry), destinationRoot, path.join(destination, entry));
  } else if (stat.isFile()) fs.copyFileSync(source, destination, fs.constants.COPYFILE_EXCL);
  else throw new Error('Migration does not follow symbolic links.');
}
function ensureDirectory(root, directory) {
  guard(root, directory, true);
  fs.mkdirSync(directory, { recursive: true });
  guard(root, directory, true);
}

function migrateLegacyData({ dataRoot, projectRoot } = {}) {
  if (!path.isAbsolute(dataRoot || '') || projectRoot !== undefined && !path.isAbsolute(projectRoot || '')) throw new Error('Migration directories must be absolute.');
  dataRoot = path.resolve(dataRoot);
  projectRoot = projectRoot === undefined ? null : path.resolve(projectRoot);
  const result = { changed: false, renamed: [], archived: [], conflicts: [], skipped: [], messages: [], backupRoot: null };
  const message = (code, from, to, text) => result.messages.push({ code, from, to, message: text });
  if (!fs.existsSync(dataRoot)) return result;
  if (fs.lstatSync(dataRoot).isSymbolicLink() || projectRoot && fs.lstatSync(projectRoot).isSymbolicLink()) throw new Error('Migration roots must not be symbolic links.');
  const backupBase = projectRoot || dataRoot;
  const backupRoot = projectRoot ? path.join(projectRoot, 'work', 'legacy-data-backup') : path.join(dataRoot, '.migration-backups');
  let backupReady = false;
  function backup(sourceRoot, source, label, fingerprint) {
    if (!backupReady) { ensureDirectory(backupBase, backupRoot); backupReady = true; result.backupRoot = backupRoot; }
    let destination = path.join(backupRoot, label);
    guard(backupRoot, destination);
    ensureDirectory(backupRoot, path.dirname(destination));
    if (fs.existsSync(destination)) {
      if (JSON.stringify(inspectTree(backupRoot, destination)) === JSON.stringify(fingerprint)) return destination;
      destination = `${destination}.${crypto.randomUUID()}.backup`;
    }
    guard(backupRoot, destination);
    copyTree(sourceRoot, source, backupRoot, destination);
    if (JSON.stringify(inspectTree(backupRoot, destination)) !== JSON.stringify(fingerprint)) throw new Error('Migration backup verification failed.');
    result.backupRoot = backupRoot;
    return destination;
  }
  const homes = [];
  function collect(base, session) {
    if (!fs.existsSync(base)) return;
    try { guard(dataRoot, base); }
    catch (error) { result.skipped.push(base); message('unsafe_path',base,null,error.message); return; }
    for (const entry of fs.readdirSync(base, { withFileTypes: true })) {
      const directory = path.join(base, entry.name);
      if (entry.isSymbolicLink()) { result.skipped.push(directory); message('unsafe_path',directory,null,'Skipped a linked data directory.'); continue; }
      if (!entry.isDirectory()) continue;
      const home = session ? path.join(directory, 'home') : directory;
      if (fs.existsSync(home)) homes.push({ root: dataRoot, home, prefix: path.join('data',path.relative(dataRoot,home)) });
    }
  }
  collect(path.join(dataRoot,'instances'), false);
  collect(path.join(dataRoot,'sessions'), true);
  if (projectRoot) homes.push({ root: projectRoot, home: projectRoot, prefix: 'project' });
  for (const { root, home, prefix } of homes) {
    for (const [oldName,newName] of Object.entries(NAMES)) {
      // Project-root leftovers are settings only; instance mod folders belong under data.
      if (root === projectRoot && home === projectRoot && oldName === 'horizon-mods') continue;
      const source = path.join(home,oldName), destination = path.join(home,newName);
      try {
        guard(root,source); guard(root,destination);
        if (!fs.existsSync(source)) continue;
        const original = inspectTree(root,source);
        const targetExists = fs.existsSync(destination);
        const target = targetExists ? inspectTree(root,destination) : null;
        const backedUp = backup(root,source,path.join(prefix,oldName),original);
        // Recheck source and target immediately before any move/unlink.
        guard(root,source); guard(root,destination);
        if (JSON.stringify(inspectTree(root,source)) !== JSON.stringify(original)) throw new Error('Source changed during migration; it was preserved.');
        if (targetExists) {
          const identicalFile = original.type === 'file' && target.type === 'file' && original.size === target.size && original.hash === target.hash;
          if (identicalFile && JSON.stringify(inspectTree(root,destination)) === JSON.stringify(target)) {
            fs.unlinkSync(source);
            result.archived.push({ from:source,to:backedUp }); result.changed = true;
            message('archived_duplicate',source,backedUp,'Archived an identical legacy settings file.');
          } else {
            result.conflicts.push({ from:source,to:destination,backup:backedUp });
            message('conflict',source,destination,'Both versions were preserved. Atlas uses the existing Atlas path; the legacy data also has a backup.');
          }
        } else {
          if (fs.existsSync(destination)) throw new Error('Destination appeared during migration; both versions were preserved.');
          fs.renameSync(source,destination);
          result.renamed.push({ from:source,to:destination,backup:backedUp }); result.changed = true;
          message('renamed',source,destination,'Migrated legacy settings to Atlas.');
        }
      } catch (error) {
        result.skipped.push(source);
        message('skipped',source,destination,error.message);
      }
    }
  }
  return result;
}

module.exports = { migrateLegacyData, NAMES };
