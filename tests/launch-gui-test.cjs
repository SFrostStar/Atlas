const { _electron } = require('playwright-core');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

module.exports = async function launchGuiTest() {
  const root = path.resolve(__dirname, '..');
  const temp = fs.realpathSync(os.tmpdir());
  const scratch = fs.mkdtempSync(path.join(temp, 'atlas-launcher-qa-'));
  const cleanup = () => {
    if (path.dirname(fs.realpathSync(scratch)) !== temp || fs.lstatSync(scratch).isSymbolicLink()) {
      throw new Error('Unexpected launcher test directory');
    }
    fs.rmSync(scratch, { recursive: true, maxRetries: 3, retryDelay: 250 });
  };
  try {
    const app = await _electron.launch({
      executablePath: path.join(root, 'node_modules/electron/dist/electron.exe'),
      args: [root],
      cwd: root,
      env: { ...process.env, ATLAS_GUI_TEST: scratch, ATLAS_NO_AUDIO: '1', ATLAS_NO_DISCORD: '1' },
    });
    return {
      app,
      async close() {
        try { await app.close(); }
        finally { cleanup(); }
      },
    };
  } catch (error) {
    cleanup();
    throw error;
  }
};
