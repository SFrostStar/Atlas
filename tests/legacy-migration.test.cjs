const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { migrateLegacyData } = require('../lib/legacy-migration.cjs');

function fixture(t) {
  const projectRoot = fs.mkdtempSync(path.join(os.tmpdir(),'atlas-legacy-test-'));
  const dataRoot = path.join(projectRoot,'data'); fs.mkdirSync(dataRoot);
  const write = (relative,content) => { const file=path.join(projectRoot,relative); fs.mkdirSync(path.dirname(file),{recursive:true}); fs.writeFileSync(file,content); return file; };
  t.after(()=>{
    if (path.dirname(projectRoot)!==os.tmpdir() || !path.basename(projectRoot).startsWith('atlas-legacy-test-')) throw new Error('Unsafe cleanup');
    fs.rmSync(projectRoot,{recursive:true,force:true});
  });
  return {projectRoot,dataRoot,write};
}
test('settings and mod directories migrate with verified backups, preserving worlds and arbitrary content', t=>{
  const {projectRoot,dataRoot,write}=fixture(t);
  write('data/instances/atlas/horizon-player.txt','name:Player12345\n');
  write('data/instances/atlas/horizon-extras.txt','discordRichPresence:true\ncustom:unchanged\n');
  write('data/instances/atlas/horizon-mods/example/file.wasm','mod-data');
  write('data/instances/atlas/saves/My Horizon World/level.dat','world-must-stay');
  write('data/sessions/window-id/home/horizon-graphics.txt','graphics:unchanged');
  write('horizon-extras.txt','root-settings');
  const result=migrateLegacyData({dataRoot,projectRoot});
  assert.equal(result.renamed.length,5);
  assert.equal(result.skipped.length,0);
  assert.equal(fs.readFileSync(path.join(dataRoot,'instances/atlas/atlas-player.txt'),'utf8'),'name:Player12345\n');
  assert.equal(fs.readFileSync(path.join(dataRoot,'instances/atlas/atlas-mods/example/file.wasm'),'utf8'),'mod-data');
  assert.equal(fs.readFileSync(path.join(dataRoot,'instances/atlas/saves/My Horizon World/level.dat'),'utf8'),'world-must-stay');
  assert.equal(fs.readFileSync(path.join(result.backupRoot,'data/instances/atlas/horizon-extras.txt'),'utf8'),'discordRichPresence:true\ncustom:unchanged\n');
  assert.equal(fs.existsSync(path.join(dataRoot,'instances/atlas/horizon-player.txt')),false);
  const repeated=migrateLegacyData({dataRoot,projectRoot});
  assert.equal(repeated.changed,false); assert.deepEqual(repeated.messages,[]);
});
test('different destination files and existing mod directories stay intact, with explicit conflict reports', t=>{
  const {projectRoot,dataRoot,write}=fixture(t);
  const old=write('data/instances/atlas/horizon-extras.txt','old');
  const current=write('data/instances/atlas/atlas-extras.txt','current');
  write('data/instances/atlas/horizon-mods/old.wasm','old-mod');
  write('data/instances/atlas/atlas-mods/new.wasm','new-mod');
  const result=migrateLegacyData({dataRoot,projectRoot});
  assert.equal(result.conflicts.length,2); assert.equal(result.changed,false);
  assert.equal(fs.readFileSync(old,'utf8'),'old'); assert.equal(fs.readFileSync(current,'utf8'),'current');
  assert.equal(result.messages.filter(item=>item.code==='conflict').length,2);
  const backupsBefore=fs.readdirSync(path.join(projectRoot,'work/legacy-data-backup/data/instances/atlas'));
  migrateLegacyData({dataRoot,projectRoot});
  assert.deepEqual(fs.readdirSync(path.join(projectRoot,'work/legacy-data-backup/data/instances/atlas')),backupsBefore);
});
test('identical duplicate settings archive only the legacy copy', t=>{
  const {projectRoot,dataRoot,write}=fixture(t);
  const old=write('data/instances/atlas/horizon-graphics.txt','same');
  const current=write('data/instances/atlas/atlas-graphics.txt','same');
  const result=migrateLegacyData({dataRoot,projectRoot});
  assert.equal(result.archived.length,1); assert.equal(fs.existsSync(old),false);
  assert.equal(fs.readFileSync(current,'utf8'),'same');
  assert.equal(fs.readFileSync(result.archived[0].to,'utf8'),'same');
});
test('symlinked files and subdirectories are never traversed or moved', t=>{
  const {projectRoot,dataRoot,write}=fixture(t);
  const outside=path.join(projectRoot,'outside'); fs.mkdirSync(outside);
  fs.writeFileSync(path.join(outside,'secret.txt'),'untouched');
  write('data/instances/atlas/horizon-mods/own.txt','own');
  fs.symlinkSync(outside,path.join(dataRoot,'instances/atlas/horizon-mods/link'),process.platform==='win32'?'junction':'dir');
  const result=migrateLegacyData({dataRoot,projectRoot});
  assert.ok(result.skipped.length);
  assert.equal(result.changed,false);
  assert.equal(fs.readFileSync(path.join(outside,'secret.txt'),'utf8'),'untouched');
  assert.equal(fs.existsSync(path.join(dataRoot,'instances/atlas/horizon-mods/own.txt')),true);
});
test('migration rejects relative roots and does not follow a linked backup directory', t=>{
  const {projectRoot,dataRoot,write}=fixture(t);
  assert.throws(()=>migrateLegacyData({dataRoot:'relative'}),/absolute/);
  write('data/instances/atlas/horizon-player.txt','name:Player123\n');
  const outside=path.join(projectRoot,'outside');fs.mkdirSync(outside);
  fs.mkdirSync(path.join(projectRoot,'work'));
  fs.symlinkSync(outside,path.join(projectRoot,'work/legacy-data-backup'),process.platform==='win32'?'junction':'dir');
  const result=migrateLegacyData({dataRoot,projectRoot});
  assert.ok(result.skipped.length); assert.equal(result.changed,false);
  assert.deepEqual(fs.readdirSync(outside),[]);
});
