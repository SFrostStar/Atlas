const {_electron}=require('playwright-core');const assert=require('node:assert/strict');const path=require('node:path');
const root=path.resolve(__dirname,'..');
(async()=>{const app=await _electron.launch({executablePath:path.join(root,'node_modules/electron/dist/electron.exe'),args:[root],cwd:root,env:{...process.env,ATLAS_GUI_TEST:'1'}});try{
 const page=await app.firstWindow();await page.waitForSelector('#boot-splash',{state:'detached'});
 const initial=await page.evaluate(()=>window.atlas.getState());
 await app.evaluate(({ipcMain,BrowserWindow},initial)=>{
  const fake={...initial,game:{running:false,count:0,sessions:[]},install:{ready:false,needsSetup:false,status:'idle',phase:'idle',active:false,canCancel:true}};
  globalThis.__atlasSetupTest={state:fake,launches:0,mode:'success'};
  const emit=()=>BrowserWindow.getAllWindows()[0].webContents.send('atlas:game-event',structuredClone(fake));
  for(const channel of ['get-state','launch','start-install','cancel-install'])ipcMain.removeHandler('atlas:'+channel);
  ipcMain.handle('atlas:get-state',()=>fake);
  ipcMain.handle('atlas:launch',()=>{if(!fake.install.ready)fake.install.needsSetup=true;else globalThis.__atlasSetupTest.launches++;return fake;});
  ipcMain.handle('atlas:start-install',()=>{
   fake.install={...fake.install,status:'running',phase:'download',active:true,progress:0.4,completed:4,total:10,error:null};
   setTimeout(()=>{if(!fake.install.active)return;Object.assign(fake.install,globalThis.__atlasSetupTest.mode==='error'?{status:'error',phase:'error',active:false,error:{message:'Downloaded file failed its checksum check.'}}:{ready:true,needsSetup:false,status:'ready',phase:'ready',active:false,progress:1});emit();},1000);
   return fake;
  });
  ipcMain.handle('atlas:cancel-install',()=>{Object.assign(fake.install,{status:'cancelled',phase:'cancelled',active:false});return fake;});
 },initial);
 await page.reload();await page.waitForSelector('#boot-splash',{state:'detached'});
 await page.locator('[data-action="launch"]').click();await page.waitForSelector('#install-dialog[open]');
 assert.equal(await app.evaluate(()=>globalThis.__atlasSetupTest.launches),0);
 await page.locator('[data-action="start-install"]').click();assert.equal(await page.locator('#install-progress').getAttribute('value'),'40');
 await page.waitForSelector('#install-dialog:not([open])',{state:'attached'});
 await page.waitForFunction(()=>!busy);assert.equal(await app.evaluate(()=>globalThis.__atlasSetupTest.launches),1);
 await app.evaluate(()=>{Object.assign(globalThis.__atlasSetupTest.state.install,{ready:false,needsSetup:false,status:'idle',phase:'idle',active:false});globalThis.__atlasSetupTest.mode='error';});
 await page.reload();await page.waitForSelector('#boot-splash',{state:'detached'});await page.locator('[data-action="launch"]').click();await page.locator('[data-action="start-install"]').click();
 await page.waitForSelector('#install-error:not([hidden])');assert.match(await page.locator('#install-error').innerText(),/checksum|целостности/);
 assert.equal(await app.evaluate(()=>globalThis.__atlasSetupTest.launches),1);
 await page.locator('[data-action="start-install"]').click();await page.locator('[data-action="cancel-install"]').click();
 assert.equal(await page.locator('#install-dialog').evaluate(e=>e.open),true);
 await page.locator('[data-action="cancel-install"]').click();
 assert.equal(await page.locator('#install-dialog').evaluate(e=>e.open),false);
 assert.equal(await app.evaluate(()=>globalThis.__atlasSetupTest.launches),1);
 console.log('PASS first-run UI: explicit download offer, real progress state, launch once after ready, error/retry/cancel with no actual downloads or game launch.');
 }finally{await app.close();}})().catch(e=>{console.error(e);process.exitCode=1});
