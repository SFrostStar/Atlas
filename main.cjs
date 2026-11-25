const { app, BrowserWindow, ipcMain, dialog, shell, safeStorage, screen } = require('electron');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { performance } = require('node:perf_hooks');
const { LauncherStore } = require('./lib/launcher-store.cjs');
const { GameSessions } = require('./lib/game-sessions.cjs');
const { DiscordPresence, readPreference } = require('./lib/discord-rpc.cjs');
const manifest = require('./package.json');
const { AccountManager } = require('./lib/account-manager.cjs');
const { t } = require('./i18n.js');
const { GameInstallManager } = require('./lib/game-install.cjs');
const { resolveLauncherLayout } = require('./lib/portable-root.cjs');
const { ModManager } = require('./lib/mod-manager.cjs');
const { NativeMods } = require('./lib/native-mods.cjs');
const { ReleaseUpdater, pruneUpdates } = require('./lib/release-updater.cjs');

app.setName('Atlas Launcher');
if (process.platform === 'win32') app.setAppUserModelId('Atlas.Launcher.Desktop');
let window;
let store;
let games;
let accounts;
let installer;
let mods;
let nativeBuildStage = null;
let nativeBuild = null;
let updater;
let updatePoll;
let updateClosing = false;
let updateApplying = false;
let needsSetup=false;
let supportVisible=true;
let discordPresence;
let discordPoll;
let discordObserved;
const launcherVersion = manifest.version;
const logs = [];
let publishTimer;
let publishLogTimer;
const layout = app.isPackaged ? resolveLauncherLayout(process.execPath) : { projectRoot: __dirname };
const { projectRoot, bundleDirectory, bootstrapManaged = false } = layout;
const startupDeadline = performance.timeOrigin + 1000;
const guiTestRoot = !app.isPackaged && process.env.ATLAS_GUI_TEST;
const guiTest = Boolean(guiTestRoot);
if (guiTestRoot) {
  if (!path.isAbsolute(guiTestRoot)) throw new Error('ATLAS_GUI_TEST requires an absolute test directory.');
  const profile = path.join(guiTestRoot, 'profile');
  fs.mkdirSync(profile, { recursive: true });
  app.setPath('userData', profile);
  app.setPath('sessionData', profile);
}

function bootstrapHandoff() {
  if (process.platform !== 'win32') return null;
  const token = process.env.ATLAS_BOOTSTRAP_HANDOFF || '';
  const coordinates = /^(-?\d{1,7}),(-?\d{1,7}),(\d{1,5}),(\d{1,5})$/.exec(process.env.ATLAS_BOOTSTRAP_BOUNDS || '');
  if (!/^[a-f0-9]{32}$/.test(token) || !coordinates) return null;
  const [x, y, width, height] = coordinates.slice(1).map(Number);
  if (width < 1 || height < 1 || width > 16384 || height > 16384) return null;
  const bounds = screen.screenToDipRect(null, { x, y, width, height });
  const workArea = screen.getDisplayMatching(bounds).workArea;
  const w = Math.min(Math.max(1, bounds.width), workArea.width);
  const h = Math.min(Math.max(1, bounds.height), workArea.height);
  return {
    marker: path.join(os.tmpdir(), `atlas-handoff-${token}.ready`),
    bounds: {
      x: Math.min(Math.max(bounds.x, workArea.x), workArea.x + workArea.width - w),
      y: Math.min(Math.max(bounds.y, workArea.y), workArea.y + workArea.height - h),
      width: w,
      height: h,
    },
  };
}

function appendLog(message, level = 'info') {
  const entry = { time: new Date().toISOString(), level, message: String(message).slice(0, 8000) };
  logs.push(entry);
  if (logs.length > 300) logs.splice(0, logs.length - 300);
  return entry;
}

function state() {
  return { ...store.getState(), mods: mods?.snapshot(store.config.selectedId), version: launcherVersion, bootstrapManaged, nativeBuildStage, nativeBuild, update: updater ? { ...updater.snapshot(), applying: updateApplying } : undefined, game: { ...games.snapshot(), busy: gameBusy() }, supportVisible, install: installer ? { ...installer.snapshot(), ...installer.inspect({runtimePath:store.config.settings.runtimePath,sourceRoot:store.config.settings.sourceRoot}), needsSetup } : null, accounts: accounts?.snapshot(), discord: { enabled: discordEnabled(), ...(discordPresence?.snapshot() || { status: discordEnabled() ? 'connecting' : 'disabled', connected: false, error: null }) }, logs: [...logs], logSessions: games.logSnapshot() };
}

function gameBusy() {
  return Boolean(games?.snapshot().running || games?.pendingInstances.size);
}

function assertUpdateIdle() {
  if (updateApplying || updateClosing) throw new Error('Обновление устанавливается.');
}

async function checkUpdates() {
  if (!updater || updateClosing) return;
  await updater.check();
  if (updater.snapshot().status === 'available') await updater.download();
}

async function applyUpdate(restart) {
  if (!updater || updateClosing || updateApplying) return;
  if (gameBusy() || installer?.snapshot().active) throw new Error('Закрой игру и дождись завершения установки файлов.');
  updateApplying = true;
  publishSoon();
  try {
    await updater.install({ restart });
    updateClosing = true;
    app.quit();
  } catch (error) {
    updateApplying = false;
    publishSoon();
    throw error;
  }
}

function discordEnabled() {
  if (guiTest) return false;
  return typeof store.config.settings.discordRichPresence === 'boolean'
    ? store.config.settings.discordRichPresence
    : readPreference(store.instancePath(store.config.selectedId));
}

function syncDiscord(force = false) {
  if (!discordPresence) return;
  const options = {
    enabled: discordEnabled(),
    applicationId: manifest.atlasDiscord.applicationId,
    imageKey: manifest.atlasDiscord.imageKey,
    version: launcherVersion,
  };
  if (force) discordPresence.configure({ ...options, enabled: false });
  discordPresence.configure(options);
}

function observeDiscordPreference() {
  if (store.deletingInstances?.has(store.config.selectedId)) return;
  const home = store.instancePath(store.config.selectedId);
  const exists = fs.existsSync(path.join(home, 'atlas-extras.txt'));
  const preference = readPreference(home);
  if (discordObserved?.home === home && discordObserved.exists && exists && discordObserved.preference !== preference) {
    store.saveSettings({ discordRichPresence: preference });
    syncDiscord();
    publishSoon();
  }
  const changedHome = discordObserved?.home !== home;
  const createdFile = !discordObserved?.exists && exists;
  discordObserved = { home, preference, exists };
  if (changedHome || createdFile) syncDiscord();
}

function startDiscord() {
  discordPresence = new DiscordPresence({ version: launcherVersion, onStatus: publishSoon });
  observeDiscordPreference();
  syncDiscord();
  discordPoll = setInterval(observeDiscordPreference, 3000);
  discordPoll.unref();
}

function publish() {
  clearTimeout(publishTimer);
  publishTimer = undefined;
  clearTimeout(publishLogTimer);
  publishLogTimer = undefined;
  if (window && !window.isDestroyed()) window.webContents.send('atlas:game-event', state());
}

function publishSoon() {
  if (!publishTimer) publishTimer = setTimeout(publish, 150);
}

function publishLogsSoon() {
  // Game output has its own event so a running client does not redraw the launcher.
  if (!publishLogTimer) publishLogTimer = setTimeout(() => {
    publishLogTimer = undefined;
    if (window && !window.isDestroyed()) window.webContents.send('atlas:log-event', games.logSnapshot());
  }, 150);
}

async function launch(id, additional = false, windowName) {
  assertUpdateIdle();
  const options = { runtimePath: store.config.settings.runtimePath, sourceRoot: store.config.settings.sourceRoot };
  const availability = installer.inspect(options);
  if (bootstrapManaged && availability.missingRuntime && !availability.missingResources &&
      availability.bundledRuntime && !gameBusy() && options.runtimePath === store.defaultRuntime) {
    await installer.start(options);
  }
  if (!installer.inspect(options).ready) { needsSetup = true; publish(); return state(); }
  needsSetup=false;
  if (additional) await games.launchAnother(id, windowName);
  else await games.launch(id);
  if (games.snapshot().running && store.config.settings.closeOnLaunch && window && !window.isDestroyed()) window.hide();
  publish();
  return state();
}

function startInstall(options={}) {
  assertUpdateIdle();
  if(gameBusy())throw new Error('Закрой игру перед подготовкой файлов.');
  if(!installer.snapshot().active){
    needsSetup=true;
    installer.start({...options,runtimePath:store.config.settings.runtimePath,sourceRoot:store.config.settings.sourceRoot})
      .then(()=>{if(installer.inspect({runtimePath:store.config.settings.runtimePath,sourceRoot:store.config.settings.sourceRoot}).ready)needsSetup=false;publish();})
      .catch(error=>{appendLog(error.message,'error');publish();});
  }
  return state();
}

function registerHandlers(onStartupReady) {
  function handle(name, callback) {
    ipcMain.handle(`atlas:${name}`, async (event, ...args) => {
      if (!window || event.sender !== window.webContents || event.senderFrame !== window.webContents.mainFrame) throw new Error('Недопустимый источник команды.');
      try { return await callback(...args); }
      catch (error) { const message = t(error.message, store.config.settings.language); appendLog(message, 'error'); publish(); throw new Error(message); }
    });
  }
  if (onStartupReady) handle('startup-ready', onStartupReady);
  handle('get-state', () => state());
  handle('check-updates', async () => { await checkUpdates(); return state(); });
  handle('apply-update', () => applyUpdate(true));
  handle('update-instance', (id,patch) => {store.updateInstance(id,patch);return state();});
  handle('support-gift', () => shell.openExternal('https://t.me/protecteduzername'));
  handle('dismiss-support', () => {supportVisible=false;return state();});
  handle('start-install', () => startInstall());
  handle('cancel-install', () => {installer.cancel();return state();});
  handle('choose-minecraft-install', async () => {
    const result=await dialog.showOpenDialog(window,{title:t('Выбрать установленный Minecraft',store.config.settings.language),properties:['openDirectory']});
    if(!result.canceled&&result.filePaths[0])return startInstall({minecraftDirectory:result.filePaths[0]});
    return state();
  });
  handle('create-instance', (input) => { store.createInstance(input); observeDiscordPreference(); syncDiscord(); return state(); });
  handle('select-instance', (id) => { store.selectInstance(id); observeDiscordPreference(); syncDiscord(); return state(); });
  handle('delete-instance', async (id) => {
    await store.deleteInstance(id, { trashItem: folder => shell.trashItem(folder), isRunning: instanceId => games.isInstanceRunning(instanceId) });
    observeDiscordPreference();
    syncDiscord();
    publish();
    return state();
  });
  handle('save-settings', (patch) => { store.saveSettings(patch); syncDiscord(); return state(); });
  handle('set-instance-memory', (id, memoryMb) => { store.setInstanceMemory(id, memoryMb); return state(); });
  handle('add-local-account', name => { accounts.addLocal(name); return state(); });
  handle('select-account', id => { accounts.select(id); return state(); });
  handle('remove-account', id => { accounts.remove(id); return state(); });
  handle('start-signin', async () => { await accounts.startSignIn(); return state(); });
  handle('cancel-signin', () => { accounts.cancelSignIn(); return state(); });
  handle('open-microsoft', () => shell.openExternal('https://www.microsoft.com/link'));
  handle('refresh-discord', () => { syncDiscord(true); return state(); });
  handle('refresh-mods', async () => { await mods.refresh(); return state(); });
  handle('change-mod', (id, source, project, action) => {
    if (games.isInstanceRunning(id)) throw new Error('Закрой окна этой сборки перед изменением её модов.');
    mods.change(id, source, project, action);
    try { mods.syncPortFiles(id); }
    catch (error) { appendLog(`Не удалось обновить список файлов модов: ${error.message}`, 'error'); }
    return state();
  });
  handle('mod-page', (id) => {
    const project=mods.projects.find(p=>p.id===id);
    if(!project) throw new Error('Мод не найден в каталоге.');
    return shell.openExternal(project.page);
  });
  handle('set-game-options', (id, patch) => {
    if (games.isInstanceRunning(id)) throw new Error('Закрой окна этой сборки перед изменением её настроек.');
    store.setGameOptions(id, patch);
    return state();
  });
  handle('launch', (id) => launch(id));
  handle('launch-another', (id, windowName) => launch(id, true, windowName));
  handle('stop-game', async (sessionId) => { await games.stop(sessionId); return state(); });
  handle('restart-game', async (sessionId) => { assertUpdateIdle(); await games.restart(sessionId); return state(); });
  handle('open-folder', async (kind, id) => {
    const directory = kind === 'mods' ? mods.syncPortFiles(id ?? store.config.selectedId) : store.folder(kind, id);
    if (!fs.existsSync(directory)) throw new Error('Эта папка ещё не создана.');
    const error = await shell.openPath(directory);
    if (error) throw new Error(`Не удалось открыть папку: ${error}`);
    return true;
  });
  handle('choose-runtime', async () => {
    const result = await dialog.showOpenDialog(window, { title: t('Выбрать клиент Atlas', store.config.settings.language), defaultPath: store.config.settings.runtimePath, filters: [{ name: 'Клиент Atlas', extensions: ['exe'] }], properties: ['openFile'] });
    if (!result.canceled && result.filePaths[0]) store.saveSettings({ runtimePath: result.filePaths[0] });
    return state();
  });
  handle('choose-source', async () => {
    const result = await dialog.showOpenDialog(window, { title: t('Папка исходников Atlas', store.config.settings.language), defaultPath: store.config.settings.sourceRoot, properties: ['openDirectory'] });
    if (!result.canceled && result.filePaths[0]) {
      if (!fs.existsSync(path.join(result.filePaths[0], 'Cargo.toml'))) throw new Error('В выбранной папке нет Cargo.toml проекта игры.');
      store.saveSettings({ sourceRoot: result.filePaths[0] });
    }
    return state();
  });
  handle('window-control', (action) => {
    if (action === 'minimize') window.minimize();
    else if (action === 'maximize') { if (window.isMaximized()) window.unmaximize(); else window.maximize(); }
    else if (action === 'close') { if (games.snapshot().running) window.hide(); else window.close(); }
    else throw new Error('Неизвестное действие окна.');
  });
}

if (!app.requestSingleInstanceLock()) app.quit();
else {
  app.on('second-instance', () => {
    if (window) { window.show(); if (window.isMinimized()) window.restore(); window.focus(); }
  });
  app.whenReady().then(() => {
    try { store = new LauncherStore(projectRoot, guiTest ? { dataRoot: path.join(guiTestRoot, 'data') } : undefined); }
    catch (error) { dialog.showErrorBox('Atlas Launcher', error.message); app.quit(); return; }
    try {
      const existingAccounts = fs.existsSync(path.join(store.dataRoot, 'accounts.json'));
      accounts = new AccountManager({ dataRoot: store.dataRoot, safeStorage, clientId: process.env.ATLAS_MSA_CLIENT_ID || manifest.atlasMicrosoft?.clientId || '', openExternal: url => shell.openExternal(url), onChange: publishSoon });
      if (!existingAccounts) {
        const home = store.instancePath(store.config.selectedId);
        for (const nameFile of ['atlas-player.txt', 'horizon-player.txt']) {
          const file = path.join(home, nameFile);
          if (!fs.existsSync(file)) continue;
          const nickname = fs.readFileSync(file, 'utf8').match(/^name:([A-Za-z0-9_]{3,16})\s*$/m)?.[1];
          if (nickname && nickname !== 'Player') {
            const initialId = accounts.snapshot().selectedId;
            accounts.addLocal(nickname); accounts.remove(initialId);
          }
          break;
        }
      }
    }
    catch (error) { dialog.showErrorBox('Atlas Launcher', error.message); app.quit(); return; }
    const nativeMods = new NativeMods({
      project: projectRoot, dataRoot: store.dataRoot, version: launcherVersion,
      source: guiTest && process.env.ATLAS_GUI_NATIVE_SOURCE
        ? process.env.ATLAS_GUI_NATIVE_SOURCE : path.join(__dirname, 'native-source'),
      sourceManifest: path.join(__dirname, 'native-source.json'),
      progress(stage, details = null) { nativeBuildStage = stage; nativeBuild = details; publishSoon(); },
      async confirm(bundles) {
        const language = store.config.settings.language;
        const { response } = await dialog.showMessageBox(window, {
          type: 'warning',
          title: t('Rust-моды', language),
          message: t('Доверяешь этим модам?', language),
          detail: bundles.map(mod => `${mod.name} · ${mod.version}`).join('\n') + '\n\n'
            + t('Atlas соберёт игру с этими модами. Их код получит доступ к файлам с правами твоего пользователя.', language)
            + '\n\n' + t('При первой сборке могут скачаться исходники и инструменты.', language),
          buttons: [t('Отмена', language), t('Доверять и собрать', language)],
          defaultId: 0, cancelId: 0, noLink: true,
        });
        return response === 1;
      },
    });
    games = new GameSessions(store, {
      prepareAccount: (home, accountId) => accounts.prepareLaunch(home, accountId),
      prepareRuntime: (home, builtinAccountSwitch) => nativeMods.prepare(home, builtinAccountSwitch ? ['ias'] : []),
      getAccounts: () => accounts.snapshot(),
      accountActions: accounts,
      builtinAccountSwitch: instanceId => mods.installed(instanceId).has('atlas-ias') && !mods.disabled(instanceId).has('atlas-ias'),
      extraGameArgs: guiTest ? ['--offscreen'] : [], onChange: (noisy) => {
      if (noisy) { publishLogsSoon(); return; }
      if (!games.snapshot().running && window && !window.isDestroyed()) {
        if (!window.isVisible()) window.show();
        if (window.isMinimized()) window.restore();
      }
      publish();
    } });
    installer = new GameInstallManager({projectRoot,dataRoot:store.dataRoot,bundleDirectory,onChange:()=>{if(installer?.snapshot().status==='ready')needsSetup=false;publishSoon();}});
    mods = new ModManager(store);
    for (const {id} of store.config.instances) {
      try { mods.syncPortFiles(id); }
      catch (error) { appendLog(error.message, 'error'); }
    }
    if (app.isPackaged && !bootstrapManaged && process.env.ATLAS_LAUNCHER_NO_SELF_UPDATE !== '1') {
      updater = new ReleaseUpdater({ projectRoot, dataRoot: store.dataRoot, executable: process.execPath,
        currentVersion: launcherVersion, onChange: publishSoon,
        isGameRunning: () => Boolean(gameBusy() || installer?.snapshot().active) });
    }
    const handoff = bootstrapHandoff();
    let painted = false;
    let rendered = false;
    let shown = false;
    let displayDeadline;
    const showLauncher = (handoffReady = false) => {
      if (shown || window.isDestroyed()) return;
      shown = true;
      clearTimeout(displayDeadline);
      if (handoffReady) {
        window.showInactive();
        try { fs.writeFileSync(handoff.marker, '', { flag: 'wx' }); }
        catch (error) { appendLog(error.message, 'error'); }
      } else {
        window.show();
      }
      setImmediate(startDiscord);
      setImmediate(() => {
        void Promise.allSettled([
          pruneUpdates(store.dataRoot, launcherVersion),
          installer.prunePreparation(),
          nativeMods.maintain(),
        ]);
      });
      if (updater) {
        setTimeout(() => checkUpdates().catch(error => { appendLog(error.message, 'error'); publishSoon(); }), 2500).unref();
        updatePoll = setInterval(() => checkUpdates().catch(error => { appendLog(error.message, 'error'); publishSoon(); }), 30 * 60 * 1000);
        updatePoll.unref();
      }
    };
    registerHandlers(handoff && (() => {
      rendered = true;
      if (painted) showLauncher(true);
    }));
    window = new BrowserWindow({
      width: 1360, height: 900,
      minWidth: Math.min(1050, handoff?.bounds.width ?? 1050),
      minHeight: Math.min(740, handoff?.bounds.height ?? 740),
      ...handoff?.bounds,
      title: 'Atlas Launcher', frame: false, backgroundColor: '#171717', show: false,
      ...(fs.existsSync(path.join(__dirname, 'assets', 'icon.png')) ? { icon: path.join(__dirname, 'assets', 'icon.png') } : {}),
      webPreferences: { preload: path.join(__dirname, 'preload.cjs'), contextIsolation: true, sandbox: true, nodeIntegration: false, devTools: !app.isPackaged },
    });
    window.setMenuBarVisibility(false);
    window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
    window.webContents.on('will-navigate', (event) => event.preventDefault());
    window.once('ready-to-show', () => {
      painted = true;
      if (!handoff || rendered) showLauncher(Boolean(handoff));
    });
    if (handoff) displayDeadline = setTimeout(showLauncher, 5000).unref();
    if (guiTest) window.webContents.on('did-finish-load', () => window.setTitle('Atlas Launcher — проверка'));
    window.on('close', (event) => {
      if (updateClosing) return;
      if (updateApplying) { event.preventDefault(); return; }
      if (gameBusy()) { event.preventDefault(); window.hide(); return; }
      if (updater?.snapshot().status === 'ready' && !installer?.snapshot().active) {
        event.preventDefault();
        applyUpdate(false).catch(error => { appendLog(error.message, 'error'); publish(); });
      }
    });
    const query = handoff ? { bootstrapHandoff: '1' } : { startupDeadline: String(startupDeadline) };
    window.loadFile(path.join(__dirname, 'index.html'), { query });
  });
  app.on('window-all-closed', () => app.quit());
  app.on('will-quit', () => { clearInterval(discordPoll); clearInterval(updatePoll); discordPresence?.dispose(); accounts?.dispose(); installer?.cancel(); });
}
