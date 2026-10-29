const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFile } = require('node:child_process');
const { promisify } = require('node:util');

const runFile = promisify(execFile);
const MICROSOFT = 'https://login.microsoftonline.com/consumers/oauth2/v2.0';
const MINECRAFT = 'https://api.minecraftservices.com';
const SCOPE = 'XboxLive.signin offline_access';
const GUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const NICK = /^[a-zA-Z0-9_]{3,16}$/;

const MESSAGES = {
  not_configured: 'Microsoft sign-in needs the Atlas application Client ID. See docs/ACCOUNTS.md.',
  storage_unavailable: 'Windows secure storage is unavailable. Microsoft tokens have not been saved.',
  storage_locked: 'This account cannot be decrypted by the current Windows user. Sign in again.',
  network: 'The account service did not respond. Check your connection and try again.',
  invalid_response: 'The account service returned an invalid response.',
  invalid_client: 'Microsoft refused the Atlas app registration. Check its Client ID and public client settings.',
  authorization_declined: 'Microsoft sign-in was declined.',
  expired_token: 'The sign-in code expired. Start sign-in again.',
  bad_verification_code: 'Microsoft refused this sign-in code. Start sign-in again.',
  invalid_grant: 'Your Microsoft sign-in expired. Sign in again.',
  xbox_profile: 'Create an Xbox profile at xbox.com, then sign in again.',
  xbox_region: 'Xbox Live is unavailable in this region.',
  xbox_family: 'This Xbox account needs family or adult verification at xbox.com.',
  xbox: 'Xbox Live refused the sign-in.',
  app_not_approved: 'Minecraft refused the Atlas app registration. Its Minecraft API access may need approval.',
  rate_limited: 'Too many sign-in requests. Wait a minute and try again.',
  minecraft: 'Minecraft services refused the sign-in.',
  no_ownership: 'This Microsoft account does not own Minecraft: Java Edition.',
  no_profile: 'Create your Minecraft: Java Edition profile at minecraft.net, then sign in again.',
  invalid_name: 'A local name must contain 3–16 letters, digits or underscores.',
  not_found: 'This account no longer exists.',
  session_security: 'Atlas could not protect the game session file. The game was not started.',
};

class AccountError extends Error {
  constructor(code) { super(MESSAGES[code] || MESSAGES.minecraft); this.name = 'AccountError'; this.code = code; }
}
const fail = code => new AccountError(code);
const publicAccount = account => ({ id: account.id, type: account.type, name: account.name, ...(account.uuid ? { uuid: account.uuid } : {}) });
const aborted = () => new DOMException('Cancelled', 'AbortError');
function delay(ms, signal) {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) { reject(aborted()); return; }
    const finish = () => { signal?.removeEventListener('abort', cancel); resolve(); };
    const timer = setTimeout(finish, ms);
    const cancel = () => { clearTimeout(timer); signal.removeEventListener('abort', cancel); reject(aborted()); };
    signal?.addEventListener('abort', cancel, { once: true });
  });
}

async function protectSessionDirectory(directory) {
  if (process.platform !== 'win32') { await fs.promises.chmod(directory, 0o700); return; }
  // Only a random directory created by Atlas is touched. Never interpolate a path into shell code.
  const { stdout } = await runFile('whoami.exe', ['/user', '/fo', 'csv', '/nh'], { windowsHide: true, timeout: 5000 });
  const sid = stdout.match(/S-1-\d+(?:-\d+)+/)?.[0];
  if (!sid) throw fail('session_security');
  await runFile('icacls.exe', [directory, '/inheritance:r', '/grant:r', `*${sid}:(OI)(CI)F`], { windowsHide: true, timeout: 5000 });
}

class AccountManager {
  constructor({ dataRoot, safeStorage, clientId = '', openExternal = async () => {}, onChange = () => {}, fetch = globalThis.fetch, now = Date.now, sleep = delay, protectDirectory = protectSessionDirectory, requestTimeoutMs = 15000 } = {}) {
    if (!path.isAbsolute(dataRoot || '')) throw new Error('Account dataRoot must be an absolute path.');
    this.root = path.resolve(dataRoot);
    this.file = path.join(this.root, 'accounts.json');
    this.safeStorage = safeStorage;
    this.clientId = typeof clientId === 'string' ? clientId.trim() : '';
    this.openExternal = openExternal;
    this.onChange = onChange;
    this.fetch = fetch;
    this.now = now;
    this.sleep = sleep;
    this.protectDirectory = protectDirectory;
    this.requestTimeoutMs = requestTimeoutMs;
    this.status = 'idle';
    this.device = null;
    this.error = null;
    this.flow = null;
    this.refreshes = new Map();
    this.cleanups = new Set();
    fs.mkdirSync(this.root, { recursive: true });
    this.removeExpiredSessions();
    if (fs.existsSync(this.file)) {
      try {
        if (fs.statSync(this.file).size > 1024 * 1024) throw new Error();
        this.data = JSON.parse(fs.readFileSync(this.file, 'utf8'));
        if (this.data.schema !== 1 || !Array.isArray(this.data.accounts) || this.data.accounts.length > 100) throw new Error();
        const ids = new Set();
        for (const a of this.data.accounts) {
          if (!a || typeof a.id !== 'string' || ids.has(a.id) || !NICK.test(a.name) || !['local', 'microsoft'].includes(a.type)) throw new Error();
          if (a.type === 'microsoft' && (!/^[0-9a-f]{32}$/i.test(a.uuid) || typeof a.vault !== 'string')) throw new Error();
          ids.add(a.id);
        }
        if (!ids.has(this.data.selectedId)) this.data.selectedId = this.data.accounts[0]?.id || null;
      } catch { throw new Error('Cannot read accounts.json. The existing file has been preserved.'); }
    } else {
      const local = { id: crypto.randomUUID(), type: 'local', name: 'Player' };
      this.data = { schema: 1, selectedId: local.id, accounts: [local] };
      this.save();
    }
  }

  snapshot() {
    return {
      accounts: this.data.accounts.map(publicAccount), selectedId: this.data.selectedId,
      configured: GUID.test(this.clientId), storageAvailable: this.secureStorageAvailable(),
      status: this.status, device: this.device ? { ...this.device } : null,
      error: this.error ? { ...this.error } : null,
    };
  }
  removeExpiredSessions() {
    const sessions = path.join(this.root, 'auth-sessions');
    try {
      if (fs.lstatSync(sessions).isSymbolicLink()) return;
      for (const entry of fs.readdirSync(sessions, { withFileTypes: true })) {
        if (!entry.isDirectory() || !/^session-[a-zA-Z0-9]{6}$/.test(entry.name)) continue;
        const directory = path.join(sessions, entry.name);
        if (fs.lstatSync(directory).isSymbolicLink() || this.now() - fs.statSync(directory).mtimeMs < 120000) continue;
        const file = path.join(directory, 'session.json');
        // Unlink only this known file, never recursively delete a directory or follow links.
        try { fs.unlinkSync(file); } catch (error) { if (error.code !== 'ENOENT') continue; }
        try { fs.rmdirSync(directory); } catch { /* Unknown files are preserved. */ }
      }
    } catch { /* A failed cleanup never destroys account data or blocks local play. */ }
  }
  changed() { this.onChange(this.snapshot()); }
  secureStorageAvailable() { try { return !!this.safeStorage?.isEncryptionAvailable(); } catch { return false; } }
  save() {
    const temporary = `${this.file}.${crypto.randomUUID()}.tmp`;
    try {
      fs.writeFileSync(temporary, `${JSON.stringify(this.data, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
      fs.renameSync(temporary, this.file);
    } finally { if (fs.existsSync(temporary)) fs.unlinkSync(temporary); }
  }
  commit(next) { const old = this.data; this.data = next; try { this.save(); } catch (error) { this.data = old; throw error; } this.changed(); return this.snapshot(); }
  addLocal(name) {
    if (typeof name !== 'string' || !NICK.test(name.trim())) throw fail('invalid_name');
    name = name.trim();
    const existing = this.data.accounts.find(a => a.type === 'local' && a.name.toLowerCase() === name.toLowerCase());
    if (existing) return this.select(existing.id);
    const account = { id: crypto.randomUUID(), type: 'local', name };
    return this.commit({ ...this.data, accounts: [...this.data.accounts, account], selectedId: account.id });
  }
  select(id) {
    if (!this.data.accounts.some(a => a.id === id)) throw fail('not_found');
    return this.commit({ ...this.data, selectedId: id });
  }
  remove(id) {
    if (!this.data.accounts.some(a => a.id === id)) throw fail('not_found');
    const accounts = this.data.accounts.filter(a => a.id !== id);
    if (!accounts.length) accounts.push({ id: crypto.randomUUID(), type: 'local', name: 'Player' });
    return this.commit({ ...this.data, accounts, selectedId: this.data.selectedId === id ? accounts[0].id : this.data.selectedId });
  }
  encrypt(secrets) {
    if (!this.secureStorageAvailable()) throw fail('storage_unavailable');
    try { return this.safeStorage.encryptString(JSON.stringify(secrets)).toString('base64'); }
    catch { throw fail('storage_unavailable'); }
  }
  decrypt(account) {
    if (!this.secureStorageAvailable()) throw fail('storage_unavailable');
    try {
      const secrets = JSON.parse(this.safeStorage.decryptString(Buffer.from(account.vault, 'base64')));
      if (typeof secrets.refreshToken !== 'string' || typeof secrets.minecraftToken !== 'string' || !Number.isFinite(secrets.expiresAt)) throw new Error();
      return secrets;
    } catch { throw fail('storage_locked'); }
  }
  async request(url, { method = 'GET', json, form, token, signal } = {}) {
    const timeout = AbortSignal.timeout(this.requestTimeoutMs);
    const headers = { Accept: 'application/json' };
    let body;
    if (json) { headers['Content-Type'] = 'application/json'; body = JSON.stringify(json); }
    if (form) { headers['Content-Type'] = 'application/x-www-form-urlencoded'; body = new URLSearchParams(form).toString(); }
    if (token) headers.Authorization = `Bearer ${token}`;
    try {
      const response = await this.fetch(url, { method, headers, body, redirect: 'error', signal: signal ? AbortSignal.any([signal, timeout]) : timeout });
      const reader = response.body?.getReader();
      if (!reader) throw fail('invalid_response');
      const chunks = []; let size = 0;
      try {
        for (;;) {
          const chunk = await reader.read(); if (chunk.done) break;
          size += chunk.value.length;
          if (size > 1024 * 1024) { await reader.cancel(); throw fail('invalid_response'); }
          chunks.push(Buffer.from(chunk.value));
        }
      } finally { reader.releaseLock(); }
      let value; try { value = JSON.parse(Buffer.concat(chunks).toString('utf8')); } catch { throw fail('invalid_response'); }
      return { status: response.status, value };
    } catch (error) {
      if (signal?.aborted) throw aborted();
      if (error instanceof AccountError) throw error;
      throw fail('network');
    }
  }
  oauthFailure(value) {
    const code = value?.error;
    return fail(['invalid_client', 'authorization_declined', 'expired_token', 'bad_verification_code', 'invalid_grant'].includes(code) ? code : code === 'unauthorized_client' ? 'invalid_client' : 'minecraft');
  }
  cancelSignIn() {
    this.flow?.controller.abort(); this.flow = null;
    this.status = 'idle'; this.device = null; this.error = null; this.changed();
    return this.snapshot();
  }
  async startSignIn() {
    this.cancelSignIn();
    if (!GUID.test(this.clientId)) throw fail('not_configured');
    if (!this.secureStorageAvailable()) throw fail('storage_unavailable');
    const flow = { controller: new AbortController() };
    this.flow = flow; this.status = 'starting'; this.changed();
    try {
      const { status, value } = await this.request(`${MICROSOFT}/devicecode`, { method: 'POST', form: { client_id: this.clientId, scope: SCOPE }, signal: flow.controller.signal });
      if (status !== 200) throw this.oauthFailure(value);
      if (typeof value.device_code !== 'string' || typeof value.user_code !== 'string' || !Number.isFinite(value.expires_in) || value.expires_in <= 0) throw fail('invalid_response');
      const url = new URL(value.verification_uri || 'https://www.microsoft.com/link');
      if (url.protocol !== 'https:' || !['www.microsoft.com', 'microsoft.com', 'login.microsoftonline.com'].includes(url.hostname) || url.port || url.username || url.password) throw fail('invalid_response');
      this.device = { userCode: value.user_code, verificationUri: url.href, expiresAt: this.now() + Math.min(value.expires_in, 3600) * 1000 };
      this.status = 'waiting'; this.changed();
      try { await this.openExternal(url.href); } catch { /* The visible link and code still allow sign-in. */ }
      if (this.flow !== flow || flow.controller.signal.aborted) return this.snapshot();
      flow.task = this.pollSignIn(flow, value).catch(error => this.flowFailed(flow, error));
      return this.snapshot();
    } catch (error) {
      this.flowFailed(flow, error);
      if (error.name === 'AbortError') return this.snapshot();
      throw error;
    }
  }
  flowFailed(flow, error) {
    if (this.flow !== flow) return;
    this.flow = null; this.device = null;
    if (error.name === 'AbortError') { this.status = 'idle'; this.error = null; }
    else { this.status = 'error'; this.error = { code: error.code || 'minecraft', message: MESSAGES[error.code] || MESSAGES.minecraft }; }
    this.changed();
  }
  async pollSignIn(flow, code) {
    const signal = flow.controller.signal;
    let interval = Math.max(1, Math.min(Number(code.interval) || 5, 60)) * 1000;
    const expiresAt = this.device.expiresAt;
    for (;;) {
      await this.sleep(interval, signal);
      if (signal.aborted) throw aborted();
      if (this.now() >= expiresAt) throw fail('expired_token');
      const { status, value } = await this.request(`${MICROSOFT}/token`, { method: 'POST', form: { client_id: this.clientId, grant_type: 'urn:ietf:params:oauth:grant-type:device_code', device_code: code.device_code }, signal });
      if (value.error === 'authorization_pending') continue;
      if (value.error === 'slow_down') { interval += 5000; continue; }
      if (status !== 200 || typeof value.access_token !== 'string' || typeof value.refresh_token !== 'string') throw this.oauthFailure(value);
      this.status = 'verifying'; this.device = null; this.changed();
      const result = await this.minecraftLogin(value.access_token, signal);
      if (signal.aborted || this.flow !== flow) throw aborted();
      const account = { id: `msa-${result.uuid}`, type: 'microsoft', name: result.name, uuid: result.uuid, vault: this.encrypt({ refreshToken: value.refresh_token, minecraftToken: result.token, expiresAt: result.expiresAt, xuid: result.xuid }) };
      this.commit({ ...this.data, accounts: [...this.data.accounts.filter(a => a.id !== account.id), account], selectedId: account.id });
      this.flow = null; this.status = 'idle'; this.error = null; this.changed();
      return;
    }
  }
  async minecraftLogin(msToken, signal) {
    let reply = await this.request('https://user.auth.xboxlive.com/user/authenticate', { method: 'POST', signal, json: { Properties: { AuthMethod: 'RPS', SiteName: 'user.auth.xboxlive.com', RpsTicket: `d=${msToken}` }, RelyingParty: 'http://auth.xboxlive.com', TokenType: 'JWT' } });
    if (reply.status !== 200 || typeof reply.value.Token !== 'string') throw fail('xbox');
    reply = await this.request('https://xsts.auth.xboxlive.com/xsts/authorize', { method: 'POST', signal, json: { Properties: { SandboxId: 'RETAIL', UserTokens: [reply.value.Token] }, RelyingParty: 'rp://api.minecraftservices.com/', TokenType: 'JWT' } });
    if (reply.value.XErr) throw fail(({ 2148916233: 'xbox_profile', 2148916235: 'xbox_region', 2148916236: 'xbox_family', 2148916237: 'xbox_family', 2148916238: 'xbox_family' })[reply.value.XErr] || 'xbox');
    const claims = reply.value.DisplayClaims?.xui?.[0];
    if (reply.status !== 200 || typeof reply.value.Token !== 'string' || typeof claims?.uhs !== 'string') throw fail('xbox');
    const xuid = typeof claims.xid === 'string' ? claims.xid : null;
    reply = await this.request(`${MINECRAFT}/authentication/login_with_xbox`, { method: 'POST', signal, json: { identityToken: `XBL3.0 x=${claims.uhs};${reply.value.Token}` } });
    if (reply.status === 403) throw fail('app_not_approved');
    if (reply.status === 429) throw fail('rate_limited');
    if (reply.status !== 200 || typeof reply.value.access_token !== 'string' || !reply.value.access_token) throw fail('minecraft');
    const token = reply.value.access_token;
    const expiresAt = this.now() + Math.max(0, Number(reply.value.expires_in) || 0) * 1000;
    const profile = await this.verifyOwnership(token, signal);
    return { ...profile, token, expiresAt, xuid };
  }
  async verifyOwnership(token, signal) {
    let reply = await this.request(`${MINECRAFT}/entitlements/mcstore`, { token, signal });
    if (reply.status === 401) throw fail('invalid_grant');
    if (reply.status !== 200) throw fail('minecraft');
    if (!Array.isArray(reply.value.items)) throw fail('invalid_response');
    if (!reply.value.items.some(item => item && ['game_minecraft', 'product_minecraft'].includes(item.name))) throw fail('no_ownership');
    reply = await this.request(`${MINECRAFT}/minecraft/profile`, { token, signal });
    if (reply.status === 404) throw fail('no_profile');
    if (reply.status !== 200 || !NICK.test(reply.value.name) || !/^[0-9a-f]{32}$/i.test(reply.value.id)) throw fail('no_profile');
    return { uuid: reply.value.id.toLowerCase(), name: reply.value.name };
  }
  async refresh(account) {
    const secrets = this.decrypt(account);
    let next = secrets;
    let profile;
    if (secrets.expiresAt - this.now() < 5 * 60 * 1000) {
      if (!GUID.test(this.clientId)) throw fail('not_configured');
      const reply = await this.request(`${MICROSOFT}/token`, { method: 'POST', form: { client_id: this.clientId, scope: SCOPE, grant_type: 'refresh_token', refresh_token: secrets.refreshToken } });
      if (reply.status !== 200 || typeof reply.value.access_token !== 'string') throw this.oauthFailure(reply.value);
      const minecraft = await this.minecraftLogin(reply.value.access_token);
      profile = minecraft;
      next = { refreshToken: reply.value.refresh_token || secrets.refreshToken, minecraftToken: minecraft.token, expiresAt: minecraft.expiresAt, xuid: minecraft.xuid };
    } else profile = await this.verifyOwnership(secrets.minecraftToken);
    if (profile.uuid !== account.uuid) throw fail('invalid_grant');
    const current = this.data.accounts.find(a => a.id === account.id);
    if (!current) throw fail('not_found');
    const updated = { ...current, name: profile.name, vault: this.encrypt(next) };
    this.commit({ ...this.data, accounts: this.data.accounts.map(a => a.id === account.id ? updated : a) });
    return { account: updated, secrets: next };
  }
  async prepareLaunch(home) {
    if (!path.isAbsolute(home || '')) throw fail('session_security');
    const selected = this.data.accounts.find(a => a.id === this.data.selectedId);
    if (!selected) throw fail('not_found');
    if (selected.type === 'local') return { args: ['--name', selected.name], cleanup: async () => {} };
    let pending = this.refreshes.get(selected.id);
    if (!pending) {
      pending = this.refresh(selected).finally(() => this.refreshes.delete(selected.id));
      this.refreshes.set(selected.id, pending);
    }
    const { account, secrets } = await pending;
    const sessions = path.join(this.root, 'auth-sessions');
    await fs.promises.mkdir(sessions, { recursive: true, mode: 0o700 });
    if ((await fs.promises.lstat(sessions)).isSymbolicLink()) throw fail('session_security');
    const directory = await fs.promises.mkdtemp(path.join(sessions, 'session-'));
    const file = path.join(directory, 'session.json');
    const cleanup = async () => {
      clearTimeout(timer);
      this.cleanups.delete(cleanup);
      try { await fs.promises.unlink(file); } catch (error) { if (error.code !== 'ENOENT') throw error; }
      try { await fs.promises.rmdir(directory); } catch (error) { if (error.code !== 'ENOENT') throw error; }
    };
    let timer;
    try {
      await this.protectDirectory(directory);
      await fs.promises.writeFile(file, JSON.stringify({ name: account.name, uuid: account.uuid, access_token: secrets.minecraftToken, user_type: 'msa', xuid: secrets.xuid }), { flag: 'wx', mode: 0o600 });
      this.cleanups.add(cleanup);
      timer = setTimeout(() => cleanup().catch(() => {}), 120000); timer.unref();
      return { args: ['--session-file', file], cleanup };
    } catch {
      await cleanup().catch(() => {});
      throw fail('session_security');
    }
  }
  dispose() { this.cancelSignIn(); for (const cleanup of this.cleanups) cleanup().catch(() => {}); }
}

module.exports = { AccountManager, AccountError, protectSessionDirectory };
