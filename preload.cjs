const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('atlas', {
  getState: () => ipcRenderer.invoke('atlas:get-state'),
  createInstance: (input) => ipcRenderer.invoke('atlas:create-instance', input),
  selectInstance: (id) => ipcRenderer.invoke('atlas:select-instance', id),
  deleteInstance: (id) => ipcRenderer.invoke('atlas:delete-instance', id),
  launch: (id) => ipcRenderer.invoke('atlas:launch', id),
  launchAnother: (id) => ipcRenderer.invoke('atlas:launch-another', id),
  stopGame: (sessionId) => ipcRenderer.invoke('atlas:stop-game', sessionId),
  restartGame: (sessionId) => ipcRenderer.invoke('atlas:restart-game', sessionId),
  openFolder: (kind, id) => ipcRenderer.invoke('atlas:open-folder', kind, id),
  chooseRuntime: () => ipcRenderer.invoke('atlas:choose-runtime'),
  chooseSource: () => ipcRenderer.invoke('atlas:choose-source'),
  saveSettings: (patch) => ipcRenderer.invoke('atlas:save-settings', patch),
  setInstanceMemory: (id, memoryMb) => ipcRenderer.invoke('atlas:set-instance-memory', id, memoryMb),
  addLocalAccount: name => ipcRenderer.invoke('atlas:add-local-account', name),
  selectAccount: id => ipcRenderer.invoke('atlas:select-account', id),
  removeAccount: id => ipcRenderer.invoke('atlas:remove-account', id),
  startSignIn: () => ipcRenderer.invoke('atlas:start-signin'),
  cancelSignIn: () => ipcRenderer.invoke('atlas:cancel-signin'),
  openMicrosoft: () => ipcRenderer.invoke('atlas:open-microsoft'),
  refreshDiscord: () => ipcRenderer.invoke('atlas:refresh-discord'),
  setGameOptions: (id, patch) => ipcRenderer.invoke('atlas:set-game-options', id, patch),
  windowControl: (action) => ipcRenderer.invoke('atlas:window-control', action),
  onGameEvent: (callback) => {
    const listener = (_event, state) => callback(state);
    ipcRenderer.on('atlas:game-event', listener);
    return () => ipcRenderer.removeListener('atlas:game-event', listener);
  },
});
