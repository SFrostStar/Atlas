const fs = require('node:fs/promises');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { createHash } = require('node:crypto');

// Package the actual engine input, including local source edits, without build outputs.
async function copyNativeSource(project, destination) {
  const inventory = execFileSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard', '--', 'engine/atlas'], {
    cwd: project, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024,
  });
  const files = [...new Set(inventory.split('\0').filter(Boolean))].sort();
  const digest = createHash('sha256');
  for (const file of files) {
    const relative = file.slice('engine/atlas/'.length);
    if (!relative || relative.split('/').some(part => part === '..') ||
        /(?:^|\/)(?:target|\.local|saves|auth-sessions|logs|\.git)(?:\/|$)/i.test(relative) ||
        /(?:^|\/)(?:accounts|session|tokens|credentials|launcher)\.json$/i.test(relative) ||
        /\.(?:jar|class|exe|dll|log|mca|mcr)$/i.test(relative)) continue;
    const source = path.join(project, file);
    const entry = await fs.lstat(source).catch(error => {
      if (error.code !== 'ENOENT') throw error;
      return null;
    });
    if (!entry?.isFile() || await fs.realpath(source) !== source) continue;
    const target = path.join(destination, relative);
    const bytes = await fs.readFile(source);
    digest.update(relative + '\0').update(createHash('sha256').update(bytes).digest());
    await fs.mkdir(path.dirname(target), { recursive: true });
    await fs.writeFile(target, bytes);
  }
  for (const name of ['Cargo.toml', 'Cargo.lock', 'ATLAS-UPSTREAM.json']) await fs.access(path.join(destination, name));
  await fs.writeFile(path.join(destination, '.atlas-source-key'), digest.digest('hex') + '\n');
}

module.exports = { copyNativeSource };
if (require.main === module) copyNativeSource(path.join(__dirname, '..'), path.resolve(process.argv[2])).catch(error => {
  console.error(error.message);
  process.exitCode = 1;
});
