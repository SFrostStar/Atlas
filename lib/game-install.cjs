const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const childProcess = require('node:child_process');
const { once } = require('node:events');

const HOSTS = new Set(['piston-meta.mojang.com', 'piston-data.mojang.com', 'launchermeta.mojang.com', 'launcher.mojang.com', 'libraries.minecraft.net', 'resources.download.minecraft.net']);
const JAVA_INDEX = 'https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json';
const PAYLOAD = ['atlas-client.exe', 'atlas-ctl.exe', 'atlas-prepare.exe', 'libc++.dll', 'libunwind.dll', 'libwinpthread-1.dll'];
const SOURCE_FILES = ['research-lock.json', 'exporters', 'mods/atlas/src/test/java/dev/atlaslab/parity', 'crates/atlas-prepare/src/exporters.rs'];
const cancelError = () => Object.assign(new Error('Installation cancelled. Verified downloads are kept.'), { code: 'cancelled' });
function relative(value) {
  if (typeof value !== 'string' || !value || value.includes('\\') || value.includes(':') || value.includes('\0') || value.split('/').some(p => !p || p === '.' || p === '..')) throw new Error('Unsafe download path.');
  return value;
}
function contained(root, target) {
  const rel = path.relative(path.resolve(root), path.resolve(target));
  return !path.isAbsolute(rel) && rel !== '..' && !rel.startsWith(`..${path.sep}`);
}
function guard(root, target) {
  if (!contained(root, target)) throw new Error('Installation path escapes its folder.');
  let current = path.parse(path.resolve(target)).root;
  for (const component of path.resolve(target).slice(current.length).split(path.sep)) {
    current = path.join(current, component);
    try { if (fs.lstatSync(current).isSymbolicLink()) throw new Error('Installation through a linked folder is not allowed.'); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
  }
  return target;
}
function safeUrl(value) {
  const url = new URL(value);
  if (url.protocol !== 'https:' || !HOSTS.has(url.hostname) || url.username || url.password || (url.port && url.port !== '443')) throw new Error('Untrusted game download address.');
  return url.href;
}
async function sha1(file) {
  const hash = crypto.createHash('sha1');
  for await (const bytes of fs.createReadStream(file)) hash.update(bytes);
  return hash.digest('hex');
}
function fileInfo(value, target) {
  if (!value || !/^[a-f0-9]{40}$/i.test(value.sha1 || '') || (value.size !== undefined && (!Number.isSafeInteger(value.size) || value.size < 0))) throw new Error('Invalid game download checksum or size.');
  return { url: safeUrl(value.url), sha1: value.sha1.toLowerCase(), size: value.size, target };
}
function allowedLibrary(library) {
  if (!library.rules) return true;
  let allowed = false;
  for (const rule of library.rules) {
    if (rule.features && Object.values(rule.features).some(Boolean)) continue;
    if (rule.os?.name && rule.os.name !== 'windows') continue;
    if (rule.os?.arch && rule.os.arch !== 'x86_64' && rule.os.arch !== 'amd64') continue;
    if (rule.os?.version && !new RegExp(rule.os.version).test(require('node:os').release())) continue;
    allowed = rule.action === 'allow';
  }
  return allowed;
}
function prepared(root, version) {
  try {
    const manifest = JSON.parse(fs.readFileSync(path.join(root, 'atlas-resources.json'), 'utf8'));
    return manifest.schema === 7 && manifest.minecraft === version && fs.statSync(path.join(root, 'assets')).isDirectory() && fs.statSync(path.join(root, 'data')).isDirectory() && fs.statSync(path.join(root, 'facts.json')).isFile();
  } catch { return false; }
}

class GameInstallManager {
  constructor({ projectRoot, dataRoot, onChange = () => {}, fetch = globalThis.fetch, spawn = childProcess.spawn, runPreparer, pins, requestTimeoutMs = 120000, jobs = 4 } = {}) {
    if (!path.isAbsolute(projectRoot || '') || !path.isAbsolute(dataRoot || '')) throw new Error('Installer folders must be absolute.');
    this.projectRoot = path.resolve(projectRoot); this.dataRoot = path.resolve(dataRoot);
    this.root = path.join(this.dataRoot, 'install'); this.runtime = path.join(this.projectRoot, 'runtime'); this.bundle = path.join(this.projectRoot, 'bundle', 'game');
    this.onChange = onChange; this.fetch = fetch; this.spawn = spawn; this.runPreparer = runPreparer; this.injectedPins = pins;
    this.requestTimeoutMs = requestTimeoutMs; this.jobs = Math.max(1, Math.min(4, jobs));
    this.state = { status: 'idle', phase: 'idle', active: false, stage: '', progress: null, completed: 0, total: 0, bytesDone: 0, bytesTotal: 0, error: null, canCancel: false };
  }
  snapshot() { return { ...this.state, error: this.state.error ? { ...this.state.error } : null }; }
  update(patch) { Object.assign(this.state, patch); this.onChange(this.snapshot()); }
  checkCancelled() { if (this.controller?.signal.aborted) throw cancelError(); }
  context(options = {}) {
    if (typeof options === 'string') options = { runtimePath: options };
    const roots = [path.join(this.bundle, 'game-root'), options.sourceRoot || path.join(this.projectRoot, 'engine', 'atlas')];
    const source = roots.find(root => fs.existsSync(path.join(root, 'research-lock.json')));
    const pins = this.injectedPins || (source ? JSON.parse(fs.readFileSync(path.join(source, 'research-lock.json'), 'utf8')).minecraft : null);
    return { source, pins, version: pins?.version || '26.3', runtimePath: options.runtimePath || path.join(this.runtime, 'atlas-client.exe') };
  }
  inspect(options = {}, sourceRoot) {
    if (typeof options === 'string') options = { runtimePath: options, sourceRoot };
    const context = this.context(options);
    const candidates = [path.join(path.dirname(context.runtimePath), '.local', 'atlas-client', context.version), path.join(this.runtime, '.local', 'atlas-client', context.version), ...(context.source ? [path.join(context.source, '.local', 'atlas-client', context.version)] : [])];
    const resourcesPath = candidates.find(root => prepared(root, context.version)) || null;
    const bundledPath=path.join(this.runtime,'atlas-client.exe');
    const missingRuntime = !fs.existsSync(context.runtimePath) || (path.resolve(context.runtimePath)===bundledPath && PAYLOAD.filter(name=>name!=='atlas-prepare.exe').some(name=>!fs.existsSync(path.join(this.runtime,name))));
    const missingPreparer = ![path.join(path.dirname(context.runtimePath), 'atlas-prepare.exe'), path.join(this.runtime, 'atlas-prepare.exe'), path.join(this.bundle, 'atlas-prepare.exe')].some(file => fs.existsSync(file));
    return { ready: !missingRuntime && !!resourcesPath, missingRuntime, missingResources: !resourcesPath, missingPreparer, resourcesPath, runtimePath: context.runtimePath, bundledRuntime: fs.existsSync(path.join(this.bundle, 'atlas-client.exe')), setupAvailable: !!context.source && !missingPreparer };
  }
  cancel() {
    if (this.state.active) { this.cancelRequested=true; this.controller.abort(); }
    return this.snapshot();
  }
  async start(options = {}) {
    if (this.pending) return this.pending;
    this.pending = this.install(options).finally(() => { this.pending = null; });
    return this.pending;
  }
  async copyTree(source, target, destinationRoot) {
    this.checkCancelled(); guard(destinationRoot, target);
    const stat = fs.lstatSync(source);
    if (stat.isSymbolicLink()) throw new Error('Linked installer payload files are not allowed.');
    if (stat.isDirectory()) {
      await fs.promises.mkdir(target, { recursive: true });
      for (const name of await fs.promises.readdir(source)) await this.copyTree(path.join(source, name), path.join(target, name), destinationRoot);
    } else if (stat.isFile()) {
      await fs.promises.mkdir(path.dirname(target), { recursive: true });
      await fs.promises.copyFile(source, target);
    }
  }
  async response(url, signal) {
    for (let redirects = 0; redirects <= 5; redirects++) {
      this.checkCancelled();
      const response = await this.fetch(safeUrl(url), { signal, redirect: 'manual', headers: { 'User-Agent': 'Atlas/0.1' } });
      if ([301,302,303,307,308].includes(response.status)) {
        const location = response.headers.get('location'); await response.body?.cancel();
        if (!location) throw new Error('Game download redirected without an address.');
        url = safeUrl(new URL(location, url).href); continue;
      }
      if (!response.ok) { await response.body?.cancel(); throw new Error(`Game download failed (HTTP ${response.status}).`); }
      return response;
    }
    throw new Error('Too many game download redirects.');
  }
  async download(info) {
    guard(this.root, info.target);
    try {
      if (fs.statSync(info.target).isFile() && (info.size === undefined || fs.statSync(info.target).size === info.size) && await sha1(info.target) === info.sha1) return;
    } catch (error) { if (!['ENOENT','EISDIR'].includes(error.code)) throw error; }
    await fs.promises.mkdir(path.dirname(info.target), { recursive: true });
    for (let attempt = 0; attempt < 3; attempt++) {
      this.checkCancelled();
      const temporary = `${info.target}.part-${crypto.randomUUID()}`;
      const timeout = new AbortController();
      const abort = () => timeout.abort(); this.controller.signal.addEventListener('abort', abort, { once: true });
      const timer = setTimeout(abort, this.requestTimeoutMs);
      let handle;
      try {
        const response = await this.response(info.url, timeout.signal);
        handle = await fs.promises.open(temporary, 'wx');
        const hash = crypto.createHash('sha1'); let bytes = 0;
        for await (const chunk of response.body) {
          this.checkCancelled(); bytes += chunk.length;
          if (bytes > (info.size ?? 1024 * 1024 * 1024)) throw new Error('Downloaded file exceeds its declared size.');
          hash.update(chunk); await handle.writeFile(chunk);
        }
        await handle.close(); handle = null;
        if ((info.size !== undefined && bytes !== info.size) || hash.digest('hex') !== info.sha1) throw new Error('Downloaded file failed its checksum check.');
        guard(this.root, info.target); await fs.promises.rename(temporary, info.target);
        return;
      } catch (error) {
        await handle?.close().catch(() => {});
        await fs.promises.unlink(temporary).catch(() => {});
        if (this.controller.signal.aborted) throw cancelError();
        if (attempt === 2) throw error;
      } finally { clearTimeout(timer); this.controller.signal.removeEventListener('abort', abort); }
    }
  }
  async downloads(files, stage) {
    this.update({ phase: 'download', stage, progress: 0, completed: 0, total: files.length, bytesDone: 0, bytesTotal: files.reduce((n,f) => n + (f.size || 0), 0) });
    let cursor = 0;
    let failure;
    await Promise.allSettled(Array.from({ length: Math.min(this.jobs, files.length) }, async () => {
      try {
        while (cursor < files.length) {
          this.checkCancelled(); const info = files[cursor++]; await this.download(info);
          this.update({ completed: this.state.completed + 1, bytesDone: this.state.bytesDone + (info.size || 0), progress: (this.state.completed + 1) / Math.max(1, files.length) });
        }
      } catch(error) { if(!failure)failure=error;this.controller.abort(); }
    }));
    if(failure)throw failure;
  }
  async checkedJson(value, target, stage) {
    const info = fileInfo(value, target); await this.downloads([info], stage);
    return JSON.parse(await fs.promises.readFile(target, 'utf8'));
  }
  async java(component) {
    relative(component);
    const response = await this.response(JAVA_INDEX, this.controller.signal);
    const text = await response.text(); if (Buffer.byteLength(text) > 16 * 1024 * 1024) throw new Error('Java runtime index is too large.');
    const entry = JSON.parse(text)['windows-x64']?.[component]?.[0];
    if (!entry?.manifest) throw new Error('Mojang has no compatible Java runtime for this platform.');
    const manifest = await this.checkedJson(entry.manifest, path.join(this.root, 'java', component + '.json'), 'java');
    const home = path.join(this.root, 'java', component); const files = [];
    for (const [name, item] of Object.entries(manifest.files || {})) {
      const target = path.join(home, relative(name));
      if (item.type === 'file') files.push(fileInfo(item.downloads?.raw, target));
      else if (!['directory','link'].includes(item.type)) throw new Error('Unknown Java runtime entry.');
    }
    if (!files.length) throw new Error('Java runtime manifest has no files.');
    await this.downloads(files, 'java');
    if (!fs.existsSync(path.join(home, 'bin', 'java.exe'))) throw new Error('Java runtime is incomplete.');
    return home;
  }
  async prepare(executable, args, env) {
    this.checkCancelled();
    if (this.runPreparer) return this.runPreparer({ executable, args, env, signal: this.controller.signal, onLine: line => this.update({ stage: 'prepare', detail: String(line).slice(0,200) }) });
    const child = this.spawn(executable, args, { cwd: this.root, env, shell: false, windowsHide: true, stdio: ['ignore','pipe','pipe'] });
    let last = '';
    const abort = () => {
      // Only our disposable resource preparer is stopped, never a game process.
      if (process.platform === 'win32' && child.pid) childProcess.execFile('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], { windowsHide:true }, () => {});
      else child.kill('SIGTERM');
    };
    this.controller.signal.addEventListener('abort', abort, { once: true });
    if(this.controller.signal.aborted)abort();
    const log = chunk => { last = (last + String(chunk)).slice(-2000); this.update({ stage: 'prepare', detail: String(chunk).trim().slice(-200) }); };
    child.stdout?.on('data', log); child.stderr?.on('data', log);
    try {
      const [code] = await once(child, 'close'); this.checkCancelled();
      if (code !== 0) throw new Error(`Resource preparation failed (exit ${code}). ${last.slice(-600)}`);
    } finally { this.controller.signal.removeEventListener('abort', abort); }
  }
  async install(options) {
    this.controller = new AbortController();
    this.cancelRequested=false;
    this.update({ status:'running',phase:'verify',active:true,canCancel:true,error:null,stage:'checking',progress:null,completed:0,total:0,bytesDone:0,bytesTotal:0 });
    try {
      guard(this.dataRoot, this.root); guard(this.projectRoot, this.runtime);
      await fs.promises.mkdir(this.root, { recursive:true });
      const context = this.context(options), pins = context.pins;
      if (!context.source || !pins || !/^[a-zA-Z0-9_.-]+$/.test(pins.version) || ['.','..'].includes(pins.version)) throw new Error('This Atlas build is missing its preparation payload. Install the complete launcher package.');
      this.update({ phase:'install',stage:'client' });
      if (this.inspect(options).missingRuntime) {
        if (path.resolve(context.runtimePath) !== path.join(this.runtime,'atlas-client.exe')) throw new Error('Choose the bundled Atlas client before setup.');
        for (const name of PAYLOAD) {
          const source = path.join(this.bundle,name);
          if (!fs.existsSync(source)) throw new Error('This Atlas build is missing its game payload. Install the complete launcher package.');
          const destination = guard(this.projectRoot,path.join(this.runtime,name));
          if (!fs.existsSync(destination)) await this.copyTree(source,destination,this.projectRoot);
        }
      }
      if(this.inspect(options).ready){
        this.update({status:'ready',phase:'ready',stage:'ready',active:false,canCancel:false,progress:1,error:null});
        return this.snapshot();
      }
      const preparer = [path.join(this.runtime,'atlas-prepare.exe'),path.join(this.bundle,'atlas-prepare.exe')].find(file=>fs.existsSync(file));
      if (!preparer) throw new Error('This Atlas build is missing atlas-prepare.exe. Install the complete launcher package.');
      const game = path.join(this.root,'minecraft');
      const versionDir = path.join(game,'versions',pins.version);
      const manifestPath = path.join(versionDir,`${pins.version}.json`);
      let manifest;
      const localManifest=options.minecraftDirectory && path.join(options.minecraftDirectory,'versions',pins.version,`${pins.version}.json`);
      if(localManifest && fs.existsSync(localManifest) && [pins.manifest_sha1,...(pins.accepted_manifest_sha1||[])].includes(await sha1(localManifest))){
        await fs.promises.mkdir(versionDir,{recursive:true});
        await fs.promises.copyFile(localManifest,manifestPath);manifest=JSON.parse(await fs.promises.readFile(manifestPath,'utf8'));
      } else manifest = await this.checkedJson({ url:`https://piston-meta.mojang.com/v1/packages/${pins.manifest_sha1}/${pins.version}.json`,sha1:pins.manifest_sha1 },manifestPath,'metadata');
      if (manifest.id !== pins.version || manifest.downloads?.client?.sha1 !== pins.client_sha1 || manifest.downloads?.server?.sha1 !== pins.server_sha1) throw new Error('Minecraft metadata does not match this Atlas version.');
      const files = [fileInfo(manifest.downloads.client,path.join(versionDir,`${pins.version}.jar`)), fileInfo(manifest.downloads.server,path.join(this.root,'server.jar'))];
      for (const library of manifest.libraries || []) if (allowedLibrary(library) && library.downloads?.artifact) {
        const artifact=library.downloads.artifact; files.push(fileInfo(artifact,path.join(game,'libraries',relative(artifact.path))));
      }
      // Explicitly selected local files are read only; verified copies stay in Atlas's install cache.
      if (options.minecraftDirectory) {
        for(const info of files){
          const rel=path.relative(game,info.target);
          if(rel.startsWith('..')||path.isAbsolute(rel))continue;
          const source=path.join(options.minecraftDirectory,rel);
          if(fs.existsSync(source)&&await sha1(source)===info.sha1){await fs.promises.mkdir(path.dirname(info.target),{recursive:true});await fs.promises.copyFile(source,info.target);}
        }
      }
      if (options.serverJar && fs.existsSync(options.serverJar) && await sha1(options.serverJar)===pins.server_sha1) await fs.promises.copyFile(options.serverJar,path.join(this.root,'server.jar'));
      await this.downloads(files,'game');
      const asset = manifest.assetIndex;
      const indexPath=path.join(game,'assets','indexes',`${relative(asset.id)}.json`);
      const localIndex=options.minecraftDirectory&&path.join(options.minecraftDirectory,'assets','indexes',`${relative(asset.id)}.json`);
      if(localIndex&&fs.existsSync(localIndex)&&await sha1(localIndex)===asset.sha1){await fs.promises.mkdir(path.dirname(indexPath),{recursive:true});await fs.promises.copyFile(localIndex,indexPath);}
      const index = await this.checkedJson(asset,indexPath,'assets');
      const objects = new Map();
      for (const value of Object.values(index.objects || {})) {
        if (!/^[a-f0-9]{40}$/i.test(value.hash || '')) throw new Error('Invalid asset hash.');
        objects.set(value.hash,fileInfo({url:`https://resources.download.minecraft.net/${value.hash.slice(0,2)}/${value.hash}`,sha1:value.hash,size:value.size},path.join(game,'assets','objects',value.hash.slice(0,2),value.hash)));
      }
      if(options.minecraftDirectory)for(const info of objects.values()){
        const source=path.join(options.minecraftDirectory,path.relative(game,info.target));
        if(fs.existsSync(source)&&await sha1(source)===info.sha1){await fs.promises.mkdir(path.dirname(info.target),{recursive:true});await fs.promises.copyFile(source,info.target);}
      }
      await this.downloads([...objects.values()],'assets');
      const javaHome=await this.java(manifest.javaVersion?.component || 'java-runtime-epsilon');
      const preparation=path.join(this.root,'preparation-root');
      this.update({phase:'prepare',stage:'prepare',progress:null,completed:0,total:0,bytesDone:0,bytesTotal:0});
      for (const name of SOURCE_FILES) {
        const source=path.join(context.source,name);
        if (fs.existsSync(source)) await this.copyTree(source,path.join(preparation,name),this.root);
      }
      await this.prepare(preparer,['--prepare','--minecraft-dir',game,'--server-jar',path.join(this.root,'server.jar'),'--root',preparation],{...process.env,JAVA_HOME:javaHome,ATLAS_HOME:this.root,ATLAS_LAUNCHER_HOME:this.root,ATLAS_NO_AUDIO:'1',ATLAS_NO_HEARTBEAT:'1',ATLAS_TELEMETRY_URL:'off',PATH:path.join(javaHome,'bin')+path.delimiter+process.env.PATH});
      this.checkCancelled();
      const result=path.join(preparation,'.local','atlas-client',pins.version);
      if (!prepared(result,pins.version)) throw new Error('Prepared game resources are incomplete.');
      this.update({phase:'install',stage:'install',progress:null,completed:0,total:0,bytesDone:0,bytesTotal:0});
      const destination=guard(this.projectRoot,path.join(this.runtime,'.local','atlas-client',pins.version));
      await fs.promises.mkdir(path.dirname(destination),{recursive:true});
      const staging=destination+'.install-'+crypto.randomUUID();
      await this.copyTree(result,staging,this.projectRoot); this.checkCancelled();
      const backup=destination+'.previous-'+crypto.randomUUID();let previous=false;
      try {
        if(fs.existsSync(destination)){await fs.promises.rename(destination,backup);previous=true;}
        await fs.promises.rename(staging,destination);
      } catch(error){if(previous&&!fs.existsSync(destination))await fs.promises.rename(backup,destination);throw error;}
      this.update({status:'ready',phase:'ready',stage:'ready',active:false,canCancel:false,progress:1,error:null});
      return this.snapshot();
    } catch(error) {
      const cancelled=this.cancelRequested===true||error.code==='cancelled';
      this.controller.abort();
      this.update({status:cancelled?'cancelled':'error',phase:cancelled?'cancelled':'error',active:false,canCancel:false,progress:null,error:cancelled?null:{code:'setup_failed',message:error.message}});
      if(!cancelled)throw error;
      return this.snapshot();
    }
  }
}
module.exports={GameInstallManager,relative,safeUrl,allowedLibrary,prepared};
