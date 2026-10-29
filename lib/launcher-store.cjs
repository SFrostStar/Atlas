const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const os = require('node:os');
const { migrateLegacyData } = require('./legacy-migration.cjs');

const APPEARANCE = Object.freeze({ language: 'ru', theme: 'dark', accentColor: 'orange', heroStyle: 'landscape', showHeroText: true, reducedMotion: false });
const CHOICES = { language: ['ru', 'en'], theme: ['dark', 'light'], accentColor: ['orange', 'blue', 'green', 'violet', 'rose'], heroStyle: ['landscape', 'sunset', 'minimal'] };
function memoryMaximum(total = os.totalmem()) {
  return Math.max(2048, Math.floor((total / 1024 ** 2 - 2048) / 256) * 256);
}

const INSTANCE_OPTIONS = require('../instance-options.js');
const ACCENTS = new Set(Object.keys(INSTANCE_OPTIONS.colors));
const GAME_OPTIONS = Object.freeze({ fullbright: false, dynamicLights: false, autoReconnect: false, mouseTweaks: false, mouseTweaksWheel: true, sort: false });

function cleanName(value) {
  const name = typeof value === 'string' ? value.trim() : '';
  if (!name || name.length > 48 || /[\x00-\x1f]/.test(name)) {
    throw new Error('Укажи название сборки: от 1 до 48 символов.');
  }
  return name;
}

function absoluteFolder(value, label) {
  if (typeof value !== 'string' || !path.isAbsolute(value) || /[\x00-\x1f]/.test(value)) {
    throw new Error(`${label}: нужен полный путь к папке.`);
  }
  return path.resolve(value);
}

class LauncherStore {
  constructor(projectRoot, { appData = process.env.APPDATA, now = () => new Date().toISOString(), dataRoot } = {}) {
    this.projectRoot = path.resolve(projectRoot);
    this.dataRoot = dataRoot === undefined ? path.join(this.projectRoot, 'data') : absoluteFolder(dataRoot, 'Папка данных');
    this.file = path.join(this.dataRoot, 'launcher.json');
    this.now = now;
    this.deletingInstances = new Set();
    this.legacyRuntime = path.join(appData || '', 'horizon-launcher', 'versions', 'v0.2.93', 'horizon-client.exe');
    this.defaultRuntime = path.join(this.projectRoot, 'runtime', 'atlas-client.exe');
    this.atlasRuntime = path.join(this.projectRoot, 'runtime', 'atlas-client.exe');
    this.sourceRoot = path.join(this.projectRoot, 'engine', 'atlas');
    fs.mkdirSync(this.dataRoot, { recursive: true });
    if (fs.existsSync(this.file)) {
      try {
        this.config = JSON.parse(fs.readFileSync(this.file, 'utf8'));
        if (this.config.schema !== 1 || !Array.isArray(this.config.instances) || !this.config.instances.length || !this.config.settings) {
          throw new Error('неизвестный формат');
        }
        for (const instance of this.config.instances) {
          if (!/^[a-z0-9-]{1,64}$/.test(instance.id)) throw new Error('неверный идентификатор сборки');
          cleanName(instance.name);
        }
      } catch (error) {
        throw new Error(`Не удалось прочитать data/launcher.json (${error.message}). Файл сохранён без изменений.`);
      }
      let migrated = false;
      if (this.config.settings.sourceRoot === path.join(this.projectRoot, 'engine', 'horizon')) {
        this.config.settings.sourceRoot = this.sourceRoot;
        migrated = true;
      }
      for (const instance of this.config.instances) {
        if (instance.description === 'Твоя первая сборка Horizon') {
          instance.description = 'Твоя первая сборка Atlas';
          migrated = true;
        }
      }
      if (migrated) this.save();
    } else {
      this.config = {
        schema: 1,
        selectedId: 'atlas',
        instances: [{ id: 'atlas', name: 'Atlas', description: 'Твоя первая сборка Atlas', accent: 'mint', createdAt: this.now(), lastPlayed: null }],
        settings: { runtimePath: this.defaultRuntime, sourceRoot: this.sourceRoot, closeOnLaunch: false },
      };
      this.save();
    }
    this.migration = migrateLegacyData({dataRoot:this.dataRoot,...(dataRoot === undefined ? {projectRoot:this.projectRoot} : {})});
    for (const instance of this.config.instances) this.ensureFolders(instance.id);
    this.preferAtlasRuntime();
  }

  preferAtlasRuntime() {
    // Keep explicit custom builds selected. Only replace the original bundled client.
    if (![this.defaultRuntime,this.legacyRuntime].some(candidate => path.resolve(this.config.settings.runtimePath).toLowerCase() === path.resolve(candidate).toLowerCase())) return;
    if (!fs.existsSync(this.atlasRuntime) || !fs.statSync(this.atlasRuntime).isFile()) return;
    this.config.settings.runtimePath = this.atlasRuntime;
    this.save();
  }

  save() {
    const temporary = `${this.file}.tmp`;
    fs.writeFileSync(temporary, `${JSON.stringify(this.config, null, 2)}\n`, 'utf8');
    fs.renameSync(temporary, this.file);
  }

  instancePath(id) {
    this.requireInstance(id);
    return path.join(this.dataRoot, 'instances', id);
  }

  requireInstance(id) {
    const instance = this.config.instances.find((item) => item.id === id);
    if (!instance) throw new Error('Эта сборка не найдена.');
    return instance;
  }

  assertInstanceAvailable(id) {
    this.requireInstance(id);
    if (this.deletingInstances.has(id)) throw new Error('Сборка удаляется. Дождись завершения.');
  }

  managedDirectory(...parts) {
    const root = path.resolve(this.dataRoot);
    const target = path.resolve(root, ...parts);
    const relative = path.relative(root, target);
    if (!relative || relative.startsWith(`..${path.sep}`) || relative === '..' || path.isAbsolute(relative)) {
      throw new Error('Небезопасный путь к папке сборки.');
    }
    let current = root;
    for (const part of ['', ...relative.split(path.sep)]) {
      if (part) current = path.join(current, part);
      if (!fs.existsSync(current)) continue;
      if (fs.lstatSync(current).isSymbolicLink()) throw new Error('Удаление папок-ссылок не поддерживается.');
    }
    if (fs.existsSync(target)) {
      const physicalRoot = fs.realpathSync(root);
      const physicalTarget = fs.realpathSync(target);
      const physicalRelative = path.relative(physicalRoot, physicalTarget);
      if (!physicalRelative || physicalRelative === '..' || physicalRelative.startsWith(`..${path.sep}`) || path.isAbsolute(physicalRelative)) {
        throw new Error('Папка сборки находится вне папки данных Atlas.');
      }
    }
    return target;
  }

  deletionFolders(id) {
    this.requireInstance(id);
    if (!/^[a-z0-9-]{1,64}$/.test(id)) throw new Error('Неверный идентификатор сборки.');
    const base = this.managedDirectory('instances', id);
    const folders = fs.existsSync(base) ? [base] : [];
    const sessionsRoot = this.managedDirectory('sessions');
    if (!fs.existsSync(sessionsRoot)) return folders;
    for (const entry of fs.readdirSync(sessionsRoot, { withFileTypes: true })) {
      if (!entry.isDirectory() || !/^[a-f0-9-]{36}$/.test(entry.name)) continue;
      const session = this.managedDirectory('sessions', entry.name);
      const metadata = path.join(session, 'session.json');
      if (!fs.existsSync(metadata) || fs.lstatSync(metadata).isSymbolicLink() || fs.statSync(metadata).size > 65536) continue;
      let data;
      try { data = JSON.parse(fs.readFileSync(metadata, 'utf8')); } catch { continue; }
      if (data.instanceId !== id) continue;
      const home = this.managedDirectory('sessions', entry.name, 'home');
      if (typeof data.home !== 'string' || path.resolve(data.home).toLowerCase() !== home.toLowerCase()) {
        throw new Error('Путь дополнительного окна не соответствует папке сборки.');
      }
      folders.push(session);
    }
    return folders;
  }

  async deleteInstance(id, { trashItem, isRunning = () => false } = {}) {
    const instance = this.requireInstance(id);
    if (this.config.instances.length <= 1) throw new Error('Последнюю сборку удалить нельзя. Сначала создай другую.');
    if (this.deletingInstances.size) throw new Error('Дождись удаления текущей сборки.');
    if (typeof trashItem !== 'function') throw new Error('Корзина недоступна. Сборка сохранена.');
    if (isRunning(id)) throw new Error('Закрой все окна этой сборки перед удалением.');
    this.deletingInstances.add(id);
    let staging;
    let trashed = false;
    const configurationBefore = { instances: this.config.instances, selectedId: this.config.selectedId };
    const moved = [];
    try {
      const folders = this.deletionFolders(id);
      if (isRunning(id)) throw new Error('Закрой все окна этой сборки перед удалением.');
      if (folders.length) {
        staging = this.managedDirectory('.delete-staging', crypto.randomUUID());
        fs.mkdirSync(staging, { recursive: true });
        for (let index = 0; index < folders.length; index++) {
          const destination = path.join(staging, index === 0 ? 'instance' : `window-${path.basename(folders[index])}`);
          fs.renameSync(folders[index], destination);
          moved.push({ source: folders[index], destination });
        }
        fs.writeFileSync(path.join(staging, 'atlas-deleted-instance.json'), JSON.stringify({ instance, folders: moved.map(item => ({ original: path.relative(this.dataRoot, item.source), folder: path.basename(item.destination) })) }, null, 2));
        await trashItem(staging);
        trashed = true;
      }
      this.config.instances = this.config.instances.filter(item => item.id !== id);
      if (this.config.selectedId === id) this.config.selectedId = this.config.instances[0].id;
      this.save();
    } catch (error) {
      this.config.instances = configurationBefore.instances;
      this.config.selectedId = configurationBefore.selectedId;
      if (trashed) throw new Error(`Файлы сборки уже в корзине, но список сборок не сохранён: ${error.message}`);
      let rollbackFailed = false;
      for (const item of moved.reverse()) {
        if (fs.existsSync(item.destination)) {
          try { fs.renameSync(item.destination, item.source); } catch { rollbackFailed = true; }
        }
      }
      if (staging && fs.existsSync(staging) && !rollbackFailed) {
        const manifest = path.join(staging, 'atlas-deleted-instance.json');
        if (fs.existsSync(manifest)) fs.unlinkSync(manifest);
        try { fs.rmdirSync(staging); } catch {}
      }
      throw new Error(rollbackFailed ? 'Не удалось удалить сборку. Часть файлов осталась в data/.delete-staging; не удаляй эту папку.' : `Не удалось удалить сборку: ${error.message}`);
    } finally {
      this.deletingInstances.delete(id);
    }
  }

  ensureFolders(id) {
    const root = this.instancePath(id);
    for (const directory of ['', 'saves', 'resourcepacks', 'logs', 'screenshots', 'atlas-mods']) {
      fs.mkdirSync(path.join(root, directory), { recursive: true });
    }
    return root;
  }

  createInstance(input) {
    if (!input || typeof input !== 'object') throw new Error('Не указаны параметры сборки.');
    const name = cleanName(input.name);
    if (this.config.instances.some((instance) => instance.name.toLocaleLowerCase() === name.toLocaleLowerCase())) {
      throw new Error('Сборка с таким названием уже есть.');
    }
    const description = typeof input.description === 'string' ? input.description.trim() : '';
    if (description.length > 160 || /[\x00-\x1f]/.test(description)) throw new Error('Описание должно быть короче 160 символов.');
    const instance = {
      id: `instance-${crypto.randomUUID()}`,
      name,
      description,
      accent: ACCENTS.has(input.accent) ? input.accent : 'mint',
      icon: INSTANCE_OPTIONS.icons.includes(input.icon) ? input.icon : 'box',
      createdAt: this.now(),
      lastPlayed: null,
    };
    this.config.instances.push(instance);
    this.ensureFolders(instance.id);
    this.config.selectedId = instance.id;
    this.save();
    return instance;
  }

  updateInstance(id, patch) {
    if (!patch || typeof patch !== 'object' || Array.isArray(patch)) throw new Error('Некорректные параметры сборки.');
    const current=this.requireInstance(id);
    const next={...current};
    if ('name' in patch) {
      next.name=cleanName(patch.name);
      if(this.config.instances.some(i=>i.id!==id&&i.name.toLocaleLowerCase()===next.name.toLocaleLowerCase()))throw new Error('Сборка с таким названием уже есть.');
    }
    if('description' in patch){
      if(typeof patch.description!=='string'||patch.description.length>160||/[\x00-\x1f]/.test(patch.description))throw new Error('Описание должно быть короче 160 символов.');
      next.description=patch.description.trim();
    }
    if('accent' in patch){if(!ACCENTS.has(patch.accent))throw new Error('Некорректный цвет сборки.');next.accent=patch.accent;}
    if('icon' in patch){if(!INSTANCE_OPTIONS.icons.includes(patch.icon))throw new Error('Некорректный значок сборки.');next.icon=patch.icon;}
    Object.assign(current,next);this.save();return current;
  }

  selectInstance(id) {
    this.requireInstance(id);
    this.config.selectedId = id;
    this.save();
  }

  saveSettings(patch) {
    if (!patch || typeof patch !== 'object' || Array.isArray(patch)) throw new Error('Некорректные настройки.');
    const next = { ...this.config.settings };
    for (const [key, values] of Object.entries(CHOICES)) {
      if (key in patch) {
        if (!values.includes(patch[key])) throw new Error('Некорректная настройка оформления.');
        next[key] = patch[key];
      }
    }
    for (const key of ['showHeroText', 'reducedMotion']) {
      if (key in patch) {
        if (typeof patch[key] !== 'boolean') throw new Error('Некорректная настройка оформления.');
        next[key] = patch[key];
      }
    }
    if ('runtimePath' in patch) {
      next.runtimePath = absoluteFolder(patch.runtimePath, 'Клиент');
      if (path.extname(next.runtimePath).toLowerCase() !== '.exe') throw new Error('Выбери исполняемый файл клиента .exe.');
    }
    if ('sourceRoot' in patch) next.sourceRoot = absoluteFolder(patch.sourceRoot, 'Исходники');
    if ('closeOnLaunch' in patch) {
      if (typeof patch.closeOnLaunch !== 'boolean') throw new Error('Некорректная настройка сворачивания окна.');
      next.closeOnLaunch = patch.closeOnLaunch;
    }
    if ('discordRichPresence' in patch) {
      if (typeof patch.discordRichPresence !== 'boolean') throw new Error('Некорректная настройка Discord Presence.');
      next.discordRichPresence = patch.discordRichPresence;
    }
    if ('discordApplicationId' in patch) {
      if (typeof patch.discordApplicationId !== 'string' || (patch.discordApplicationId !== '' && !/^[0-9]{16,22}$/.test(patch.discordApplicationId))) throw new Error('Discord Application ID должен содержать от 16 до 22 цифр.');
      next.discordApplicationId = patch.discordApplicationId;
    }
    if ('discordAssetKey' in patch) {
      if (typeof patch.discordAssetKey !== 'string' || !/^[a-z0-9_-]{1,64}$/.test(patch.discordAssetKey)) throw new Error('Некорректное имя изображения Discord.');
      next.discordAssetKey = patch.discordAssetKey;
    }
    this.config.settings = next;
    this.save();
  }

  setInstanceMemory(id, memoryMb) {
    if (!Number.isSafeInteger(memoryMb) || (memoryMb !== 0 && (memoryMb < 2048 || memoryMb > memoryMaximum() || memoryMb % 256 !== 0))) {
      throw new Error('Выбери допустимый объём памяти.');
    }
    this.requireInstance(id).memoryMb = memoryMb;
    this.save();
  }

  markLaunched(id) {
    this.requireInstance(id).lastPlayed = this.now();
    this.save();
  }

  gameOptions(id = this.config.selectedId) {
    const file = path.join(this.instancePath(id), 'atlas-extras.txt');
    const values = { ...GAME_OPTIONS };
    if (fs.existsSync(file)) {
      for (const line of fs.readFileSync(file, 'utf8').split(/\r?\n/)) {
        const separator = line.indexOf(':');
        const key = line.slice(0, separator);
        const value = line.slice(separator + 1).trim();
        if (separator !== -1 && Object.hasOwn(values, key) && (value === 'true' || value === 'false')) values[key] = value === 'true';
      }
    }
    return values;
  }

  setGameOptions(id, patch) {
    if (!patch || typeof patch !== 'object' || Array.isArray(patch)) throw new Error('Некорректные настройки игры.');
    for (const [key, value] of Object.entries(patch)) {
      if (!Object.hasOwn(GAME_OPTIONS, key) || typeof value !== 'boolean') throw new Error('Неизвестная настройка игры или неверное значение.');
    }
    const file = path.join(this.instancePath(id), 'atlas-extras.txt');
    let lines = fs.existsSync(file) ? fs.readFileSync(file, 'utf8').split(/\r?\n/) : [];
    lines = lines.filter((line) => {
      const separator = line.indexOf(':');
      return separator < 0 || !Object.hasOwn(patch, line.slice(0, separator));
    });
    while (lines.at(-1) === '') lines.pop();
    for (const [key, value] of Object.entries(patch)) lines.push(`${key}:${value}`);
    fs.writeFileSync(`${file}.atlas-tmp`, `${lines.join('\n')}\n`, 'utf8');
    fs.renameSync(`${file}.atlas-tmp`, file);
  }

  getState() {
    this.preferAtlasRuntime();
    const settings = { ...APPEARANCE, discordApplicationId: '', discordAssetKey: 'atlas', ...this.config.settings };
    return {
      projectRoot: this.projectRoot,
      dataRoot: this.dataRoot,
      sourceRoot: settings.sourceRoot,
      runtimePath: settings.runtimePath,
      version: '0.1',
      memory: { totalMb: Math.floor(os.totalmem() / 1024 ** 2), maximumMb: memoryMaximum() },
      engineVersion: 'v0.2.93',
      sourceAvailable: fs.existsSync(path.join(settings.sourceRoot, 'Cargo.toml')),
      runtimeAvailable: fs.existsSync(settings.runtimePath),
      instances: this.config.instances.map((instance) => ({ ...instance, path: this.instancePath(instance.id) })),
      selectedId: this.config.selectedId,
      settings,
      selectedGameOptions: this.gameOptions(),
    };
  }

  folder(kind, id = this.config.selectedId) {
    const root = this.instancePath(id);
    switch (kind) {
      case 'project': return this.projectRoot;
      case 'instance': return root;
      case 'saves': return path.join(root, 'saves');
      case 'resourcepacks': return path.join(root, 'resourcepacks');
      case 'logs': return path.join(root, 'logs');
      case 'source': return this.config.settings.sourceRoot;
      case 'guide': return path.join(this.projectRoot, 'docs', 'EDITING-ATLAS.md');
      default: throw new Error('Неизвестная папка.');
    }
  }
}

function buildLaunchSpec(store, id) {
  store.preferAtlasRuntime();
  const instance = store.requireInstance(id);
  const runtime = store.config.settings.runtimePath;
  if (!fs.existsSync(runtime) || !fs.statSync(runtime).isFile()) throw new Error('Клиент не найден. Выбери atlas-client.exe в настройках.');
  if (path.extname(runtime).toLowerCase() !== '.exe') throw new Error('Клиент должен быть файлом .exe.');
  // A locally compiled client can reuse the matching prepared resources installed by Horizon.
  const resourcesCandidates = [
    path.join(path.dirname(runtime), '.local', 'atlas-client', '26.3'),
    path.join(store.projectRoot, 'runtime', '.local', 'atlas-client', '26.3'),
    path.join(path.dirname(runtime), '.local', 'horizon-client', '26.3'),
    path.join(store.config.settings.sourceRoot, '.local', 'atlas-client', '26.3'),
    path.join(path.dirname(store.legacyRuntime), '.local', 'horizon-client', '26.3'),
  ];
  const resources = resourcesCandidates.find((directory) => fs.existsSync(path.join(directory, 'assets')) && fs.existsSync(path.join(directory, 'data')));
  if (!resources) throw new Error('Подготовленные ресурсы Minecraft 26.3 не найдены. Проверь папку ресурсов установленной игры.');
  const home = store.ensureFolders(instance.id);
  return { executable: runtime, args: ['--resources', resources], cwd: home, env: { ...process.env, ATLAS_HOME: home, HORIZON_HOME: home, ATLAS_MEMORY_MB: String(instance.memoryMb || 0) } };
}

module.exports = { LauncherStore, buildLaunchSpec, cleanName, GAME_OPTIONS };
