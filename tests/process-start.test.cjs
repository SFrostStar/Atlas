const { test } = require('node:test');
const assert = require('node:assert/strict');
const { EventEmitter } = require('node:events');
const { waitForSpawn } = require('../lib/process-start.cjs');

test('launch settles on spawn and removes startup listeners', async () => {
  const child = new EventEmitter();
  const started = waitForSpawn(child);
  child.emit('spawn');
  await started;
  assert.equal(child.listenerCount('spawn'), 0);
  assert.equal(child.listenerCount('error'), 0);
  assert.equal(child.listenerCount('close'), 0);
});

test('launch rejects on failure, early exit or missing startup confirmation', async () => {
  const failed = new EventEmitter();
  const spawnFailure = assert.rejects(waitForSpawn(failed), /not found/);
  failed.emit('error', new Error('not found'));
  await spawnFailure;

  const exited = new EventEmitter();
  const earlyExit = assert.rejects(waitForSpawn(exited), /до запуска/);
  exited.emit('close', 7);
  await earlyExit;

  const silent = new EventEmitter();
  await assert.rejects(waitForSpawn(silent, 10), /не подтвердил запуск/);
  assert.equal(silent.listenerCount('close'), 0);
});
