const test = require('node:test');
const assert = require('node:assert/strict');
const { cargoProgress, compileEstimate } = require('../lib/native-mods.cjs');

test('Cargo progress survives divided output chunks and leaves diagnostics in the log', () => {
  const records = [], diagnostics = [];
  const stream = cargoProgress((...progress) => records.push(progress), text => diagnostics.push(text));
  stream.push('   Compiling atlas-client\n    Buil');
  stream.push('ding [=========> ] 441/443: atlas-client\r    Building [=========> ] 442/');
  stream.push('443: atlas-client(bin)\r\n   Finished');
  stream.finish();
  assert.deepEqual(records, [[441, 443], [442, 443]]);
  assert.equal(diagnostics.join(''), '   Compiling atlas-client\n   Finished\n');
});

test('ETA ignores cached units and leaves time for the last crates on a warm build', () => {
  const results = [];
  const progress = compileEstimate(value => results.push(value), 0, 70_000);
  progress.update(200, 300, 1000);
  assert.equal(results.at(-1).etaAt, null);
  progress.update(250, 300, 6000);
  assert.equal(results.at(-1).etaAt, 80_800);
  progress.update(299, 300, 8000);
  assert.equal(results.at(-1).etaAt, 78_000);
  progress.update(299, 300, 12000);
  assert.equal(results.length, 3);
  progress.update(300, 300, 12000);
  assert.equal(results.at(-1).etaAt, 78_000);
  assert.equal(progress.tail(13000), 5000);
});
