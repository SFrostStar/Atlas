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
  ...['lib.rs','client_lib.rs','entry.rs','main.rs','title.rs','title/atlas.rs','game.rs','game/packs.rs','game/server_packs.rs','game/startup_menu.rs','game/native_hooks.rs','game/extras.rs','game/remote.rs','game/chat.rs','game/stations.rs','game/debug_lines.rs','game/lan.rs']
    .map(name => 'crates/atlas-client/src/' + name),
  ...['game/agent.rs','game/fov.rs','game/hand.rs','game/maps.rs','game/creatures.rs','game/armor_stands.rs','game/glowing.rs','game/render_tick.rs','menu.rs'].map(name=>'crates/atlas-client/src/'+name),
  'crates/atlas-client-render/src/audio.rs', 'crates/atlas-client-render/src/chat.rs','crates/atlas-client-render/src/entity.wgsl',
  'crates/atlas-client-input/src/mouse.rs','crates/atlas-client/src/game/potions.rs',
  'crates/atlas-client/assets/atlas/lang/en_us.json','crates/atlas-client/assets/atlas/lang/ru_ru.json',
  'crates/atlas-client-render/Cargo.toml', 'crates/atlas-client/Cargo.toml', 'crates/atlas-client-lib/Cargo.toml', 'ATLAS-UPSTREAM.json', 'ATLAS-PORTS.md', 'MOD-PORT-NOTICES.md',
];
fs.mkdirSync(output, { recursive: true });
fs.cpSync(path.join(source, 'atlas'), path.join(output, 'atlas'), { recursive: true });
fs.cpSync(path.join(engine,'mod-port-licenses'),path.join(output,'mod-port-licenses'),{recursive:true});
const ports = path.join(engine, 'crates/atlas-client/src/game/atlas_ports');
if (fs.existsSync(ports)) fs.cpSync(ports, path.join(output, 'game-atlas-ports'), { recursive: true });
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
  format: 3, kind: 'Atlas presentation/loading, game feature ports and adapter references',
  upstreamBase: JSON.parse(fs.readFileSync(path.join(engine, 'ATLAS-UPSTREAM.json'), 'utf8')).commit,
  note: 'Adapter references need manual integration; this is not an upstream patch or engine backup.',
  files,
}, null, 2));
console.log(output);
