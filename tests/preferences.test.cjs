const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const {LauncherStore,buildLaunchSpec}=require('../lib/launcher-store.cjs');
function fixture(t){
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'atlas-preferences-'));
  t.after(()=>{if(path.dirname(root)!==os.tmpdir())throw new Error('unsafe cleanup');fs.rmSync(root,{recursive:true,force:true});});
  return new LauncherStore(root,{appData:path.join(root,'appdata')});
}
test('appearance defaults migrate without rewriting older instance data; changes persist',t=>{
  const store=fixture(t);
  assert.equal(store.getState().settings.language,'ru');
  store.saveSettings({language:'en',theme:'light',accentColor:'blue',heroStyle:'sunset',showHeroText:false,reducedMotion:true});
  const reloaded=new LauncherStore(store.projectRoot).getState();
  assert.equal(reloaded.settings.language,'en');
  assert.equal(reloaded.settings.theme,'light');
  assert.equal(reloaded.settings.showHeroText,false);
  assert.equal(reloaded.settings.reducedMotion,true);
  for(const patch of [{language:'de'},{theme:'auto'},{accentColor:'url(x)'},{heroStyle:'custom'},{showHeroText:'false'},{reducedMotion:1}])assert.throws(()=>store.saveSettings(patch));
  assert.equal(store.getState().settings.language,'en');
});
test('memory budgets are validated, per instance, persisted and passed only to child',t=>{
  const store=fixture(t);
  const other=store.createInstance({name:'Other'});
  assert.equal(store.requireInstance('atlas').memoryMb,4096);
  assert.equal(other.memoryMb,4096);
  assert.deepEqual(store.getState().memory,{totalMb:Math.floor(os.totalmem()/1024**2),minimumMb:1024,maximumMb:32768});
  const runtime=path.join(store.projectRoot,'runtime','atlas-client.exe');
  const resources=path.join(path.dirname(runtime),'.local','atlas-client','26.3');
  for(const p of ['assets','data'])fs.mkdirSync(path.join(resources,p),{recursive:true});
  fs.writeFileSync(runtime,'fixture');store.saveSettings({runtimePath:runtime});
  const parentBudget=process.env.ATLAS_MEMORY_MB;
  for(const memoryMb of [1024,2048,8192,32768]){
    store.setInstanceMemory('atlas',memoryMb);
    assert.equal(buildLaunchSpec(store,'atlas').env.ATLAS_MEMORY_MB,String(memoryMb));
    assert.equal(new LauncherStore(store.projectRoot).requireInstance('atlas').memoryMb,memoryMb);
  }
  assert.equal(buildLaunchSpec(store,other.id).env.ATLAS_MEMORY_MB,'4096');
  assert.equal(process.env.ATLAS_MEMORY_MB,parentBudget);
  for(const n of [-1,0,1,1023,2049,33024,33792,NaN,Infinity,Number.MAX_SAFE_INTEGER,'4096',null])assert.throws(()=>store.setInstanceMemory('atlas',n));
  assert.equal(store.requireInstance('atlas').memoryMb,32768,'invalid values never replace the saved budget');
});

test('old unlimited and missing budgets migrate to finite GB without changing other instance data',t=>{
  const store=fixture(t);
  const cases=[[undefined,4096],[0,4096],[256,1024],[-1,1024],[2304,2048],[8192,8192],[65536,32768]];
  store.config.instances=cases.map(([value],index)=>({
    id:`memory-${index}`,name:`Memory ${index}`,description:'Keep description',accent:'blue',icon:'gem',createdAt:'2026-01-01',lastPlayed:null,
    ...(value===undefined?{}:{memoryMb:value}),customField:'preserved',
  }));
  store.config.selectedId='memory-0';
  const world=path.join(store.dataRoot,'instances','memory-0','saves','fixture.txt');
  fs.mkdirSync(path.dirname(world),{recursive:true});fs.writeFileSync(world,'world bytes unchanged');
  store.save();
  const reloaded=new LauncherStore(store.projectRoot,{appData:path.join(store.projectRoot,'appdata')});
  const saved=JSON.parse(fs.readFileSync(store.file,'utf8'));
  for(const [index,[,expected]] of cases.entries()){
    assert.equal(reloaded.requireInstance(`memory-${index}`).memoryMb,expected);
    assert.deepEqual(saved.instances[index],{...store.config.instances[index],memoryMb:expected});
  }
  assert.equal(saved.selectedId,'memory-0');
  assert.equal(fs.readFileSync(world,'utf8'),'world bytes unchanged');
  assert.deepEqual(new LauncherStore(store.projectRoot).config.instances,saved.instances,'migration is stable across further launches');
});
test('locale lookup leaves custom names unchanged and translates complete UI labels',()=>{
  const {t}=require('../i18n.js');
  assert.equal(t('Мои сборки','en'),'Instances');
  assert.equal(t('Настройки','ru'),'Настройки');
  assert.equal(t('Мой сервер Atlas','en'),'Мой сервер Atlas');
  assert.equal(t('Запущено окон: 3','en'),'Running windows: 3');
});
