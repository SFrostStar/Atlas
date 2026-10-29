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

module.exports = { resolvePortableRoot };
