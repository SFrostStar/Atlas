// Native AppleSkin regression: requires prepared resources, built client and Python Pillow.
// Uses only a fresh scratch profile under work/. Run: node scripts/verify-appleskin.cjs [runtime-directory]
const fs=require('fs'),p=require('path'),cp=require('child_process'),{promisify}=require('util'),assert=require('assert/strict');
const {ModManager,PORTS}=require('../lib/mod-manager.cjs'),run=promisify(cp.execFile),sleep=ms=>new Promise(r=>setTimeout(r,ms));
(async()=>{
 const root=p.resolve(__dirname,'..'),scratch=fs.mkdtempSync(p.join(root,'work/appleskin-qa-')),data=p.join(scratch,'data'),home=p.join(data,'instances','test'),runtime=p.join(root,process.argv[2]||'runtime'),socket=p.join(home,'control.json');fs.mkdirSync(home,{recursive:true});
 const store={dataRoot:data,assertInstanceAvailable:id=>assert.equal(id,'test'),managedDirectory:(...parts)=>p.join(data,...parts)};const manager=new ModManager(store);for(const port of PORTS.filter(p=>p.id==='atlas-appleskin'))manager.change('test','atlas',port.id,'install');
 fs.writeFileSync(p.join(home,'atlas-extras.txt'),'agentControl:true\nnoAi:false\n');fs.writeFileSync(p.join(home,'options.txt'),'lang:ru_ru\nmaxFps:30\nrenderDistance:2\nsimulationDistance:2\n');
 const env={...process.env,ATLAS_HOME:home,ATLAS_LAUNCHER_HOME:home,XDG_DATA_HOME:home,XDG_CONFIG_HOME:home,ATLAS_NO_AUDIO:'1',ATLAS_NO_HEARTBEAT:'1',ATLAS_NO_DISCORD:'1',ATLAS_TELEMETRY_URL:'off',ATLAS_MEMORY_MB:'0',ATLAS_LAUNCHER_NO_SELF_UPDATE:'1'};
 const child=cp.spawn(p.join(runtime,'atlas-client.exe'),['--game-dir',home,'--resources',p.join(root,'runtime/.local/atlas-client/26.3'),'--control-socket',socket,'--name','AtlasPortTest','--offscreen','--width','1280','--height','720'],{cwd:home,env,windowsHide:true,stdio:['ignore','pipe','pipe']});let log='';child.stdout.on('data',b=>log+=b);child.stderr.on('data',b=>log+=b);const closed=new Promise(r=>child.once('close',r));
 const ctl=async(...args)=>JSON.parse((await run(p.join(runtime,'atlas-ctl.exe'),['--socket',socket,...args],{env,windowsHide:true,timeout:120000})).stdout);
 try{
  const end=Date.now()+60000;while(!fs.existsSync(socket)||!log.includes('first frame')){if(child.exitCode!==null||Date.now()>end)throw Error(log.slice(-1500));await sleep(75);}
  await ctl('open','PortsTest','12345','creative');await ctl('wait-ready','120');await ctl('command','give @s minecraft:golden_carrot 1');

  await ctl('command','gamemode survival');await ctl('command','fill ~-10 ~-1 ~-10 ~10 ~-1 ~10 minecraft:stone');await ctl('command','fill ~-10 ~ ~-10 ~10 ~4 ~10 minecraft:air');await ctl('options-set','chatVisibility','2');await sleep(1200);await ctl('screenshot',p.join(scratch,'01-hud.png'));fs.writeFileSync(p.join(scratch,'01-state.json'),JSON.stringify(await ctl('state'),null,2));

  await ctl('command','gamerule minecraft:natural_health_regeneration false');
  await ctl('command','effect give @s minecraft:hunger 2 255 true');await sleep(2700);await ctl('command','effect clear @s');
  await ctl('command','damage @s 8 minecraft:generic');await sleep(1300);
  const state=await ctl('state');fs.writeFileSync(p.join(scratch,'hungry-state.json'),JSON.stringify(state,null,2));
  assert(state.player.food<18&&state.player.food>=12,JSON.stringify(state.player));assert(state.player.health<20);await ctl('command','gamerule minecraft:natural_health_regeneration true');await sleep(250);
  for(let i=0;i<20;i++){await ctl('screenshot',p.join(scratch,'held-'+String(i).padStart(2,'0')+'.png'));await sleep(70);}
  await ctl('command','item replace entity @s weapon.mainhand with minecraft:stone');
  await ctl('command','item replace entity @s weapon.offhand with minecraft:golden_carrot');
  for(let i=0;i<20;i++){await ctl('screenshot',p.join(scratch,'offhand-'+String(i).padStart(2,'0')+'.png'));await sleep(70);}
  await ctl('key','e','tap');await sleep(120);const ui=await ctl('ui');fs.writeFileSync(p.join(scratch,'inventory-ui.json'),JSON.stringify(ui,null,2));
  const widgets=ui.widgets||ui.nodes||ui;const slot=Array.isArray(widgets)?widgets.find(n=>n.stack?.item==='minecraft:golden_carrot'):null;
  if(slot){const [x,y,w,h]=slot.bounds;await ctl('scroll-at',String(x+w/2),String(y+h/2),'0');await ctl('screenshot',p.join(scratch,'tooltip.png'));}
  const pixels=await run('python',['-c',`from PIL import Image
from pathlib import Path
import sys,json
p=Path(sys.argv[1]);base=Image.open(p/'01-hud.png').convert('RGB').crop((666,600,916,635))
assert sum(r>220 and g>200 and b<100 for r,g,b in base.getdata())>5,'missing saturation outlines'
result={}
for prefix in ['held','offhand']:
 values=[]
 for f in sorted(p.glob(prefix+'-*.png')):
  image=Image.open(f).convert('RGB').crop((365,601,611,633))
  values.append(sum(r>g*1.7 and r>b*1.7 and r>80 for r,g,b in image.getdata()))
 assert max(values)-min(values)>80,(prefix,'health preview does not blink',values)
 result[prefix]={'minimum':min(values),'maximum':max(values)}
print(json.dumps(result))`,scratch]);console.log(pixels.stdout);
  fs.writeFileSync(p.join(scratch,'result.json'),JSON.stringify({passed:true,scratch,food:state.player.food,health:state.player.health,uiSlot:slot},null,2));console.log(JSON.stringify({scratch,passed:true}));
 }finally{try{await ctl('quit');await closed;}catch{if(child.exitCode===null)child.kill();}fs.writeFileSync(p.join(scratch,'game.log'),log);}
 if(child.exitCode!==0||/Validation Error|wgpu error/i.test(log))throw Error('Game/GPU failed '+child.exitCode);
})().catch(e=>{console.error(e);process.exitCode=1});
