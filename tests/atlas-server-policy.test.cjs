const { test } = require('node:test');
const assert = require('node:assert/strict');
const { stripBundledServerOffers } = require('../scripts/atlas-server-policy.cjs');
const { transformText } = require('../scripts/Rebrand-Atlas.cjs');

const fixture = brand => `fn launch() {
    heartbeat::start();
    let player_dir = ${brand}_client::logging::player_dir();
    if let Err(e) = ${brand}_client::multiplayer::add_official_server(&player_dir) {
        log::warn!("could not add official: {e}");
    }
    if let Err(e) = ${brand}_client::multiplayer::add_partner_servers(&player_dir) {
        log::warn!("could not add partners: {e}");
    }
}
fn user_servers() { servers::load("servers.dat"); }
`;

test('both upstream offers are removed, preserving other startup and user server loading', () => {
  for (const brand of ['atlas', 'horizon']) {
    for (const newline of ['\n', '\r\n']) {
      const updated = stripBundledServerOffers(fixture(brand).replaceAll('\n', newline));
      assert(!updated.includes('add_official_server'));
      assert(!updated.includes('add_partner_servers'));
      assert(!updated.includes('let player_dir'));
      assert(updated.includes('heartbeat::start();'));
      assert(updated.includes('servers::load("servers.dat");'));
      assert.equal(stripBundledServerOffers(updated), updated);
    }
  }
});
test('rebranding an upstream update also applies the Atlas server policy', () => {
  const updated = transformText(fixture('horizon'), 'crates/horizon-client/src/game.rs', new Set());
  assert(!updated.includes('multiplayer::add_'));
  assert(updated.includes('servers::load("servers.dat");'));
});
test('an unrecognized upstream injection blocks the build rather than being ignored', () => {
  assert.throws(() => stripBundledServerOffers('atlas_client::multiplayer::add_official_server(&dir)?;'), /changed/);
});
