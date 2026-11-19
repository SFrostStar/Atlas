const fs = require('node:fs/promises');
const path = require('node:path');
const { guard } = require('./release-updater.cjs');

async function openAccountSwitch(dataRoot, sessionId, initial, getAccounts, { actions, home } = {}) {
  const root = guard(dataRoot, path.join(dataRoot, 'control', `ias-${sessionId}`));
  await fs.mkdir(root, { recursive: true });
  const list = path.join(root, 'accounts.json');
  const request = path.join(root, 'request.json');
  const reply = path.join(root, 'reply.json');
  const sessionRoot = guard(dataRoot, path.join(dataRoot, 'auth-sessions'));
  let currentId = initial.selectedId;
  let lastRequest;
  let polling = false;
  let closed = false;
  let currentSignIn;
  let published;
  const sessions = new Set();
  const write = async (file, value) => {
    const temp = guard(root, file + '.tmp');
    await fs.writeFile(temp, JSON.stringify(value), { flag: 'wx' });
    try { await fs.rename(temp, guard(root, file)); }
    finally { await fs.unlink(temp).catch(() => {}); }
  };
  async function publish(data = getAccounts()) {
    const accounts = data.accounts.map(({ id, name, type, uuid }) => ({ id, name, type, uuid }));
    const active = Boolean(currentSignIn && ['starting', 'waiting', 'verifying'].includes(data.status));
    const microsoft = {
      configured: Boolean(data.configured && data.storageAvailable),
      active,
      userCode: active ? data.device?.userCode : undefined,
      expiresAt: active ? data.device?.expiresAt : undefined,
      error: currentSignIn ? data.error?.code : undefined,
    };
    const next = { currentId, accounts, microsoft };
    const snapshot = JSON.stringify(next);
    if (snapshot !== published) {
      await write(list, next);
      published = snapshot;
    }
    return accounts;
  }
  await publish();
  async function poll() {
    const accounts = await publish();
    let stat;
    const file = guard(root, request);
    try { stat = await fs.lstat(file); }
    catch (error) { if (error.code === 'ENOENT') return; throw error; }
    if (!stat.isFile() || stat.size > 4096) return;
    const text = await fs.readFile(file, 'utf8');
    await fs.unlink(file);
    const packet = JSON.parse(text);
    const { requestId, accountId, action = 'switch', name, offline, direction, targetId } = packet;
    if (typeof requestId !== 'string' || !/^[a-zA-Z0-9-]{1,40}$/.test(requestId) || requestId === lastRequest) return;
    lastRequest = requestId;
    const account = accounts.find(item => item.id === accountId);
    let response = { requestId };
    try {
      if (action === 'switch') {
        if (!account) response.error = 'atlas.ias.invalid_name';
        else if (!actions && (account.type !== 'local' || initial.accounts.find(item => item.id === currentId)?.type === 'microsoft')) {
          response.error = 'atlas.ias.local_only';
        } else if (account.type === 'microsoft' && offline !== true) {
          const session = await actions.prepareLaunch(home, account.id);
          sessions.add(session.cleanup);
          const refreshed = getAccounts().accounts.find(item => item.id === account.id);
          if (!refreshed) throw new Error('The account was removed during sign-in.');
          response = { requestId, name: refreshed.name, accountId: account.id, sessionPath: guard(sessionRoot, session.args[1]) };
          currentId = account.id;
        } else {
          response = { requestId, name: account.name, ...(actions ? { accountId: account.id } : {}) };
          currentId = account.id;
        }
      } else if (actions) {
        if (action === 'add') {
          const data = actions.addLocal(name, { select: false });
          response.accountId = data.accounts.find(item => item.type === 'local' && item.name.toLowerCase() === name.trim().toLowerCase())?.id;
        } else if (account && action === 'edit') {
          actions.editLocal(account.id, name);
          response.accountId = account.id;
        } else if (account && action === 'delete') {
          actions.remove(account.id);
        } else if (account && action === 'move') {
          actions.move(account.id, direction, targetId);
          response.accountId = account.id;
        } else if (action === 'microsoft') {
          const data = getAccounts();
          if (!data.configured || !data.storageAvailable) response.error = 'atlas.ias.microsoft_setup';
          else if (['starting', 'waiting', 'verifying'].includes(data.status)) response.error = 'atlas.ias.failed';
          else {
            const start = actions.startSignIn();
            currentSignIn = actions.flow;
            await start;
            if (closed && currentSignIn && actions.flow === currentSignIn) actions.cancelSignIn();
          }
        } else if (action === 'cancel-microsoft') {
          if (currentSignIn && actions.flow === currentSignIn) actions.cancelSignIn();
          currentSignIn = null;
        } else response.error = 'atlas.ias.failed';
      } else response.error = 'atlas.ias.failed';
    } catch (error) {
      response = { requestId, error: error.code === 'invalid_name' ? 'atlas.ias.invalid_name'
        : error.code === 'not_configured' ? 'atlas.ias.microsoft_setup' : 'atlas.ias.failed' };
    }
    if (!closed) {
      await publish();
      await write(reply, response);
    }
  }
  let pending = Promise.resolve();
  const timer = setInterval(() => {
    if (polling) return;
    polling = true;
    pending = poll().catch(() => {}).finally(() => { polling = false; });
  }, 150);
  timer.unref();
  return {
    env: { ATLAS_ACCOUNT_SWITCH_FILE: list, ATLAS_ACCOUNT_SWITCH_REQUEST: request, ATLAS_ACCOUNT_SWITCH_REPLY: reply, ATLAS_ACCOUNT_SWITCH_SESSION_ROOT: sessionRoot },
    async close() {
      closed = true;
      clearInterval(timer);
      if (currentSignIn && actions?.flow === currentSignIn) actions.cancelSignIn();
      await pending;
      await Promise.allSettled([...sessions].map(cleanup => cleanup()));
      for (const file of [list, request, reply]) await fs.unlink(guard(root, file)).catch(() => {});
      await fs.rmdir(root).catch(() => {});
    },
  };
}

module.exports = { openAccountSwitch };
