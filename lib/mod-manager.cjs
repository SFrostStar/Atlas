const fs = require('node:fs');
const path = require('node:path');
const INDEX_URL = 'https://horizon-rust.github.io/horizon-mods/index.json';
const PORTS = Object.freeze([
  {id:'atlas-appleskin',name:'AppleSkin · Atlas',icon:'apple',description:'Голод, насыщение и восстановление от еды в руке.',descriptionEn:'Hunger, saturation and held-food restoration preview.',scope:'Начальный аналог: без прогнозирования здоровья и расширенных подсказок.',scopeEn:'Initial equivalent: no health prediction or extended tooltips.'},
  {id:'atlas-armorindicator',name:'ArmorIndicator · Atlas',icon:'shield',description:'Своя экипированная броня и оставшаяся прочность.',descriptionEn:'Your equipped armor and remaining durability.',scope:'Без показателей брони других игроков.',scopeEn:'Does not show other players’ armor.'},
  {id:'atlas-autosprint',name:'AutoSprint · Atlas',icon:'footprints',description:'Обычный спринт при движении вперёд.',descriptionEn:'Normal sprint while moving forward.',scope:'Сохраняет ограничения голода и движения.',scopeEn:'Respects ordinary hunger and movement checks.'},
  {id:'atlas-noshakehand',name:'NoShakeHand · Atlas',icon:'hand',description:'Убирает инерционное качание руки при повороте камеры.',descriptionEn:'Removes inertial hand sway when turning the camera.',scope:'Удары, броски и использование предметов сохраняются.',scopeEn:'Attack, throwing and item-use animations are preserved.'},
  {id:'atlas-zoomify',name:'Zoomify · Atlas',icon:'scan-search',description:'Плавное приближение 4× при удержании C.',descriptionEn:'Smooth 4× zoom while holding C.',scope:'Клавиша меняется в игровом меню Mods. Базовый аналог Zoomify.',scopeEn:'Rebind in the in-game Mods page. Basic Zoomify equivalent.'},
  {id:'atlas-mentionsound',name:'MentionSound · Atlas',icon:'bell',description:'Звук при упоминании ника в чате.',descriptionEn:'A sound when incoming chat mentions your name.',scope:'Пауза между сигналами — 2 секунды; свои сообщения не пингуют.',scopeEn:'Two-second cooldown; your own messages do not ping.'},
]);
const ADAPTERS = Object.freeze({zoom:'atlas-zoomify',classic_zoom:'atlas-zoomify'});
function checked(root,name){
  const target=path.resolve(root,name),rel=path.relative(root,target);
  if(!rel||path.isAbsolute(rel)||rel==='..'||rel.startsWith('..'+path.sep))throw Error('Unsafe mod path');
  let here=path.resolve(root);
  for(const part of ['',...rel.split(path.sep)]){if(part)here=path.join(here,part);try{if(fs.lstatSync(here).isSymbolicLink())throw Error('Mod paths cannot be links');}catch(e){if(e.code!=='ENOENT')throw e;}}
  return target;
}
function read(file,fallback,max=16384){try{if(fs.statSync(file).size>max)throw Error('Mod configuration is too large');return JSON.parse(fs.readFileSync(file,'utf8'));}catch(e){if(e.code==='ENOENT')return fallback;throw e;}}
function atomic(file,bytes){fs.mkdirSync(path.dirname(file),{recursive:true});const temp=file+'.tmp';if(fs.existsSync(temp)&&fs.lstatSync(temp).isSymbolicLink())throw Error('Unsafe temporary mod path');fs.writeFileSync(temp,bytes);fs.renameSync(temp,file);}
function registry(raw){
  if(raw?.schema!==1||!Array.isArray(raw.projects)||raw.projects.length>10000)throw Error('Unsupported Horizon catalogue');
  return raw.projects.filter(p=>p&&/^[a-z][a-z0-9_-]{0,95}$/.test(p.id)&&typeof p.name==='string'&&['mod','native-mod'].includes(p.kind)).map(p=>({id:p.id,name:p.name.slice(0,100),summary:String(p.summary||'').slice(0,500),kind:p.kind,author:Array.isArray(p.authors)?p.authors.slice(0,5).map(String).join(', '):'',version:String(p.versions?.[0]?.version||''),api:String(p.versions?.[0]?.api||''),page:'https://playhorizon.dev/mods/'+encodeURIComponent(p.id),port:Object.hasOwn(ADAPTERS,p.id)?ADAPTERS[p.id]:null}));
}
class ModManager {
  constructor(store,{fetch=globalThis.fetch}={}){
    this.store=store;this.fetch=fetch;this.projects=[];this.error=null;this.loading=false;this.cache=checked(store.dataRoot,'mod-catalog.json');
    try{const cache=read(this.cache,null,4*1024*1024);if(cache)this.projects=registry(cache);}catch{this.error='Не удалось прочитать кэш каталога.';}
  }
  home(id){this.store.assertInstanceAvailable(id);return this.store.managedDirectory('instances',id);}
  installed(id){const home=this.home(id),config=read(checked(home,'atlas-ports.json'),{schema:1,installed:[]});if(config.schema!==1||!Array.isArray(config.installed))throw Error('Unsupported Atlas port configuration');return new Set(config.installed.filter(id=>PORTS.some(p=>p.id===id)));}
  disabled(id){const file=checked(this.home(id),'atlas-mods.txt');try{if(fs.statSync(file).size>1024*1024)throw Error('Mod settings are too large');return new Set(fs.readFileSync(file,'utf8').split(/\r?\n/).map(l=>l.split(':')).filter(([key,value])=>key.trim().startsWith('mod.')&&value?.trim()==='false').map(([key])=>key.trim().slice(4)));}catch(e){if(e.code==='ENOENT')return new Set();throw e;}}
  snapshot(id){let installed=new Set(),disabled=new Set(),error=this.error;try{installed=this.installed(id);disabled=this.disabled(id);}catch{error='Не удалось прочитать настройки модов сборки.';}return {instanceId:id,loading:this.loading,error,ports:PORTS.map(p=>({...p,installed:installed.has(p.id),enabled:installed.has(p.id)&&!disabled.has(p.id)})),horizon:this.projects.map(p=>({...p,installed:!!p.port&&installed.has(p.port),enabled:!!p.port&&installed.has(p.port)&&!disabled.has(p.port)}))};}
  change(id,source,project,action){
    this.home(id);if(!['install','remove','enable','disable'].includes(action))throw Error('Unknown mod action');
    const port=source==='atlas'?PORTS.find(p=>p.id===project)?.id:source==='horizon'?this.projects.find(p=>p.id===project)?.port:null;
    if(!port)throw Error('Этому моду нужна адаптация и нативная сборка Atlas.');
    const installed=this.installed(id);if(action==='install')installed.add(port);else if(action==='remove')installed.delete(port);else if(!installed.has(port))throw Error('Мод не установлен.');
    const home=this.home(id),flags=checked(home,'atlas-mods.txt');let text='';try{if(fs.statSync(flags).size>1024*1024)throw Error('Mod settings are too large');text=fs.readFileSync(flags,'utf8');}catch(e){if(e.code!=='ENOENT')throw e;}
    const lines=text.split(/\r?\n/).filter(l=>l&&l.split(':')[0].trim()!=='mod.'+port);lines.push('mod.'+port+':'+(!['remove','disable'].includes(action)));
    atomic(flags,lines.join('\n')+'\n');atomic(checked(home,'atlas-ports.json'),JSON.stringify({schema:1,installed:[...installed].sort()},null,2)+'\n');
  }
  async refresh(){
    if(this.loading)return;this.loading=true;this.error=null;
    try{const response=await this.fetch(INDEX_URL,{redirect:'error',signal:AbortSignal.timeout(20000)});if(!response.ok)throw Error('Catalogue request failed');let size=0;const chunks=[];for await(const chunk of response.body){size+=chunk.length;if(size>4*1024*1024)throw Error('Catalogue too large');chunks.push(chunk);}const raw=JSON.parse(Buffer.concat(chunks).toString('utf8'));const projects=registry(raw);atomic(this.cache,JSON.stringify(raw));this.projects=projects;}
    catch{this.error='Каталог Horizon недоступен. Показана сохранённая копия.';}finally{this.loading=false;}
  }
}
module.exports={ModManager,PORTS,registry,checked};
