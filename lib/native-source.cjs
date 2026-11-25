const fs = require('node:fs');
const fsp = fs.promises;
const path = require('node:path');
const { Readable, Transform } = require('node:stream');
const { pipeline } = require('node:stream/promises');
const { extractVerifiedArchive, guard, releaseResponse, REPOSITORY } = require('./release-updater.cjs');

const SOURCE_LIMITS = {
  files: 12000, json: 4 * 1024 * 1024, archive: 128 * 1024 * 1024,
  file: 32 * 1024 * 1024, total: 384 * 1024 * 1024,
};
const isDigest = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const isRevision = value => typeof value === 'string' && /^[a-f0-9]{40}$/.test(value);

async function sourceTree(directory) {
  const [key, upstream] = await Promise.all([
    fsp.readFile(path.join(directory, '.atlas-source-key'), 'utf8'),
    fsp.readFile(path.join(directory, 'ATLAS-UPSTREAM.json'), 'utf8'),
  ]);
  const sourceKey = key.trim();
  const revision = JSON.parse(upstream).commit;
  if (!isDigest(sourceKey) || !isRevision(revision)) throw new Error('Invalid Atlas native source build.');
  return { sourceKey, revision };
}

async function nativeSourceInfo(directory, manifest, launcherVersion) {
  if (directory && fs.existsSync(path.join(directory, '.atlas-source-key'))) {
    return { ...await sourceTree(directory), directory };
  }
  if (!manifest) throw new Error('Atlas native source metadata is missing.');
  const info = JSON.parse(await fsp.readFile(manifest, 'utf8'));
  if (info?.schema !== 1 || info.version !== launcherVersion || !/^\d+\.\d+\.\d+$/.test(info.version)
      || !isDigest(info.sourceKey) || !isRevision(info.revision) || !isDigest(info.sha256)
      || !Number.isSafeInteger(info.size) || info.size < 22 || info.size > SOURCE_LIMITS.archive) {
    throw new Error('Invalid Atlas native source metadata.');
  }
  return info;
}

async function downloadSource(info, archive, fetch, report) {
  const signal = AbortSignal.timeout(10 * 60_000);
  const base = `https://github.com/${REPOSITORY}/releases/download`;
  const name = `Atlas-${info.version}-Native-Source.zip`;
  report({ bytesDone: 0, bytesTotal: info.size });
  let response;
  try {
    response = await releaseResponse(fetch, `${base}/atlas-payload-${info.version}/${name}`, signal, info.version);
  } catch (error) {
    if (error.status !== 404) throw error;
    response = await releaseResponse(fetch, `${base}/atlas-${info.version}/${name}`, signal, info.version);
  }
  if (!response.body) throw new Error('Atlas native source download was empty.');
  let received = 0;
  let reported = 0;
  const meter = new Transform({
    transform(chunk, encoding, callback) {
      received += chunk.length;
      if (received > info.size) return callback(new Error('Atlas native source exceeds its declared size.'));
      if (received - reported >= 512 * 1024 || received === info.size) {
        reported = received;
        report({ bytesDone: received, bytesTotal: info.size });
      }
      callback(null, chunk);
    },
  });
  await pipeline(Readable.fromWeb(response.body), meter, fs.createWriteStream(archive, { flags: 'wx' }));
}

async function prepareNativeSource(info, work, base, fetch, report) {
  // The marker is written only after the complete tree has been checked.
  const marker = '.horizon-revision';
  const ready = await fsp.readFile(guard(base, path.join(base, marker)), 'utf8')
    .catch(error => { if (error.code !== 'ENOENT') throw error; return ''; });
  if (ready.trim() === info.revision && fs.existsSync(path.join(base, 'Cargo.toml'))
      && fs.existsSync(path.join(base, 'Cargo.lock')) && fs.existsSync(path.join(base, 'ATLAS-UPSTREAM.json'))) {
    const key = await fsp.readFile(guard(base, path.join(base, '.atlas-source-key')), 'utf8')
      .catch(error => { if (error.code !== 'ENOENT') throw error; return ''; });
    if (key.trim() === info.sourceKey) return;
  }

  const directory = guard(work, path.join(work, 'source'));
  if (info.directory) {
    await fsp.cp(info.directory, directory, { recursive: true });
  } else {
    const archive = guard(work, path.join(work, 'native-source.zip'));
    await downloadSource(info, archive, fetch, report);
    await extractVerifiedArchive(archive, directory, info, SOURCE_LIMITS);
  }
  const actual = await sourceTree(directory);
  if (actual.sourceKey !== info.sourceKey || actual.revision !== info.revision) throw new Error('Atlas native source archive is for a different client.');
  await fsp.access(path.join(directory, 'Cargo.toml'));
  await fsp.access(path.join(directory, 'Cargo.lock'));
  await fsp.writeFile(path.join(directory, marker), info.revision + '\n');
  // Older clients could leave an incomplete cache in this same managed directory.
  if (fs.existsSync(base)) await fsp.rm(guard(base, base), { recursive: true });
  await fsp.rename(directory, base);
}

module.exports = { nativeSourceInfo, prepareNativeSource, SOURCE_LIMITS };
