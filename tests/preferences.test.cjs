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
  const runtime=path.join(store.projectRoot,'runtime','atlas-client.exe');
  const resources=path.join(path.dirname(runtime),'.local','atlas-client','26.3');
  for(const p of ['assets','data'])fs.mkdirSync(path.join(resources,p),{recursive:true});
  fs.writeFileSync(runtime,'fixture');store.saveSettings({runtimePath:runtime});
  store.setInstanceMemory('atlas',2048);
  assert.equal(buildLaunchSpec(store,'atlas').env.ATLAS_MEMORY_MB,'2048');
  assert.equal(buildLaunchSpec(store,other.id).env.ATLAS_MEMORY_MB,'0');
  assert.equal(new LauncherStore(store.projectRoot).requireInstance('atlas').memoryMb,2048);
  for(const n of [-1,1,2049,Infinity,Number.MAX_SAFE_INTEGER,'4096',null])assert.throws(()=>store.setInstanceMemory('atlas',n));
  store.setInstanceMemory('atlas',0);assert.equal(buildLaunchSpec(store,'atlas').env.ATLAS_MEMORY_MB,'0');
});
test('locale lookup leaves custom names unchanged and translates complete UI labels',()=>{
  const {t}=require('../i18n.js');
  assert.equal(t('Мои сборки','en'),'Instances');
  assert.equal(t('Настройки','ru'),'Настройки');
  assert.equal(t('Мой сервер Atlas','en'),'Мой сервер Atlas');
  assert.equal(t('Запущено окон: 3','en'),'Running windows: 3');
});
