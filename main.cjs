const { app, BrowserWindow, ipcMain, dialog, shell, safeStorage } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const { performance } = require('node:perf_hooks');
const { LauncherStore } = require('./lib/launcher-store.cjs');
const { GameSessions } = require('./lib/game-sessions.cjs');
const { DiscordPresence, readPreference } = require('./lib/discord-rpc.cjs');
const manifest = require('./package.json');
const { AccountManager } = require('./lib/account-manager.cjs');
const { t } = require('./i18n.js');

app.setName('Atlas Launcher');
if (process.platform === 'win32') app.setAppUserModelId('Atlas.Launcher.Desktop');
let window;
let store;
let games;
let accounts;
let discordPresence;
let discordPoll;
let discordObserved;
const launcherVersion = manifest.version.replace(/\.0$/, '');
const logs = [];
let publishTimer;
const projectRoot = app.isPackaged ? path.resolve(path.dirname(process.execPath), '..', '..') : __dirname;
const startupDeadline = performance.timeOrigin + 1000;
const guiTest = !app.isPackaged && process.env.ATLAS_GUI_TEST === '1';
if (guiTest) {
  const profile = path.join(projectRoot, 'work', 'gui-test-profile');
  fs.mkdirSync(profile, { recursive: true });
  app.setPath('userData', profile);
}

function appendLog(message, level = 'info') {
  const entry = { time: new Date().toISOString(), level, message: String(message).slice(0, 8000) };
  logs.push(entry);
  if (logs.length > 300) logs.splice(0, logs.length - 300);
  return entry;
}

function state() {
  return { ...store.getState(), version: launcherVersion, game: games.snapshot(), accounts: accounts?.snapshot(), discord: { enabled: discordEnabled(), ...(discordPresence?.snapshot() || { status: discordEnabled() ? 'connecting' : 'disabled', connected: false, error: null }) }, logs: [...logs] };
}

function discordEnabled() {
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
  if (window && !window.isDestroyed()) window.webContents.send('atlas:game-event', state());
}

function publishSoon() {
  // A noisy client must not flood the renderer and delay the launch IPC reply.
  if (!publishTimer) publishTimer = setTimeout(publish, 150);
}

async function launch(id, additional = false) {
  if (additional) await games.launchAnother(id);
  else await games.launch(id);
  if (store.config.settings.closeOnLaunch && window && !window.isDestroyed()) window.hide();
  publish();
  return state();
}

function registerHandlers() {
  function handle(name, callback) {
    ipcMain.handle(`atlas:${name}`, async (event, ...args) => {
      if (!window || event.sender !== window.webContents || event.senderFrame !== window.webContents.mainFrame) throw new Error('Недопустимый источник команды.');
      try { return await callback(...args); }
      catch (error) { const message = t(error.message, store.config.settings.language); appendLog(message, 'error'); publish(); throw new Error(message); }
    });
  }
  handle('get-state', () => state());
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
  handle('set-game-options', (id, patch) => {
    if (games.isInstanceRunning(id)) throw new Error('Закрой окна этой сборки перед изменением её настроек.');
    store.setGameOptions(id, patch);
    return state();
  });
  handle('launch', (id) => launch(id));
  handle('launch-another', (id) => launch(id, true));
  handle('stop-game', async (sessionId) => { await games.stop(sessionId); return state(); });
  handle('restart-game', async (sessionId) => { await games.restart(sessionId); return state(); });
  handle('open-folder', async (kind, id) => {
    const directory = store.folder(kind, id);
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
    try { store = new LauncherStore(projectRoot, guiTest ? { dataRoot: path.join(projectRoot, 'work', 'gui-test-data') } : undefined); }
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
    games = new GameSessions(store, { prepareAccount: home => accounts.prepareLaunch(home), extraGameArgs: guiTest ? ['--offscreen'] : [], onLog: appendLog, onChange: (noisy) => {
      if (noisy) { publishSoon(); return; }
      if (!games.snapshot().running && window && !window.isDestroyed()) {
        if (!window.isVisible()) window.show();
        if (window.isMinimized()) window.restore();
      }
      publish();
    } });
    registerHandlers();
    window = new BrowserWindow({
      width: 1360, height: 900, minWidth: 1050, minHeight: 740,
      title: 'Atlas Launcher', frame: false, backgroundColor: '#171717', show: false,
      ...(fs.existsSync(path.join(__dirname, 'assets', 'icon.png')) ? { icon: path.join(__dirname, 'assets', 'icon.png') } : {}),
      webPreferences: { preload: path.join(__dirname, 'preload.cjs'), contextIsolation: true, sandbox: true, nodeIntegration: false, devTools: !app.isPackaged },
    });
    window.setMenuBarVisibility(false);
    window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
    window.webContents.on('will-navigate', (event) => event.preventDefault());
    window.once('ready-to-show', () => { window.show(); setImmediate(startDiscord); });
    if (guiTest) window.webContents.on('did-finish-load', () => window.setTitle('Atlas Launcher — проверка'));
    window.on('close', (event) => { if (games.snapshot().running) { event.preventDefault(); window.hide(); } });
    appendLog('Atlas Launcher готов. Сборки хранятся в отдельных папках.');
    window.loadFile(path.join(__dirname, 'index.html'), { query: { startupDeadline: String(startupDeadline) } });
  });
  app.on('window-all-closed', () => app.quit());
  app.on('will-quit', () => { clearInterval(discordPoll); discordPresence?.dispose(); accounts?.dispose(); });
}
