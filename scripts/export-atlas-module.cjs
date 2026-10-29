const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

const root = path.resolve(__dirname, '..');
const engine = path.join(root, 'engine/atlas');
const source = path.join(engine, 'crates/atlas-client-render/src');
const output = path.join(root, 'work', `atlas-module-${Date.now()}`);
const adapters = [
  ...['lib.rs','atlas_menu.rs','atlas_loading.rs','ui_theme.rs','options_ui.rs','ui.rs',
    'font.rs','hud.rs','hud.wgsl','hud_overlays.rs','screen_draw.rs','packs.rs','packs/cache.rs',
    'gfx.rs'].map(name => 'crates/atlas-client-render/src/' + name),
  ...['lib.rs','title.rs','title/atlas.rs','game.rs','game/packs.rs','game/startup_menu.rs']
    .map(name => 'crates/atlas-client/src/' + name),
  'crates/atlas-client-render/Cargo.toml', 'ATLAS-UPSTREAM.json',
];
fs.mkdirSync(output, { recursive: true });
fs.cpSync(path.join(source, 'atlas'), path.join(output, 'atlas'), { recursive: true });
for (const name of adapters) {
  const target = path.join(output, 'adapters-reference', name);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(path.join(engine, name), target);
}
const localGuide = path.join(root, 'docs/ATLAS-MODULE.md');
if (fs.existsSync(localGuide)) fs.copyFileSync(localGuide, path.join(output, 'README.md'));
else fs.writeFileSync(path.join(output, 'README.md'),
  '# Atlas module\n\nThe atlas/ directory owns presentation and resource loading.\n' +
  'adapters-reference/ contains engine integration references, not drop-in replacements.\n' +
  'Validate the adapters and rebuild after each upstream update.\n');
const files = {};
function inventory(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) inventory(file);
    else files[path.relative(output, file).replaceAll('\\', '/')] =
      crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
  }
}
inventory(output);
fs.writeFileSync(path.join(output, 'manifest.json'), JSON.stringify({
  format: 2, kind: 'Atlas presentation/loading module and adapter references',
  upstreamBase: JSON.parse(fs.readFileSync(path.join(engine, 'ATLAS-UPSTREAM.json'), 'utf8')).commit,
  note: 'Adapter references need manual integration; this is not an upstream patch or engine backup.',
  files,
}, null, 2));
console.log(output);
