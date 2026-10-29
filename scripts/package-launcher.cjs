const path = require('node:path');

(async () => {
  const { packager } = await import('@electron/packager');
  const root = path.resolve(__dirname, '..');
  const out = process.argv[2] ? path.resolve(process.argv[2]) : path.join(root, 'dist');
  const outputs = await packager({
    dir: root, name: 'Atlas Launcher', platform: 'win32', arch: 'x64',
    icon: path.join(root, 'assets/atlas-launcher.ico'), out, overwrite: true,
    asar: true, prune: true,
    ignore: [
      /^\/(engine|data|dist|docs|scripts|tests|work|tools|runtime|bundle)(\/|$)/,
      /^\/\./,
      /^\/[^/]+\.(lnk|cmd|md|png)$/i,
      /^\/(options(?:of)?\.txt|(?:atlas|horizon)-(?:extras|graphics|player|controller)\.txt|(?:atlas|horizon)-shaders\.toml)$/i,
      /^\/(accounts.*|launcher\.json|session\.json|control\.json|credentials\.json|tokens\.json)$/i,
      /^\/[^/]+\.(log|tmp|bak|dmp|stackdump)$/i,
    ],
  });
  for (const output of outputs) console.log(output);
})().catch(error => { console.error(error.message); process.exitCode = 1; });
