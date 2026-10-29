const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const childProcess = require('node:child_process');
const { promisify } = require('node:util');
const { buildLaunchSpec } = require('./launcher-store.cjs');
const { waitForSpawn } = require('./process-start.cjs');

const SETTINGS_FILES = ['options.txt', 'atlas-extras.txt', 'atlas-graphics.txt', 'atlas-player.txt', 'servers.dat'];
const PACK_FOLDERS = ['resourcepacks', 'atlas-mods', 'shaderpacks'];

async function createAdditionalHome(store, instanceId, sessionId) {
  const base = store.ensureFolders(instanceId);
  const home = path.join(store.dataRoot, 'sessions', sessionId, 'home');
  for (const directory of ['', 'saves', 'logs', 'screenshots']) await fs.promises.mkdir(path.join(home, directory), { recursive: true });
  for (const name of SETTINGS_FILES) {
    const source = path.join(base, name);
    if (fs.existsSync(source) && fs.lstatSync(source).isFile()) await fs.promises.copyFile(source, path.join(home, name));
  }
  for (const name of PACK_FOLDERS) {
    const source = path.join(base, name);
    if (!fs.existsSync(source)) continue;
    await fs.promises.cp(source, path.join(home, name), { recursive: true, filter: (file) => !fs.lstatSync(file).isSymbolicLink() });
  }
  await fs.promises.writeFile(path.join(path.dirname(home), 'session.json'), JSON.stringify({ instanceId, home, createdAt: new Date().toISOString() }, null, 2));
  return home;
}

class GameSessions {
  constructor(store, options = {}) {
    this.store = store;
    this.spawn = options.spawn || childProcess.spawn;
    this.execFile = options.execFile || promisify(childProcess.execFile);
    this.onChange = options.onChange || (() => {});
    this.onLog = options.onLog || (() => {});
    this.stopTimeoutMs = options.stopTimeoutMs || 45000;
    this.startTimeoutMs = options.startTimeoutMs || 15000;
    this.prepareAccount = options.prepareAccount || (async () => ({ args: [], cleanup: async () => {} }));
    this.extraGameArgs = options.extraGameArgs || [];
    this.sessions = new Map();
    this.pendingHomes = new Set();
    this.pendingInstances = new Map();
  }

  snapshot() {
    const sessions = [...this.sessions.values()].map(({ id, instanceId, name, child, home, socketPath, status, isolated, startedAt }) => ({ id, instanceId, name: this.store.requireInstance(instanceId).name + (isolated ? ' · ещё окно' : ''), pid: child?.pid || null, home, socketPath, status, isolated, startedAt }));
    const selected = sessions.find(item => item.instanceId === this.store.config.selectedId) || sessions[0];
    return { running: sessions.length > 0, count: sessions.length, instanceId: selected?.instanceId || null, sessionId: selected?.id || null, sessions };
  }

  isInstanceRunning(instanceId) {
    return this.pendingInstances.has(instanceId) || [...this.sessions.values()].some(session => session.instanceId === instanceId);
  }

  pick(sessionId) {
    if (sessionId !== undefined && sessionId !== null) {
      const session = this.sessions.get(sessionId);
      if (!session) throw new Error('Это окно игры уже закрыто.');
      return session;
    }
    const selectedId = this.store.config.selectedId;
    const session = [...this.sessions.values()].find(item => item.instanceId === selectedId) || this.sessions.values().next().value;
    if (!session) throw new Error('Игра не запущена.');
    return session;
  }

  async launch(instanceId, { additional = false, home: providedHome, isolated: providedIsolated } = {}) {
    this.store.assertInstanceAvailable(instanceId);
    this.pendingInstances.set(instanceId, (this.pendingInstances.get(instanceId) || 0) + 1);
    try {
      return await this.launchProcess(instanceId, { additional, home: providedHome, isolated: providedIsolated });
    } finally {
      const remaining = this.pendingInstances.get(instanceId) - 1;
      if (remaining) this.pendingInstances.set(instanceId, remaining);
      else this.pendingInstances.delete(instanceId);
    }
  }

  async launchProcess(instanceId, { additional = false, home: providedHome, isolated: providedIsolated } = {}) {
    this.store.requireInstance(instanceId);
    const sessionId = crypto.randomUUID();
    const isolated = providedIsolated ?? additional;
    let home = providedHome;
    if (!home) home = additional ? await createAdditionalHome(this.store, instanceId, sessionId) : this.store.ensureFolders(instanceId);
    if (this.pendingHomes.has(home) || [...this.sessions.values()].some(session => session.home === home)) {
      throw new Error('Эта сборка уже открыта. Используй «Ещё окно».');
    }
    this.pendingHomes.add(home);
    let accountSession;
    let accountCleanup;
    const cleanupAccount = () => {
      if (!accountSession) return Promise.resolve();
      return accountCleanup ||= (async () => {
        try { await accountSession.cleanup(); }
        catch { this.onLog('Could not remove an expired account session.', 'error'); }
      })();
    };
    let output;
    let record;
    try {
      const spec = buildLaunchSpec(this.store, instanceId);
      accountSession = await this.prepareAccount(home);
      const socketPath = path.join(this.store.dataRoot, 'control', `${sessionId}.json`);
      await fs.promises.mkdir(path.dirname(socketPath), { recursive: true });
      await fs.promises.mkdir(path.join(home, 'logs'), { recursive: true });
      output = fs.createWriteStream(path.join(home, 'logs', 'atlas-launch.log'), { flags: 'a' });
      output.on('error', error => this.onLog(`Не удалось записать журнал: ${error.message}`, 'error'));
      output.write(`\n[${new Date().toISOString()}] Atlas запускает ${spec.executable}\n`);
      let resolveClosed;
      record = { id: sessionId, instanceId, name: this.store.requireInstance(instanceId).name + (isolated ? ' · ещё окно' : ''), home, socketPath, isolated, startedAt: Date.now(), status: 'starting', closed: new Promise(resolve => { resolveClosed = resolve; }), resolveClosed };
      const child = this.spawn(spec.executable, [...spec.args, ...accountSession.args, ...this.extraGameArgs, '--control-socket', socketPath], { cwd: home, env: { ...spec.env, HORIZON_HOME: home, ATLAS_HOME: home }, shell: false, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
      const started = waitForSpawn(child, this.startTimeoutMs);
      record.child = child;
      this.sessions.set(sessionId, record);
      let finished = false;
      const finish = (code, error) => {
        if (finished) return;
        finished = true;
        output.end();
        void cleanupAccount();
        this.sessions.delete(sessionId);
        record.resolveClosed({ code, error });
        this.onLog(error ? `Ошибка запуска ${record.name}: ${error.message}` : `${record.name}: игра завершена${code === 0 ? '' : ` (код ${code})`}.`, error || code !== 0 ? 'error' : 'info');
        this.onChange(false);
      };
      for (const stream of [child.stdout, child.stderr]) stream?.on('data', chunk => {
        if (finished) return;
        output.write(chunk);
        for (const line of String(chunk).split(/\r?\n/).filter(Boolean)) this.onLog(`[${record.name}] ${line}`);
        this.onChange(true);
      });
      child.once('error', error => finish(null, error));
      child.once('close', code => finish(code));
      this.onLog(`Запуск ${record.name} · ${path.basename(spec.executable)}`);
      this.onChange(false);
      await started;
      if (this.sessions.has(sessionId)) record.status = 'running';
      this.store.markLaunched(instanceId);
      this.onChange(false);
      return this.snapshot();
    } catch (error) {
      // A startup timeout never force-terminates a process that might already hold a world.
      if (record && this.sessions.has(record.id)) { record.status = 'running'; this.onChange(false); }
      else { output?.end(); await cleanupAccount(); }
      throw error;
    } finally {
      this.pendingHomes.delete(home);
    }
  }

  async launchAnother(instanceId) {
    return this.launch(instanceId, { additional: true });
  }

  helperPath() {
    const runtime = this.store.config.settings.runtimePath;
    const candidates = [path.join(path.dirname(runtime), 'atlas-ctl.exe'), path.join(this.store.projectRoot, 'runtime', 'atlas-ctl.exe'), path.join(path.dirname(runtime), 'horizon-ctl.exe')];
    const helper = candidates.find(candidate => fs.existsSync(candidate));
    if (!helper) throw new Error('Не найден atlas-ctl.exe. Закрой игру её кнопкой выхода.');
    return helper;
  }

  async waitUntilClosed(record, timeoutMs) {
    let timer;
    try {
      return await Promise.race([record.closed, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Игра ещё сохраняется или не отвечает. Дождись выхода либо закрой её окно.')), timeoutMs); })]);
    } finally { clearTimeout(timer); }
  }

  async requestClose(record) {
    // Respect the player's No AI choice: normal OS window close still works without agent control.
    const extras = path.join(record.home, 'atlas-extras.txt');
    const noAi = fs.existsSync(extras) && /^\s*noAi\s*:\s*true\s*$/m.test(fs.readFileSync(extras, 'utf8'));
    let controlError;
    if (!noAi) {
      try {
        const helper = this.helperPath();
        const deadline = Date.now() + Math.min(12000, this.stopTimeoutMs);
        while (this.sessions.has(record.id)) {
          try {
            const result = await this.execFile(helper, ['--socket', record.socketPath, '--timeout', '10', 'quit'], { windowsHide: true, timeout: 12000, maxBuffer: 1024 * 1024 });
            const reply = JSON.parse((result.stdout || '').trim());
            if (!reply.ok) throw new Error(reply.error || 'Клиент не подтвердил выход.');
            return;
          } catch (error) {
            controlError = error;
            if (error.code === 4 || Date.now() >= deadline || fs.existsSync(record.socketPath)) break;
            await new Promise(resolve => setTimeout(resolve, 250));
          }
        }
        if (!this.sessions.has(record.id)) return;
      } catch (error) { controlError = error; }
    }
    if (process.platform === 'win32' && Number.isInteger(record.child?.pid)) {
      // /F is deliberately absent: taskkill asks this one managed window to close normally.
      await this.execFile(path.join(process.env.SystemRoot || 'C:\\Windows', 'System32', 'taskkill.exe'), ['/PID', String(record.child.pid)], { windowsHide: true, timeout: 10000 });
      return;
    }
    throw new Error(`Не удалось завершить игру${controlError ? `: ${controlError.message}` : ''}. Закрой её обычной кнопкой выхода.`);
  }

  async stopRecord(record, restart = false) {
    const before = record.status;
    record.status = restart ? 'restarting' : 'stopping';
    this.onChange(false);
    try {
      await this.requestClose(record);
      await this.waitUntilClosed(record, this.stopTimeoutMs);
    } catch (error) {
      if (this.sessions.has(record.id)) { record.status = before; this.onChange(false); }
      throw error;
    }
  }

  async stop(sessionId) {
    const record = this.pick(sessionId);
    if (record.operation) throw new Error('Дождись текущей операции с этим окном.');
    record.operation = this.stopRecord(record);
    try { await record.operation; return this.snapshot(); }
    finally { record.operation = null; }
  }

  async restart(sessionId) {
    const record = this.pick(sessionId);
    if (record.operation) throw new Error('Дождись текущей операции с этим окном.');
    record.operation = this.stopRecord(record, true);
    try {
      await record.operation;
      return await this.launch(record.instanceId, { home: record.home, isolated: record.isolated });
    } finally { record.operation = null; }
  }
}

module.exports = { GameSessions, createAdditionalHome };
