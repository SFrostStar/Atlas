// Dry run: node scripts/Rebrand-Atlas.cjs
// Apply only after the dry run has no collisions: node scripts/Rebrand-Atlas.cjs --apply
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

const SKIP = new Set(['.git', 'target', '.local', 'third_party', 'vendor', 'vendored', 'node_modules', '.cache', '.gradle', '.venv', 'venv', '__pycache__', 'benchmark-deps']);
const TEXT = new Set(['.rs','.toml','.lock','.md','.txt','.json','.json5','.yaml','.yml','.py','.java','.kt','.kts','.gradle','.sh','.ps1','.cjs','.js','.mjs','.ts','.css','.html','.wgsl','.wgslh','.wit','.svg','.desktop','.xml','.properties','.nix','.service','.socket','.conf','.cfg','.ini','.example','.in','.sql','.csv','.bat','.cmd']);
const PROTECTED = /^(?:licen[cs]e|copying|ofl|third[_-]party[_-]notices)(?:[._-]|$)/i;
const HASH = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
// Horizontal/horizontally are mathematical and external format/API vocabulary,
// not branding. Renaming those would change e.g. Minecraft advancement keys.
const renameBrand = text => text.replace(/horizon(?!tal)/gi, token => token === 'HORIZON' ? 'ATLAS' : token === 'Horizon' ? 'Atlas' : 'atlas');
// The replacement Atlas icon already exists. Retain the obsolete source icon under an unused
// legacy filename; rewritten source references intentionally use our current atlas.ico.
const PATH_OVERRIDES = new Map([['packaging/icons/horizon.ico', 'packaging/icons/atlas-legacy.ico']]);

function inside(root, file, allowRoot = false) {
  const relative = path.relative(path.resolve(root), path.resolve(file));
  return (!!relative || allowRoot) && relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}
function assertInside(root, file, allowRoot = false) {
  if (!inside(root, file, allowRoot)) throw new Error(`Path escapes the intended workspace: ${file}`);
}
function transformText(text, filename, localPackages) {
  if (PROTECTED.test(path.basename(filename))) return text;
  const preserved = [];
  const keep = value => { const index = preserved.push(value) - 1; return `\0${index}\0`; };
  const restore = value => value.replace(/\0(\d+)\0/g, (_match, index) => preserved[Number(index)]);
  // Keep legal attribution and actual upstream URLs. Rebranding must not invent a new repository
  // or claim that third-party work was written by Atlas.
  let source = text.replace(/^.*(?:copyright|co-authored-by:|spdx-license|generated with|based on.*horizon|derived from.*horizon).*$/gim, keep);
  source = source.replace(/(?:https?:\/\/|git@)[^\s<>"'`)\]}]+/gi, keep);
  // Mojang's pinned API is an external interface: getHorizonHeight is not Atlas branding.
  source = source.replace(/\bnet\.minecraft\.[\w.$/#]*/g, keep);
  source = source.replace(/\b(?:get|set|has|is)[A-Za-z0-9_$]*Horizon[A-Za-z0-9_$]*\b/g, keep);
  if (path.basename(filename) === 'Cargo.lock') {
    // Only our path package names and references to them change. Registry/git package metadata
    // and their checksums/source URLs remain exactly as Cargo recorded them.
    source = source.replace(/"([a-zA-Z0-9_-]+)([^"\r\n]*)"/g, (full, name, rest) => localPackages.has(name) ? `"${renameBrand(name)}${rest}"` : full);
  } else source = renameBrand(source);
  return restore(source);
}

function inventory(sourceRoot, workspaceRoot) {
  sourceRoot = path.resolve(sourceRoot); workspaceRoot = path.resolve(workspaceRoot);
  assertInside(workspaceRoot, sourceRoot);
  if (fs.lstatSync(sourceRoot).isSymbolicLink()) throw new Error('The source root must not be a symbolic link.');
  assertInside(fs.realpathSync(workspaceRoot), fs.realpathSync(sourceRoot));
  const entries = [], skipped = [];
  function walk(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      assertInside(sourceRoot, file);
      if (entry.isSymbolicLink()) { skipped.push(path.relative(sourceRoot, file)); continue; }
      if (entry.isDirectory()) {
        if (SKIP.has(entry.name)) { skipped.push(path.relative(sourceRoot, file)); continue; }
        entries.push({ file, directory: true }); walk(file);
      } else if (entry.isFile()) entries.push({ file, directory: false });
    }
  }
  walk(sourceRoot);
  const localPackages = new Set();
  for (const entry of entries) {
    if (!entry.directory && path.basename(entry.file) === 'Cargo.toml') {
      const manifest = fs.readFileSync(entry.file, 'utf8');
      const packageSection = manifest.match(/^\[package\]\s*\r?\n([\s\S]*?)(?=^\[|(?![\s\S]))/m)?.[1];
      const name = packageSection?.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
      if (name && /horizon/i.test(name)) localPackages.add(name);
    }
  }
  const edits = [], renames = [], finalPaths = new Map(), collisions = [];
  for (const entry of entries) {
    const relative = path.relative(sourceRoot, entry.file);
    const parts = relative.split(path.sep);
    const override = PATH_OVERRIDES.get(parts.join('/'));
    const finalRelative = override ? override.split('/').join(path.sep) : parts.map((part, index) => index === parts.length - 1 && PROTECTED.test(part) ? part : renameBrand(part)).join(path.sep);
    const finalPath = path.join(sourceRoot, finalRelative);
    assertInside(sourceRoot, finalPath);
    const key = process.platform === 'win32' ? finalPath.toLowerCase() : finalPath;
    const existing = finalPaths.get(key);
    if (existing && existing !== relative) collisions.push({ first: existing, second: relative, destination: finalRelative });
    else finalPaths.set(key, relative);
    const newName = override ? path.basename(finalRelative) : entry.directory || !PROTECTED.test(path.basename(entry.file)) ? renameBrand(path.basename(entry.file)) : path.basename(entry.file);
    if (newName !== path.basename(entry.file)) renames.push({ from: entry.file, to: path.join(path.dirname(entry.file), newName), finalPath });
    if (entry.directory || PROTECTED.test(path.basename(entry.file))) continue;
    const ext = path.extname(entry.file).toLowerCase();
    if (!TEXT.has(ext) && ext && !['.gitignore','.gitattributes','.dockerignore'].includes(path.basename(entry.file))) continue;
    const bytes = fs.readFileSync(entry.file);
    if (bytes.includes(0)) continue;
    let text; try { text = new TextDecoder('utf-8', { fatal: true }).decode(bytes); } catch { continue; }
    const updated = transformText(text, entry.file, localPackages);
    if (updated !== text) edits.push({ file: entry.file, finalPath, hash: HASH(bytes), updated, mode: fs.statSync(entry.file).mode });
  }
  renames.sort((a,b) => b.from.split(path.sep).length - a.from.split(path.sep).length || b.from.localeCompare(a.from));
  return { sourceRoot, workspaceRoot, entries, edits, renames, collisions, skipped, localPackages: [...localPackages] };
}

function applyPlan(plan) {
  const { sourceRoot, workspaceRoot, edits, renames, collisions } = plan;
  assertInside(workspaceRoot, sourceRoot);
  if (collisions.length) throw new Error(`Rebranding stopped: ${collisions.length} destination collisions. No files changed.`);
  // Validate every operation before changing a file. Never overwrite an existing rename target.
  for (const edit of edits) {
    assertInside(sourceRoot, edit.file);
    if (fs.lstatSync(edit.file).isSymbolicLink() || HASH(fs.readFileSync(edit.file)) !== edit.hash) throw new Error(`Source changed after inventory: ${edit.file}`);
  }
  for (const rename of renames) {
    assertInside(sourceRoot, rename.from); assertInside(sourceRoot, rename.to);
    if (fs.existsSync(rename.to)) throw new Error(`Rename target exists: ${rename.to}. No files changed.`);
  }
  const backupRoot = path.join(workspaceRoot, 'work', `rebrand-backup-${new Date().toISOString().replace(/[:.]/g, '-')}-${crypto.randomBytes(3).toString('hex')}`);
  assertInside(workspaceRoot, backupRoot);
  fs.mkdirSync(backupRoot, { recursive: true });
  for (const edit of edits) {
    const backup = path.join(backupRoot, 'files', path.relative(sourceRoot, edit.file));
    assertInside(backupRoot, backup);
    fs.mkdirSync(path.dirname(backup), { recursive: true });
    fs.copyFileSync(edit.file, backup, fs.constants.COPYFILE_EXCL);
  }
  fs.writeFileSync(path.join(backupRoot, 'manifest.json'), JSON.stringify({ sourceRoot, edits: edits.map(({file,finalPath,hash})=>({file,finalPath,hash})), renames }, null, 2), { flag: 'wx' });
  for (const edit of edits) {
    // Check again immediately before replacing a file; don't follow changed symlinks.
    if (fs.lstatSync(edit.file).isSymbolicLink() || HASH(fs.readFileSync(edit.file)) !== edit.hash) throw new Error(`Source changed during apply: ${edit.file}. Backup: ${backupRoot}`);
    const temporary = `${edit.file}.atlas-rebrand-${crypto.randomBytes(6).toString('hex')}.tmp`;
    assertInside(sourceRoot, temporary);
    try {
      fs.writeFileSync(temporary, edit.updated, { mode: edit.mode, flag: 'wx' });
      fs.renameSync(temporary, edit.file);
    } finally { if (fs.existsSync(temporary)) fs.unlinkSync(temporary); }
  }
  for (const rename of renames) {
    if (fs.lstatSync(rename.from).isSymbolicLink() || fs.existsSync(rename.to)) throw new Error(`Unsafe rename stopped. Backup: ${backupRoot}`);
    fs.renameSync(rename.from, rename.to);
  }
  return backupRoot;
}

function summary(plan) {
  return { sourceRoot: plan.sourceRoot, mode: 'dry-run', filesToEdit: plan.edits.length, namesToRename: plan.renames.length, localPackages: plan.localPackages.length, collisions: plan.collisions, excluded: plan.skipped, examples: plan.renames.slice(0, 8).map(item => ({from:path.relative(plan.sourceRoot,item.from),to:path.relative(plan.sourceRoot,item.finalPath)})) };
}
if (require.main === module) {
  const args = process.argv.slice(2);
  if (args.some(arg => arg !== '--apply')) { process.stderr.write('Usage: node scripts/Rebrand-Atlas.cjs [--apply]\n'); process.exitCode = 1; }
  else {
    try {
      const workspace = path.resolve(__dirname, '..');
      const plan = inventory(path.join(workspace, 'engine', 'atlas'), workspace);
      const report = summary(plan);
      if (args.includes('--apply')) { report.backupRoot = applyPlan(plan); report.mode = 'applied'; }
      process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
    } catch (error) { process.stderr.write(`${error.message}\n`); process.exitCode = 1; }
  }
}
module.exports = { inventory, applyPlan, summary, transformText, renameBrand };
