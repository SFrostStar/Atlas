const fs = require('node:fs');
const path = require('node:path');

// Remove upstream's automatic offers, never entries from a player's servers.dat.
function stripBundledServerOffers(source) {
  const call = /(?:horizon|atlas)_client::multiplayer::add_(?:official_server|partner_servers)\s*\(/;
  if (!call.test(source)) return source;
  let updated = source.replace(/^[\t ]*if let Err\(\w+\) = (?:horizon|atlas)_client::multiplayer::add_(?:official_server|partner_servers)\([^\r\n]*\) \{\r?\n[\t ]*log::warn!\([^\r\n]*\);\r?\n[\t ]*\}\r?\n/gm, '');
  // Stop a changed upstream implementation from silently restoring promoted servers.
  if (call.test(updated)) throw new Error('Upstream server offers changed; review them before building Atlas.');
  updated = updated.replace(/^[\t ]*let player_dir = (?:horizon|atlas)_client::logging::player_dir\(\);\r?\n(?=[\t ]*\})/gm, '');
  return updated;
}

function applyServerPolicy(engineRoot) {
  const file = path.join(engineRoot, 'crates/atlas-client/src/game.rs');
  if (fs.lstatSync(file).isSymbolicLink()) throw new Error('Refusing a linked engine source file.');
  const source = fs.readFileSync(file, 'utf8');
  const updated = stripBundledServerOffers(source);
  if (updated !== source) fs.writeFileSync(file, updated);
  return updated !== source;
}

if (require.main === module) {
  try {
    const changed = applyServerPolicy(path.join(__dirname, '../engine/atlas'));
    console.log(changed ? 'Disabled upstream server offers.' : 'Upstream server offers already disabled.');
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
module.exports = { stripBundledServerOffers, applyServerPolicy };
