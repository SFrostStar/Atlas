function waitForSpawn(child, timeoutMs = 15000) {
  return new Promise((resolve, reject) => {
    const cleanup = () => {
      clearTimeout(timer);
      child.removeListener('spawn', onSpawn);
      child.removeListener('error', onError);
      child.removeListener('close', onClose);
    };
    const onSpawn = () => { cleanup(); resolve(); };
    const onError = (error) => { cleanup(); reject(error); };
    const onClose = (code) => { cleanup(); reject(new Error(`Клиент завершился до запуска (код ${code}).`)); };
    const timer = setTimeout(() => {
      cleanup();
      reject(new Error('Клиент не подтвердил запуск. Проверь журнал запуска.'));
    }, timeoutMs);
    child.once('spawn', onSpawn);
    child.once('error', onError);
    child.once('close', onClose);
  });
}

module.exports = { waitForSpawn };
