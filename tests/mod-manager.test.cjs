const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {ModManager,PORTS,registry}=require('../lib/mod-manager.cjs');
function fixture(t,options={}){const root=fs.mkdtempSync(path.join(os.tmpdir(),'atlas-mods-test-')),dataRoot=path.join(root,'data');t.after(()=>fs.rmSync(root,{recursive:true,force:true}));for(const id of ['a','b'])fs.mkdirSync(path.join(dataRoot,'instances',id),{recursive:true});const store={dataRoot,assertInstanceAvailable:id=>{assert(['a','b'].includes(id));},managedDirectory:(...parts)=>path.join(dataRoot,...parts)};return{root,store,manager:new ModManager(store,options),home:id=>path.join(dataRoot,'instances',id)};}
const index={schema:1,projects:[{id:'zoom',kind:'native-mod',name:'Zoom',versions:[{version:'2.0',api:'native-1/test'}]},{id:'other',kind:'native-mod',name:'Other'},{id:'constructor',kind:'native-mod',name:'Untrusted'}]};
test('ports install, disable, reenable and remove per instance, preserving foreign flags',t=>{const f=fixture(t);fs.writeFileSync(path.join(f.home('a'),'atlas-mods.txt'),'mod.other:true\nkey.other.action:key.keyboard.v\n');assert.equal(f.manager.snapshot('a').ports.length,26);f.manager.change('a','atlas',PORTS[0].id,'install');assert(f.manager.snapshot('a').ports[0].enabled);assert(!f.manager.snapshot('b').ports[0].installed);f.manager.change('a','atlas',PORTS[0].id,'disable');assert(f.manager.snapshot('a').ports[0].installed);assert(!f.manager.snapshot('a').ports[0].enabled);f.manager.change('a','atlas',PORTS[0].id,'enable');assert(f.manager.snapshot('a').ports[0].enabled);f.manager.change('a','atlas',PORTS[0].id,'remove');assert(!f.manager.snapshot('a').ports[0].installed);assert.match(fs.readFileSync(path.join(f.home('a'),'atlas-mods.txt'),'utf8'),/key.other.action:key.keyboard.v/);});
test('catalogue is lazy, cached, and unavailable native packages never pretend to install',async t=>{let requests=0;const f=fixture(t,{fetch:async()=>{requests++;return new Response(JSON.stringify(index));}});assert.equal(requests,0);await f.manager.refresh();assert.equal(requests,1);assert.equal(f.manager.snapshot('a').horizon.length,3);f.manager.change('a','horizon','zoom','install');assert(f.manager.snapshot('a').ports.find(p=>p.id==='atlas-zoomify').installed);assert.throws(()=>f.manager.change('a','horizon','other','install'),/адаптация/);assert.throws(()=>f.manager.change('a','horizon','constructor','install'),/адаптация/);const offline=new ModManager(f.store,{fetch:async()=>{throw Error('offline');}});await offline.refresh();assert.equal(offline.projects.length,3);assert(offline.error);});
test('invalid selections and corrupt configuration never replace user data',t=>{const f=fixture(t),file=path.join(f.home('a'),'atlas-ports.json');assert.throws(()=>f.manager.change('a','atlas','../../world','install'));fs.writeFileSync(file,'not JSON');assert(f.manager.snapshot('a').error);assert.throws(()=>f.manager.change('a','atlas',PORTS[0].id,'install'));assert.equal(fs.readFileSync(file,'utf8'),'not JSON');assert.throws(()=>registry({schema:99,projects:[]}));});
test('linked instance folders are refused',t=>{const f=fixture(t),outside=path.join(f.root,'outside');fs.mkdirSync(outside);fs.renameSync(f.home('a'),path.join(f.root,'old-a'));fs.symlinkSync(outside,f.home('a'),process.platform==='win32'?'junction':'dir');assert.throws(()=>f.manager.change('a','atlas',PORTS[0].id,'install'),/links/);assert.deepEqual(fs.readdirSync(outside),[]);});


test('every new native port installs independently without touching instance data',t=>{
  const f=fixture(t);
  const worlds=path.join(f.home('a'),'saves','kept-world');fs.mkdirSync(worlds,{recursive:true});fs.writeFileSync(path.join(worlds,'level.dat'),'world sentinel');
  for(const port of PORTS.slice(6)){
    f.manager.change('a','atlas',port.id,'install');
    const row=f.manager.snapshot('a').ports.find(p=>p.id===port.id);assert(row.installed&&row.enabled,port.id);
    assert(!f.manager.snapshot('b').ports.find(p=>p.id===port.id).installed,port.id);
  }
  const installed=JSON.parse(fs.readFileSync(path.join(f.home('a'),'atlas-ports.json'),'utf8'));
  assert.equal(installed.installed.length,20);assert.equal(new Set(installed.installed).size,20);
  assert.equal(fs.readFileSync(path.join(worlds,'level.dat'),'utf8'),'world sentinel');
  assert.deepEqual(fs.readdirSync(f.home('a')).sort(),['atlas-mods.txt','atlas-ports.json','saves']);
});
test('native catalogue uses supplied metadata versions and describes the real CullParticles behavior',()=>{
  assert.equal(PORTS.length,26);assert.equal(new Set(PORTS.map(p=>p.id)).size,26);
  for(const p of PORTS){assert.match(p.id,/^atlas-[a-z0-9-]+$/);assert(p.description&&p.descriptionEn&&p.scope&&p.scopeEn&&p.version);if(p.config)assert(p.config.startsWith('config/'))}
  assert.equal(PORTS.find(p=>p.id==='atlas-lootbeams').version,'1.1.0');
  assert.equal(PORTS.find(p=>p.id==='atlas-dropall').version,'1.0.0');
  assert.equal(PORTS.find(p=>p.id==='atlas-sortmod').version,'1.0.0');
  assert.match(PORTS.find(p=>p.id==='atlas-cullparticles').descriptionEn,/push-out-of-blocks/);
  assert.match(PORTS.find(p=>p.id==='atlas-cullparticles').scopeEn,/does not optimize particles/);
  assert.equal(PORTS.find(p=>p.id==='atlas-appleskin').config,'config/appleskin.json5');
});
