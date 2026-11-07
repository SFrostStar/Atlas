// Native client integration checks. Every run owns a fresh, disposable profile.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict');
const {promisify}=require('node:util'),execFile=promisify(cp.execFile);
const {ModManager,PORTS}=require('../lib/mod-manager.cjs');
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
(async()=>{
  const root=path.resolve(__dirname,'..'),runtime=path.resolve(process.argv[2]||path.join(root,'work/runtime-mod-ports-complete'));
  const scratch=fs.mkdtempSync(path.join(root,'work/native-mod-qa-')),home=path.join(scratch,'data/instances/test'),socket=path.join(home,'control.json');
  fs.mkdirSync(home,{recursive:true});
  const store={dataRoot:path.join(scratch,'data'),assertInstanceAvailable:id=>assert.equal(id,'test'),managedDirectory:(...p)=>path.join(scratch,'data',...p)};
  const manager=new ModManager(store);for(const p of PORTS)manager.change('test','atlas',p.id,'install');
  fs.writeFileSync(path.join(home,'atlas-extras.txt'),'agentControl:true\nnoAi:false\n');
  fs.writeFileSync(path.join(home,'options.txt'),'lang:en_us\nmaxFps:30\nrenderDistance:2\nsimulationDistance:2\n');
  const env={...process.env,ATLAS_HOME:home,ATLAS_LAUNCHER_HOME:home,XDG_DATA_HOME:home,XDG_CONFIG_HOME:home,ATLAS_NO_AUDIO:'1',ATLAS_NO_HEARTBEAT:'1',ATLAS_NO_DISCORD:'1',ATLAS_TELEMETRY_URL:'off',ATLAS_MEMORY_MB:'0',ATLAS_LAUNCHER_NO_SELF_UPDATE:'1'};
  const child=cp.spawn(path.join(runtime,'atlas-client.exe'),['--resources',path.join(root,'runtime/.local/atlas-client/26.3'),'--game-dir',home,'--control-socket',socket,'--name','AtlasModsQA','--offscreen','--width','1280','--height','720'],{cwd:home,env,windowsHide:true,stdio:['ignore','pipe','pipe']});
  let log='';child.stdout.on('data',s=>log+=s);child.stderr.on('data',s=>log+=s);
  const closed=new Promise(resolve=>child.once('close',resolve));
  const ctl=async(...args)=>{
    const result=JSON.parse((await execFile(path.join(runtime,'atlas-ctl.exe'),['--socket',socket,...args],{env,windowsHide:true,timeout:120000,maxBuffer:16*1024*1024})).stdout);
    if(result.error)throw Error(JSON.stringify(result));return result;
  };
  const shot=async name=>{await delay(150);await ctl('screenshot',path.join(scratch,name+'.png'));};
  const closeScreens=async()=>{for(let n=0;n<10;n++){if((await ctl('state')).screen==='playing')return;await ctl('key','esc','tap');await delay(100);}throw Error('Screen did not close');};
  const checks=[];
  try{
    const deadline=Date.now()+90000;
    while(!fs.existsSync(socket)||!log.includes('first frame')){if(child.exitCode!==null||Date.now()>deadline)throw Error(log.slice(-3000));await delay(100);}
    await ctl('open','NativeModsQA','54321','creative');await ctl('wait-ready','120');
    await ctl('command','tp @s 0 90 0 0 0');
    await ctl('command','fill -12 89 -12 12 89 12 minecraft:stone');
    await ctl('command','fill -12 90 -12 12 95 12 minecraft:air');
    await ctl('command','gamemode survival');await delay(800);
    await ctl('key','c','down');await delay(1250);
    const base=await ctl('state');assert(Math.abs(base.client_zoom.factor-4)<0.02,'Zoomify initial factor');
    await ctl('scroll-at','200','110','1');await delay(850);
    const scroll=await ctl('state');assert(Math.abs(scroll.client_zoom.factor-6)<0.04,'Zoomify wheel 4x -> 6x');assert.equal(scroll.selected,base.selected,'Zoom wheel must not switch hotbar');
    await shot('01-scroll-zoom');await ctl('key','c','up');await delay(850);assert.equal((await ctl('state')).client_zoom,null);checks.push('Zoomify geometric wheel zoom and release');
    await ctl('key','w','down');await delay(500);assert.equal((await ctl('state')).player.sprinting,true);await ctl('key','w','up');checks.push('AutoSprint');
    await ctl('command','item replace entity @s hotbar.0 with minecraft:stone 64');
    await ctl('command','item replace entity @s hotbar.1 with minecraft:ender_pearl 8');await ctl('select','1');
    for(let n=0;n<30;n++){if((await ctl('inventory')).counts['minecraft:ender_pearl']===8)break;await delay(50);}
    await ctl('mouse','middle','click');await delay(250);const inv=await ctl('inventory');
    assert.equal(inv.selected,1,'Pearl restores selected slot');
    fs.writeFileSync(path.join(scratch,'pearl-inventory.json'),JSON.stringify(inv,null,2));
    const pearl=inv.hotbar.find(s=>s.item==='minecraft:ender_pearl');assert.equal(pearl?.count,7,'Pearl consumes exactly one item');checks.push('MiddleClickPearl');
    await ctl('key','v','tap');await delay(250);assert.equal((await ctl('state')).screen,'mod');await shot('02-viewmodel');await closeScreens();checks.push('ViewModel settings');
    await ctl('key','h','tap');await delay(250);assert.equal((await ctl('state')).screen,'mod');await shot('03-hotcommands');await closeScreens();checks.push('HotCommands settings');
    await ctl('command','radar config');await delay(200);assert.equal((await ctl('state')).screen,'mod');await shot('04-radar-config');await closeScreens();checks.push('Radar editable settings');
    await ctl('command','gamma max');await delay(200);
    const gamma=JSON.parse(fs.readFileSync(path.join(home,'config/gammautils.json'),'utf8'));assert.equal(gamma.gamma.value,15);checks.push('GammaUtils commands/config');
    await ctl('options','mods');await delay(200);const mods=await ctl('screen');fs.writeFileSync(path.join(scratch,'mods-screen.json'),JSON.stringify(mods,null,2));await shot('05-mods');
    assert(JSON.stringify(mods).includes('AppleSkin'));
    await ctl('scroll-at','200','110','-40');await delay(200);const bottom=await ctl('screen');
    fs.writeFileSync(path.join(scratch,'mods-bottom.json'),JSON.stringify(bottom,null,2));
    assert(JSON.stringify(bottom).includes('Gamma Utils'));checks.push('Native mod catalog');
    await closeScreens();await ctl('key','e','tap');await delay(200);await shot('06-inventory');await closeScreens();
    const result={passed:true,checks,scratch};fs.writeFileSync(path.join(scratch,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
  }finally{
    try{await ctl('quit');await closed;}catch{if(child.exitCode===null)child.kill();}
    fs.writeFileSync(path.join(scratch,'game.log'),log);
  }
  assert.equal(child.exitCode,0);assert(!/Validation Error|wgpu error|panicked at/i.test(log),'No GPU/runtime errors');
})().catch(e=>{console.error(e);process.exitCode=1;});
