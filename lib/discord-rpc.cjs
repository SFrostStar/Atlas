const net = require('node:net');
const fs = require('node:fs');
const path = require('node:path');

const APP_ID = /^[0-9]{16,22}$/;
const IMAGE_KEY = /^[a-z0-9_-]{1,64}$/;
const MAX_FRAME = 1024 * 1024;
const DETAILS = 'The Fastest Minecraft Launcher';

function encodeFrame(opcode, value) {
  const body = Buffer.from(JSON.stringify(value), 'utf8');
  const header = Buffer.alloc(8);
  header.writeUInt32LE(opcode, 0);
  header.writeUInt32LE(body.length, 4);
  return Buffer.concat([header, body]);
}

class FrameReader {
  constructor() { this.pending = Buffer.alloc(0); }
  push(chunk) {
    this.pending = Buffer.concat([this.pending, chunk]);
    const frames = [];
    while (this.pending.length >= 8) {
      const length = this.pending.readUInt32LE(4);
      if (length > MAX_FRAME) throw new Error('Discord прислал слишком большой RPC-кадр.');
      if (this.pending.length < 8 + length) break;
      frames.push({ opcode: this.pending.readUInt32LE(0), value: JSON.parse(this.pending.subarray(8, 8 + length).toString('utf8')) });
      this.pending = this.pending.subarray(8 + length);
    }
    return frames;
  }
}

function ipcPaths(platform = process.platform, env = process.env) {
  if (platform === 'win32') return Array.from({ length: 10 }, (_, index) => `\\\\?\\pipe\\discord-ipc-${index}`);
  const directories = [...new Set([env.XDG_RUNTIME_DIR, env.TMPDIR, env.TMP, env.TEMP, '/tmp'].filter(Boolean))];
  return directories.flatMap(directory => Array.from({ length: 10 }, (_, index) => path.join(directory, `discord-ipc-${index}`)));
}

function readPreference(home) {
  try {
    const file = path.join(home, 'atlas-extras.txt');
    if (fs.statSync(file).size > 64 * 1024) return false;
    let enabled = false;
    for (const line of fs.readFileSync(file, 'utf8').split(/\r?\n/)) {
      const match = /^discordPresence:(true|false)\s*$/.exec(line);
      if (match) enabled = match[1] === 'true';
    }
    return enabled;
  } catch { return false; }
}

function activity(version = '0.1', imageKey = 'atlas') {
  return { type: 0, details: DETAILS, state: `v${version.replace(/^v/, '')}`, assets: { large_image: imageKey, large_text: 'Atlas' } };
}

class DiscordPresence {
  constructor(options = {}) {
    this.connect = options.connect || (options => net.createConnection(options));
    this.paths = options.paths || ipcPaths();
    this.pid = options.pid || process.pid;
    this.version = options.version || '0.1';
    this.onStatus = options.onStatus || (() => { });
    this.retryMs = options.retryMs ?? 15000;
    this.handshakeTimeoutMs = options.handshakeTimeoutMs ?? 2000;
    this.replyTimeoutMs = options.replyTimeoutMs ?? 5000;
    this.refreshMs = options.refreshMs ?? 60000;
    this.generation = 0;
    this.nonce = 0;
    this.config = null;
    this.socket = null;
    this.ready = false;
    this.status = { status: 'disabled', connected: false, applicationId: '', error: null };
  }

  snapshot() { return { ...this.status, activity: this.acknowledgedActivity ? { ...this.acknowledgedActivity } : null }; }

  configure(options = {}) {
    const config = {
      enabled: options.enabled === true,
      applicationId: String(options.applicationId || '').trim(),
      imageKey: options.imageKey || 'atlas',
      version: String(options.version || this.version).replace(/^v/, ''),
    };
    if (JSON.stringify(this.config) === JSON.stringify(config)) return this.snapshot();
    this.generation++;
    this.disconnect(true);
    this.config = config;
    this.acknowledgedActivity = null;
    if (!config.enabled) this.setStatus('disabled');
    else if (!APP_ID.test(config.applicationId)) this.setStatus('needs_application_id', 'Нужен Application ID собственного Discord-приложения Atlas.');
    else if (!IMAGE_KEY.test(config.imageKey)) this.setStatus('error', 'Ключ изображения Discord должен содержать только a-z, 0-9, _ и -.');
    else {
      this.setStatus('connecting');
      const generation = this.generation;
      queueMicrotask(() => this.tryPath(0, generation));
    }
    return this.snapshot();
  }

  setStatus(status, error = null) {
    const next = { status, connected: status === 'connected', applicationId: this.config?.applicationId || '', error };
    if (JSON.stringify(next) === JSON.stringify(this.status)) return;
    this.status = next;
    this.onStatus(this.snapshot());
  }

  clearTimers() {
    clearTimeout(this.retryTimer);
    clearTimeout(this.handshakeTimer);
    clearTimeout(this.replyTimer);
    clearTimeout(this.refreshTimer);
  }

  disconnect(clearActivity) {
    this.clearTimers();
    const socket = this.socket;
    const wasReady = this.ready;
    this.socket = null;
    this.ready = false;
    this.pendingNonce = null;
    if (!socket || socket.destroyed) return;
    if (clearActivity && wasReady) {
      try {
        socket.end(encodeFrame(1, { cmd: 'SET_ACTIVITY', args: { pid: this.pid, activity: null }, nonce: `atlas-clear-${++this.nonce}` }));
        const timer = setTimeout(() => socket.destroy(), 150);
        timer.unref?.();
      } catch { socket.destroy(); }
    } else socket.destroy();
  }

  scheduleRetry(generation, error, status = 'waiting_for_discord') {
    if (generation !== this.generation || !this.config?.enabled) return;
    this.disconnect(false);
    this.setStatus(status, error);
    this.retryTimer = setTimeout(() => {
      this.setStatus('connecting');
      this.tryPath(0, generation);
    }, this.retryMs);
    this.retryTimer.unref?.();
  }

  tryPath(index, generation) {
    if (generation !== this.generation || !this.config?.enabled) return;
    if (index >= this.paths.length) {
      this.scheduleRetry(generation, 'Discord не найден. Открой настольный Discord.');
      return;
    }
    let socket;
    try { socket = this.connect({ path: this.paths[index] }); }
    catch { queueMicrotask(() => this.tryPath(index + 1, generation)); return; }
    this.socket = socket;
    this.ready = false;
    const reader = new FrameReader();
    let failed = false;
    const fail = (error, scan = false) => {
      if (failed || generation !== this.generation || this.socket !== socket) return;
      failed = true;
      const wasReady = this.ready;
      this.disconnect(false);
      if (scan && !wasReady) queueMicrotask(() => this.tryPath(index + 1, generation));
      else this.scheduleRetry(generation, String(error?.message || error || 'Discord отключён.').slice(0, 300), 'error');
    };
    this.handshakeTimer = setTimeout(() => fail(new Error('Discord не подтвердил подключение.'), true), this.handshakeTimeoutMs);
    this.handshakeTimer.unref?.();
    socket.once('connect', () => {
      if (generation !== this.generation || this.socket !== socket) return;
      socket.write(encodeFrame(0, { v: 1, client_id: this.config.applicationId }));
    });
    socket.on('error', error => fail(error, true));
    socket.on('close', () => fail(new Error('Соединение с Discord закрыто.'), true));
    socket.on('data', chunk => {
      if (generation !== this.generation || this.socket !== socket) return;
      try {
        for (const { opcode, value } of reader.push(chunk)) {
          if (opcode === 3) { socket.write(encodeFrame(4, value)); continue; }
          if (opcode === 2) { fail(new Error(value.message || 'Discord отклонил Application ID.')); return; }
          if (opcode !== 1) continue;
          if (!this.ready && value.evt === 'READY') {
            clearTimeout(this.handshakeTimer);
            this.ready = true;
            this.sendActivity(generation, socket, fail);
          } else if (value.nonce === this.pendingNonce) {
            clearTimeout(this.replyTimer);
            this.pendingNonce = null;
            if (value.evt === 'ERROR') { fail(new Error(value.data?.message || 'Discord отклонил Rich Presence.')); return; }
            const acknowledged = value.data?.activity || value.data || {};
            this.acknowledgedActivity = {
              applicationName: typeof acknowledged.name === 'string' ? acknowledged.name : null,
              applicationId: typeof acknowledged.application_id === 'string' ? acknowledged.application_id : null,
              details: typeof acknowledged.details === 'string' ? acknowledged.details : null,
              state: typeof acknowledged.state === 'string' ? acknowledged.state : null,
              largeImage: typeof acknowledged.assets?.large_image === 'string' ? acknowledged.assets.large_image : null,
            };
            this.setStatus('connected');
            this.refreshTimer = setTimeout(() => this.sendActivity(generation, socket, fail), this.refreshMs);
            this.refreshTimer.unref?.();
          } else if (!this.ready && value.evt === 'ERROR') {
            fail(new Error(value.data?.message || 'Discord отклонил подключение.'));
          }
        }
      } catch (error) { fail(error); }
    });
  }

  sendActivity(generation, socket, fail) {
    if (generation !== this.generation || this.socket !== socket || !this.ready) return;
    this.pendingNonce = `atlas-${++this.nonce}`;
    socket.write(encodeFrame(1, { cmd: 'SET_ACTIVITY', args: { pid: this.pid, activity: activity(this.config.version, this.config.imageKey) }, nonce: this.pendingNonce }));
    this.replyTimer = setTimeout(() => fail(new Error('Discord не подтвердил Rich Presence.')), this.replyTimeoutMs);
    this.replyTimer.unref?.();
  }

  dispose() {
    this.generation++;
    this.disconnect(true);
    this.config = null;
    this.setStatus('disabled');
  }
}

module.exports = { DiscordPresence, FrameReader, encodeFrame, ipcPaths, readPreference, activity };
