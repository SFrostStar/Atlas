const fs = require('node:fs/promises');
const path = require('node:path');
const { randomUUID } = require('node:crypto');
const { archiveNativeSource } = require('./native-source.cjs');

(async () => {
  const { packager } = await import('@electron/packager');
  const root = path.resolve(__dirname, '..');
  const out = process.argv[2] ? path.resolve(process.argv[2]) : path.join(root, 'dist');
  const { version } = require('../package.json');
  const archive = path.join(out, `Atlas-${version}-Native-Source.zip`);
  const pending = `${archive}.${randomUUID()}.tmp`;
  await fs.mkdir(out, { recursive: true });
  try {
    const source = await archiveNativeSource(root, pending, version);
    const outputs = await packager({
      dir: root, name: 'Atlas Launcher', platform: 'win32', arch: 'x64',
      icon: path.join(root, 'assets/atlas-launcher.ico'), out, overwrite: true,
      asar: true, prune: true,
      afterCopy: [({ buildPath }) => fs.writeFile(path.join(buildPath, 'native-source.json'), JSON.stringify(source) + '\n')],
      electronZipDir: process.env.ATLAS_ELECTRON_ZIP_DIR || undefined,
      ignore: [
        /^\/(engine|data|dist|docs|scripts|tests|work|tools|runtime|bundle|bootstrap|versions|website|native-source)(\/|$)/,
        /^\/(Atlas-Launcher\.exe|atlas-bootstrap-version\.txt|atlas-bootstrap\.lock|native-source\.json|Atlas-[\d.]+-Native-Source\.zip)$/i,
        /^\/\./,
        /^\/[^/]+\.(lnk|cmd|md|png)$/i,
        /^\/(options(?:of)?\.txt|(?:atlas|horizon)-(?:extras|graphics|player|controller)\.txt|(?:atlas|horizon)-shaders\.toml)$/i,
        /^\/(atlas-ports\.json|mod-catalog\.json|(?:atlas|horizon)-mods\.txt)$/i,
        /^\/(accounts.*|launcher\.json|session\.json|control\.json|credentials\.json|tokens\.json)$/i,
        /^\/[^/]+\.(log|tmp|bak|dmp|stackdump)$/i,
      ],
    });
    await fs.rename(pending, archive);
    for (const output of outputs) console.log(output);
    console.log(archive);
  } finally {
    await fs.unlink(pending).catch(error => { if (error.code !== 'ENOENT') throw error; });
  }
})().catch(error => { console.error(error.message); process.exitCode = 1; });
