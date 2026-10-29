// Developer check: compile the real pack/cache modules against existing fast-build deps.
// Uses only generated temporary fixtures; no player's worlds or prepared resources.
const fs = require('node:fs');
const path = require('node:path');
const cp = require('node:child_process');
const root = path.resolve(__dirname, '..');
const cachedEngine = path.join(root, 'engine/atlas');
const engine = process.env.ATLAS_ENGINE_ROOT ? path.resolve(process.env.ATLAS_ENGINE_ROOT) : cachedEngine;
const source = path.join(engine, fs.existsSync(path.join(engine, 'crates/atlas-client-render/src/packs.rs')) ? 'crates/atlas-client-render/src' : 'crates/atlas-client/src');
const cacheRoot = process.env.ATLAS_LOADING_DEPS_ROOT ? path.resolve(process.env.ATLAS_LOADING_DEPS_ROOT) : cachedEngine;
const deps = path.join(cacheRoot, 'target/fast/deps');
const scratch = fs.mkdtempSync(path.join(root, 'work/loading-unit-'));
const rust = path.join(scratch, 'probe.rs');
fs.cpSync(path.join(source, 'packs'), path.join(scratch, 'packs'), {recursive:true});
fs.writeFileSync(path.join(scratch,'packs.rs'),fs.readFileSync(path.join(source,'packs.rs'),'utf8').replace('mod cache;','#[path="'+path.join(source,'packs/cache.rs').replaceAll('\\','/')+'"] mod cache;'));
const exe = path.join(scratch, 'probe.exe');
const unix = file => file.replaceAll('\\', '/');
const loading = path.join(source, 'atlas/loading/mod.rs');
fs.writeFileSync(rust, `pub use atlas_client::{text,zip,pack_screen};
#[path="${unix(path.join(source, 'sound_events.rs'))}"] mod sound_events;
#[path="${unix(path.join(scratch, 'packs.rs'))}"] mod packs;
${fs.existsSync(loading) ? `mod atlas { #[path="${unix(loading)}"] pub mod loading; }` : ''}
`);
const env = { ...process.env, CARGO_HOME: path.join(root, 'tools/cargo'), RUSTUP_HOME: path.join(root, 'tools/rustup') };
const llvm = fs.readdirSync(path.join(root, 'tools')).find(n => n.startsWith('llvm-mingw-') && n.endsWith('ucrt-x86_64'));
env.PATH = path.join(root, 'tools', llvm, 'bin') + ';' + path.join(env.CARGO_HOME, 'bin') + ';' + env.PATH;
const args = ['--edition=2024', '--test', rust, '-A', 'dead_code', '-L', 'dependency=' + deps, '-o', exe];
const client=fs.readdirSync(deps).filter(n=>n.startsWith('libatlas_client-')&&n.endsWith('.rlib')).sort((a,b)=>fs.statSync(path.join(deps,b)).mtimeMs-fs.statSync(path.join(deps,a)).mtimeMs)[0];
const fingerprints=path.join(cacheRoot,'target/fast/.fingerprint');
const clientHash=client.slice('libatlas_client-'.length,-'.rlib'.length);
const fingerprint=fs.readFileSync(path.join(fingerprints,'atlas-client-'+clientHash,'lib-atlas_client.json'),'utf8');
const selected=new Map();
for(const match of fingerprint.matchAll(/\[\d+,"([^"]+)",false,(\d+)\]/g)) {
 const bytes=BigInt(match[2]).toString(16).padStart(16,'0').match(/../g).reverse().join('');
 for(const folder of fs.readdirSync(fingerprints)) {
  const marker=path.join(fingerprints,folder,'lib-'+match[1]);
  if(fs.existsSync(marker)&&fs.readFileSync(marker,'utf8').trim()===bytes)selected.set(match[1],folder.slice(folder.lastIndexOf('-')+1));
 }
}
for (const name of ['atlas_client','atlas_client_core','atlas_core','serde','serde_json','sha2','junction','miniz_oxide','zip']) {
  const files = fs.readdirSync(deps).filter(n => n.startsWith('lib' + name + '-') && n.endsWith('.rlib'))
    .sort((a,b) => fs.statSync(path.join(deps,b)).mtimeMs - fs.statSync(path.join(deps,a)).mtimeMs);
  if (!files.length) throw Error(`Build atlas-client first: missing ${name}`);
  const selectedFile=name==='atlas_client'?client:selected.has(name)?'lib'+name+'-'+selected.get(name)+'.rlib':files[0];
  args.push('--extern', name + '=' + path.join(deps, selectedFile));
}
for (const name of ['windows_x86_64_gnullvm-0.53.1','windows_x86_64_gnullvm-0.52.6']) {
  args.push('-L','native=' + path.join(root,'tools/cargo/registry/src/index.crates.io-1949cf8c6b5b557f',name,'lib'));
}
const built = cp.spawnSync(path.join(env.CARGO_HOME,'bin/rustc.exe'), args, {env,windowsHide:true,encoding:'utf8'});
if (built.status !== 0) throw Error(built.stderr);
const tested = cp.spawnSync(exe, ['--test-threads=1'], {env,windowsHide:true,encoding:'utf8'});
process.stdout.write(tested.stdout);
process.stderr.write(tested.stderr);
process.exitCode = tested.status;
