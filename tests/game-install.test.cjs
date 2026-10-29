const {test}=require('node:test'),assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto');
const {GameInstallManager,safeUrl,relative}=require('../lib/game-install.cjs');
const hash=b=>crypto.createHash('sha1').update(b).digest('hex');
function fixture(t,overrides={}){
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'atlas-installer-test-'));t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
 const bundle=path.join(root,'bundle','game'),source=path.join(bundle,'game-root');fs.mkdirSync(source,{recursive:true});
 for(const name of ['atlas-client.exe','atlas-ctl.exe','atlas-prepare.exe','libc++.dll','libunwind.dll','libwinpthread-1.dll'])fs.writeFileSync(path.join(bundle,name),'original payload');
 const bodies=new Map();const item=(url,data)=>{const body=Buffer.from(data);bodies.set(url,body);return {url,sha1:hash(body),size:body.length}};
 const client=item('https://piston-data.mojang.com/client','client'),server=item('https://piston-data.mojang.com/server','server');
 const asset=item('https://resources.download.minecraft.net/aa/asset','sound');
 const assetInfo={hash:asset.sha1,size:asset.size};bodies.set(`https://resources.download.minecraft.net/${asset.sha1.slice(0,2)}/${asset.sha1}`,Buffer.from('sound'));
 const assetIndex=item('https://piston-meta.mojang.com/assets',JSON.stringify({objects:{sound:assetInfo}}));assetIndex.id='fake';
 const javaFile=item('https://piston-data.mojang.com/java','java');
 const javaManifest=item('https://piston-meta.mojang.com/java-manifest',JSON.stringify({files:{'bin/java.exe':{type:'file',downloads:{raw:javaFile}}}}));
 bodies.set('https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json',Buffer.from(JSON.stringify({'windows-x64':{'java-runtime-epsilon':[{manifest:javaManifest}]}})));
 const manifest=Buffer.from(JSON.stringify({id:'26.3',downloads:{client,server},libraries:[],assetIndex,javaVersion:{component:'java-runtime-epsilon'}}));
 const pins={version:'26.3',manifest_sha1:hash(manifest),client_sha1:client.sha1,server_sha1:server.sha1,java_major:25};
 bodies.set(`https://piston-meta.mojang.com/v1/packages/${pins.manifest_sha1}/26.3.json`,manifest);
 fs.writeFileSync(path.join(source,'research-lock.json'),JSON.stringify({minecraft:pins}));
 let requests=0;const updates=[];
 const manager=new GameInstallManager({projectRoot:root,dataRoot:path.join(root,'data'),onChange:s=>updates.push(s),fetch:async url=>{requests++;const body=bodies.get(url);return new Response(body||'missing',{status:body?200:404});},runPreparer:async({args})=>{
  const preparing=manager.snapshot();
  assert.equal(preparing.phase,'prepare');
  assert.equal(preparing.progress,null,'native preparation has indeterminate progress');
  for(const key of ['completed','total','bytesDone','bytesTotal'])assert.equal(preparing[key],0,`preparation must clear stale download ${key}`);
  const preparation=args[args.indexOf('--root')+1],result=path.join(preparation,'.local','atlas-client','26.3');
  fs.mkdirSync(path.join(result,'assets'),{recursive:true});fs.mkdirSync(path.join(result,'data'),{recursive:true});fs.writeFileSync(path.join(result,'facts.json'),'{}');fs.writeFileSync(path.join(result,'atlas-resources.json'),JSON.stringify({schema:7,minecraft:'26.3'}));
 },...overrides});
 return {root,manager,updates,bodies,get requests(){return requests}};
}
test('first-run installs bundled client, verified official resources and Java, then publishes valid resources',async t=>{
 const f=fixture(t);assert.equal(f.manager.inspect().ready,false);await f.manager.start();assert.equal(f.manager.inspect().ready,true);
 assert.equal(f.manager.snapshot().status,'ready');assert.ok(f.requests>4);assert.ok(f.updates.some(s=>s.phase==='prepare'));assert.ok(f.updates.some(s=>s.phase==='download'));
 const publishing=f.updates.find(s=>s.phase==='install'&&s.stage==='install');
 assert.ok(publishing,'publishing phase is observable');assert.equal(publishing.progress,null);
 for(const key of ['completed','total','bytesDone','bytesTotal'])assert.equal(publishing[key],0,`publishing must clear stale download ${key}`);
 const requests=f.requests;await f.manager.start();assert.equal(f.requests,requests,'ready installations never download again');
 assert.equal(fs.existsSync(path.join(f.root,'data','instances')),false);
});
test('unsafe URLs and path traversal are rejected before fetch',()=>{
 for(const url of ['http://piston-data.mojang.com/a','https://evil.example/a','https://piston-data.mojang.com.evil/a','https://user:pass@piston-data.mojang.com/a'])assert.throws(()=>safeUrl(url));
 for(const p of ['../x','/x','C:/x','x\\y','a//b'])assert.throws(()=>relative(p));
});
test('corrupt download fails checksum and never publishes resources',async t=>{
 const f=fixture(t);f.bodies.set('https://piston-data.mojang.com/client',Buffer.from('broken'));
 await assert.rejects(f.manager.start(),/declared size|checksum/);assert.equal(f.manager.inspect().ready,false);assert.equal(f.manager.snapshot().status,'error');
});
test('cancel is observable, keeps verified files and can retry',async t=>{
 const f=fixture(t,{onChange:s=>{if(s.phase==='download'&&s.completed===1)f.manager.cancel();}});
 await f.manager.start();assert.equal(f.manager.snapshot().status,'cancelled');assert.equal(f.manager.inspect().ready,false);
 f.manager.onChange=()=>{};await f.manager.start();assert.equal(f.manager.inspect().ready,true);
});
