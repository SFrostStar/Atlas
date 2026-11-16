const fs = require('node:fs');
const path = require('node:path');

// Complete portable downloads keep the executable and bundle side by side.
// The developer checkout retains its existing dist/<app> layout and data path.
function resolvePortableRoot(executable) {
  const directory = path.dirname(path.resolve(executable));
  if (fs.existsSync(path.join(directory, 'atlas-portable.json')) ||
      fs.existsSync(path.join(directory, 'bundle', 'game', 'game-root', 'research-lock.json'))) {
    return directory;
  }
  return path.resolve(directory, '..', '..');
}

function resolveLauncherLayout(executable) {
  const directory = path.dirname(path.resolve(executable));
  const versions = path.dirname(directory);
  const root = path.dirname(versions);
  const marker = path.join(root, 'atlas-bootstrap-version.txt');
  const bundle = path.join(directory, 'bundle', 'game');
  if (path.basename(versions) === 'versions' && /^\d+\.\d+\.\d+$/.test(path.basename(directory))) {
    try {
      const markerInfo = fs.lstatSync(marker);
      if (markerInfo.isFile() && markerInfo.size <= 40 &&
          /^\d+\.\d+\.\d+$/.test(fs.readFileSync(marker, 'utf8').trim()) &&
          fs.realpathSync(bundle) === bundle &&
          fs.statSync(path.join(bundle, 'game-root', 'research-lock.json')).isFile()) {
        return { projectRoot: root, bundleDirectory: bundle, bootstrapManaged: true };
      }
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }
  return { projectRoot: resolvePortableRoot(executable), bootstrapManaged: false };
}

module.exports = { resolvePortableRoot, resolveLauncherLayout };
