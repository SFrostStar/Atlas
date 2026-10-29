const {test}=require('node:test');const assert=require('node:assert/strict');
const fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {LauncherStore}=require('../lib/launcher-store.cjs');
const options=require('../instance-options.js');
test('20 safe icons and exact shared instance colors persist through edit/rename without moving worlds',t=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'atlas-edit-test-'));t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
 const store=new LauncherStore(root);assert.equal(options.icons.length,20);assert.equal(new Set(options.icons).size,20);
 const instance=store.createInstance({name:'Before',accent:'blue',icon:'moon'}),home=store.instancePath(instance.id);
 fs.writeFileSync(path.join(home,'saves','world-sentinel'),'keep');
 store.updateInstance(instance.id,{name:'After',accent:'violet',icon:'gem',description:'Custom'});
 const restored=new LauncherStore(root).requireInstance(instance.id);
 assert.equal(restored.name,'After');assert.equal(restored.icon,'gem');assert.equal(restored.accent,'violet');
 assert.equal(store.instancePath(instance.id),home);assert.equal(fs.readFileSync(path.join(home,'saves','world-sentinel'),'utf8'),'keep');
 for(const patch of [{accent:'url(evil)'},{icon:'<script>'},{name:'Atlas'},{description:'\x00'}])assert.throws(()=>store.updateInstance(instance.id,patch));
 assert.equal(store.requireInstance(instance.id).name,'After');
});
