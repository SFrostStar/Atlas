const { test } = require('node:test');
const assert = require('node:assert/strict');
const net = require('node:net');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const crypto = require('node:crypto');
const { DiscordPresence, FrameReader, encodeFrame, ipcPaths, readPreference, activity } = require('../lib/discord-rpc.cjs');
const APP_ID = '1234567890123456789';
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(check, timeout = 2000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) { if (check()) return; await delay(10); }
  throw new Error('Timed out waiting for local RPC fixture');
}
async function fakeDiscord(t, onFrame) {
  const endpoint = process.platform === 'win32'
    ? `\\\\?\\pipe\\atlas-rpc-test-${process.pid}-${crypto.randomUUID()}`
    : path.join(os.tmpdir(), `atlas-rpc-test-${crypto.randomUUID()}.sock`);
  const sockets = new Set();
  const server = net.createServer(socket => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
    socket.on('error', () => {});
    const reader = new FrameReader();
    socket.on('data', chunk => {
      for (const frame of reader.push(chunk)) onFrame(frame, socket);
    });
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(endpoint, resolve); });
  t.after(async () => { for (const socket of sockets) socket.destroy(); await new Promise(resolve => server.close(resolve)); });
  return endpoint;
}

test('Discord wire framing handles fragmented UTF-8 frames and bounds frame size', () => {
  const reader = new FrameReader();
  const frame = encodeFrame(1, { text: 'Атлас' });
  assert.equal(frame.readUInt32LE(4), Buffer.byteLength(JSON.stringify({ text: 'Атлас' })));
  assert.deepEqual(reader.push(frame.subarray(0, 6)), []);
  assert.deepEqual(reader.push(frame.subarray(6, 10)), []);
  assert.deepEqual(reader.push(Buffer.concat([frame.subarray(10), encodeFrame(3, { nonce: 'ping' })])), [
    { opcode: 1, value: { text: 'Атлас' } }, { opcode: 3, value: { nonce: 'ping' } },
  ]);
  const invalid = Buffer.alloc(8);
  invalid.writeUInt32LE(1024 * 1024 + 1, 4);
  assert.throws(() => reader.push(invalid), /слишком большой/);
  assert.equal(ipcPaths('win32')[0], '\\\\?\\pipe\\discord-ipc-0');
});

test('missing opt-in or own Application ID never contacts Discord', () => {
  let connections = 0;
  const client = new DiscordPresence({ connect: () => { connections++; throw Error('unexpected connect'); } });
  assert.equal(client.configure({ enabled: false, applicationId: APP_ID }).status, 'disabled');
  assert.equal(client.configure({ enabled: true, applicationId: '' }).status, 'needs_application_id');
  assert.equal(connections, 0);
  client.dispose();
});

test('only the explicit selected game preference opts in', t => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'atlas-rpc-pref-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  assert.equal(readPreference(directory), false);
  fs.writeFileSync(path.join(directory, 'atlas-extras.txt'), 'unknown:true\ndiscordPresence:false\n');
  assert.equal(readPreference(directory), false);
  fs.writeFileSync(path.join(directory, 'atlas-extras.txt'), 'discordPresence:true\n');
  assert.equal(readPreference(directory), true);
  fs.writeFileSync(path.join(directory, 'atlas-extras.txt'), 'discordPresence:true\ndiscordPresence:false\n');
  assert.equal(readPreference(directory), false);
});

test('real IPC sends Atlas activity only after READY, handles pings and clears on disable', async t => {
  const received = [];
  let gotPong = false;
  let acknowledge;
  const endpoint = await fakeDiscord(t, ({ opcode, value }, socket) => {
    received.push({ opcode, value });
    if (opcode === 0) {
      assert.deepEqual(value, { v: 1, client_id: APP_ID });
      socket.write(encodeFrame(3, { ping: 1 }));
      const ready = encodeFrame(1, { evt: 'READY', data: { v: 1 } });
      socket.write(ready.subarray(0, 5));
      setTimeout(() => socket.write(ready.subarray(5)), 5);
    } else if (opcode === 4) gotPong = true;
    else if (value.cmd === 'SET_ACTIVITY' && value.args.activity) {
      acknowledge = () => socket.write(encodeFrame(1, { cmd: 'SET_ACTIVITY', nonce: value.nonce, evt: null, data: { name: 'Atlas', application_id: APP_ID, ...value.args.activity } }));
    }
  });
  const client = new DiscordPresence({ paths: [endpoint], retryMs: 5000, pid: 1234 });
  t.after(() => client.dispose());
  client.configure({ enabled: true, applicationId: APP_ID, version: '0.1' });
  await until(() => acknowledge);
  assert.equal(client.snapshot().connected, false, 'READY alone is not a successful activity');
  const set = received.find(frame => frame.value.cmd === 'SET_ACTIVITY');
  assert.deepEqual(set.value.args, { pid: 1234, activity: activity('0.1', 'atlas') });
  assert.equal(set.value.args.activity.details, 'The Fastest Minecraft Launcher');
  acknowledge();
  await until(() => client.snapshot().connected);
  assert.equal(gotPong, true);
  assert.equal(client.snapshot().activity.applicationName, 'Atlas');
  client.configure({ enabled: false, applicationId: APP_ID });
  await until(() => received.some(frame => frame.value.cmd === 'SET_ACTIVITY' && frame.value.args.activity === null));
  assert.equal(client.snapshot().status, 'disabled');
});

test('Discord activity errors are reported as errors rather than successful presence', async t => {
  const endpoint = await fakeDiscord(t, ({ opcode, value }, socket) => {
    if (opcode === 0) socket.write(encodeFrame(1, { evt: 'READY' }));
    else if (value.cmd === 'SET_ACTIVITY') socket.write(encodeFrame(1, { evt: 'ERROR', nonce: value.nonce, data: { code: 4000, message: 'invalid application' } }));
  });
  const client = new DiscordPresence({ paths: [endpoint], retryMs: 5000 });
  t.after(() => client.dispose());
  client.configure({ enabled: true, applicationId: APP_ID });
  await until(() => client.snapshot().status === 'error');
  assert.equal(client.snapshot().connected, false);
  assert.equal(client.snapshot().error, 'invalid application');
});

test('an unresponsive Discord pipe times out and the next IPC pipe is tried', async t => {
  const blackhole = await fakeDiscord(t, () => {});
  const ready = await fakeDiscord(t, ({ opcode, value }, socket) => {
    if (opcode === 0) socket.write(encodeFrame(1, { evt: 'READY' }));
    else if (value.cmd === 'SET_ACTIVITY') socket.write(encodeFrame(1, { nonce: value.nonce, evt: null, data: value.args.activity }));
  });
  const client = new DiscordPresence({ paths: [blackhole, ready], handshakeTimeoutMs: 30, retryMs: 5000 });
  t.after(() => client.dispose());
  client.configure({ enabled: true, applicationId: APP_ID });
  await until(() => client.snapshot().connected);
});
