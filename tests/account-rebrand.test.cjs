const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { inventory, applyPlan, transformText } = require('../scripts/Rebrand-Atlas.cjs');

function fixture(t) {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-rebrand-test-'));
  t.after(() => {
    if (path.dirname(workspace) !== os.tmpdir() || !path.basename(workspace).startsWith('atlas-rebrand-test-')) throw new Error('Unsafe test cleanup');
    fs.rmSync(workspace, { recursive: true, force: true });
  });
  const source = path.join(workspace, 'engine', 'atlas');
  fs.mkdirSync(source, { recursive: true });
  const write = (relative, text) => { const file = path.join(source, relative); fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, text); };
  return { workspace, source, write };
}
test('rebranding retains legal notices, external URLs and the pinned Minecraft interface', () => {
  const text = 'Horizon horizon HORIZON\nCopyright (c) 2026 Horizon contributors\nuse horizon_client;\nhttps://github.com/horizon-rust/horizon\npackage dev.horizonlab.parity;\nlevel.getHorizonHeight();\nnet.minecraft.world.HorizonReference';
  assert.equal(transformText(text, 'source.rs', new Set()), 'Atlas atlas ATLAS\nCopyright (c) 2026 Horizon contributors\nuse atlas_client;\nhttps://github.com/horizon-rust/horizon\npackage dev.atlaslab.parity;\nlevel.getHorizonHeight();\nnet.minecraft.world.HorizonReference');
  assert.equal(transformText('Horizon copyright', 'LICENSE', new Set()), 'Horizon copyright');
  assert.equal(transformText('crates/horizon-theme', 'THIRD_PARTY_NOTICES.md', new Set()), 'crates/horizon-theme');
  assert.equal(transformText('net.minecraft.resources.Identifier.parse("horizon:custom")', 'Oracle.java', new Set()), 'net.minecraft.resources.Identifier.parse("atlas:custom")');
});
test('mathematical horizontal names and external resource keys are not branding', () => {
  const text = 'Horizon HORIZONTAL Horizontal horizontal horizontally isHorizontalCollisionMinor minecraft:half_horizontal {"horizontal":4}';
  assert.equal(transformText(text, 'source.rs', new Set()), 'Atlas HORIZONTAL Horizontal horizontal horizontally isHorizontalCollisionMinor minecraft:half_horizontal {"horizontal":4}');
});
test('dry run is non-mutating and excludes only dependencies, caches, resources and symlinks', t => {
  const { workspace, source, write } = fixture(t);
  write('crates/horizon-client/Cargo.toml', '[package]\nname = "horizon-client"\nversion = "0.1.0"\n');
  write('crates/horizon-client/src/registry/mod.rs', 'use horizon_core;');
  write('third_party/dep/source.rs', 'Horizon');
  write('.local/horizon-client/file.rs', 'Horizon');
  write('.githooks/pre-push', '#!/bin/sh\ncargo run -p horizon-client\n');
  write('LICENSE', 'Horizon copyright');
  const plan = inventory(source, workspace);
  assert.equal(plan.localPackages.length, 1);
  assert.equal(plan.edits.some(edit => edit.file.includes('registry')), true);
  assert.equal(plan.edits.some(edit => edit.file.includes('pre-push')), true);
  assert.equal(plan.edits.some(edit => edit.file.includes('third_party')), false);
  assert.equal(fs.readFileSync(path.join(source, 'crates/horizon-client/src/registry/mod.rs'), 'utf8'), 'use horizon_core;');
  assert.throws(() => inventory(workspace, workspace), /escapes/);
});
test('apply renames bottom-up, preserves registry lock entries, and makes an edit backup', t => {
  const { workspace, source, write } = fixture(t);
  write('crates/horizon-client/Cargo.toml', '[package]\nname = "horizon-client"\nversion = "0.1.0"\n');
  write('crates/horizon-client/src/horizon.rs', 'use horizon_client;\n');
  write('Cargo.lock', '[[package]]\nname = "horizon-client"\ndependencies = ["horizon-third-party"]\n[[package]]\nname = "horizon-third-party"\nsource = "registry+https://crates.io"\nchecksum = "abcdef"\n');
  const plan = inventory(source, workspace);
  const backup = applyPlan(plan);
  assert.equal(fs.readFileSync(path.join(source, 'crates/atlas-client/src/atlas.rs'), 'utf8'), 'use atlas_client;\n');
  const lock = fs.readFileSync(path.join(source, 'Cargo.lock'), 'utf8');
  assert.match(lock, /name = "atlas-client"/);
  assert.match(lock, /name = "horizon-third-party"/);
  assert.match(lock, /checksum = "abcdef"/);
  assert.equal(fs.readFileSync(path.join(backup, 'files/crates/horizon-client/src/horizon.rs'), 'utf8'), 'use horizon_client;\n');
  assert.equal(fs.existsSync(path.join(backup, 'manifest.json')), true);
});
test('collisions and source changes prevent apply before any source mutation', t => {
  const { workspace, source, write } = fixture(t);
  write('horizon.txt', 'Horizon'); write('atlas.txt', 'Atlas');
  const plan = inventory(source, workspace);
  assert.equal(plan.collisions.length, 1);
  assert.throws(() => applyPlan(plan), /collisions/);
  assert.equal(fs.existsSync(path.join(workspace, 'work')), false);
  fs.unlinkSync(path.join(source, 'atlas.txt'));
  const fresh = inventory(source, workspace);
  fs.writeFileSync(path.join(source, 'horizon.txt'), 'user edit');
  assert.throws(() => applyPlan(fresh), /Source changed/);
  assert.equal(fs.readFileSync(path.join(source, 'horizon.txt'), 'utf8'), 'user edit');
});

test('the existing Atlas icon is retained while its obsolete source icon gets an explicit legacy name', t => {
  const { workspace, source, write } = fixture(t);
  write('packaging/icons/atlas.ico', 'Atlas icon');
  write('packaging/icons/horizon.ico', 'Old icon');
  write('build.rs', 'let icon = "packaging/icons/horizon.ico";');
  const plan = inventory(source, workspace);
  assert.deepEqual(plan.collisions, []);
  applyPlan(plan);
  assert.equal(fs.readFileSync(path.join(source, 'packaging/icons/atlas.ico'), 'utf8'), 'Atlas icon');
  assert.equal(fs.readFileSync(path.join(source, 'packaging/icons/atlas-legacy.ico'), 'utf8'), 'Old icon');
  assert.equal(fs.readFileSync(path.join(source, 'build.rs'), 'utf8'), 'let icon = "packaging/icons/atlas.ico";');
});
