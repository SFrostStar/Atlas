'use strict';

const messages = {
  ru: {
    skip: 'К содержимому', navigation: 'Навигация', language: 'Язык', navAbout: 'О проекте', navDownload: 'Установка',
    heroTitle: 'Знакомый Minecraft. Другая основа.', heroDescription: 'Независимый клиент на Rust и свой лаунчер. На основе Horizon, с характером Atlas.',
    download: 'Скачать лаунчер', viewSource: 'Смотреть код', joinDiscord: 'В Discord', explore: 'Чуть ближе к Atlas',
    benchmarkTitle: 'Замер рендера', benchmarkScenes: 'Сцена теста',
    benchmark_village: 'Деревня', benchmark_forest: 'Лес', benchmark_ocean: 'Океан', benchmark_mountain: 'Горы',
    benchmarkAverage: 'Средний FPS', benchmarkLowest: 'Самый низкий фпс', benchmarkSettings: '1920×1080 · Vulkan · 16 чанков · без модов',
    benchmarkMethod: 'Условия замера', benchmarkDetails: 'Рендер в текстуру. Один 12-секундный пролёт на сцену, 2 с разогрева. Дальность симуляции 12 чанков, VSync выключен. Для деревни 1474 FPS — ориентир.',
    benchmarkLow: 'Нижний показатель FPS вычисляется по среднему времени 1% самых медленных кадров.',
    aboutTitle: ['Твой мир.', 'Твоя сборка.'], aboutDescription: 'Minecraft на нативной основе Rust. Atlas объединяет игру и лаунчер: меньше возни с файлами, больше места для своих идей.',
    landscapeAlt: 'Горная долина Minecraft на закате', worldCaption: 'Начинается с одного мира.',
    instancesTitle: 'Всё по сборкам', instancesText: 'Отдельные миры, ресурспаки и настройки. Переключайся между сборками в лаунчере.',
    modsTitle: 'Место для модов', modsText: 'Выбирай совместимые нативные моды в каталоге Atlas и устанавливай в свою сборку.',
    updatesTitle: 'Проект развивается', updatesText: 'Открытый код и обновления с GitHub. Можно следить за изменениями — или заглянуть внутрь.',
    downloadTitle: 'Увидимся в игре.', downloadDescription: 'Portable-версия для Windows. Скачай, распакуй и начни со своей первой сборки.', allReleases: 'Релизы и подробности',
    stepOneTitle: 'Распакуй архив', stepOneText: 'Полностью извлеки ZIP в отдельную папку.',
    stepTwoTitle: 'Открой Atlas Launcher.exe', stepTwoText: 'Создай профиль и свою первую сборку.',
    stepThreeTitle: 'Нажми «Играть»', stepThreeText: 'Если нужны игровые файлы, лаунчер предложит их скачать.',
    bootstrap: {
      downloadDescription: 'Загрузчик для Windows установит Atlas и будет проверять обновления при запуске.',
      stepOneTitle: 'Скачай Atlas-Launcher.exe', stepOneText: 'После установки открывай Atlas с рабочего стола.',
      stepTwoTitle: 'Запусти загрузчик', stepTwoText: 'Когда файлы будут готовы, лаунчер откроется автоматически.'
    },
    madeBy: 'Разрабатывает', basedOn: 'На основе', legal: '. Неофициальный проект, не связанный с Mojang или Microsoft.', footerNote: 'Сделано для своих миров.',
    pageTitle: 'Atlas — Minecraft на Rust', metaDescription: 'Atlas — независимый клиент Minecraft на Rust. Свой лаунчер, сборки и нативные моды. Скачать для Windows и посмотреть исходный код.'
  },
  en: {
    skip: 'Skip to content', navigation: 'Navigation', language: 'Language', navAbout: 'About', navDownload: 'Get started',
    heroTitle: 'Familiar Minecraft. A different foundation.', heroDescription: 'An independent Rust client with its own launcher. Based on Horizon, shaped into Atlas.',
    download: 'Download launcher', viewSource: 'View source', joinDiscord: 'Join Discord', explore: 'A little more Atlas',
    benchmarkTitle: 'Rendering benchmark', benchmarkScenes: 'Test scene',
    benchmark_village: 'Village', benchmark_forest: 'Forest', benchmark_ocean: 'Ocean', benchmark_mountain: 'Mountains',
    benchmarkAverage: 'Average FPS', benchmarkLowest: 'Lowest FPS', benchmarkSettings: '1920×1080 · Vulkan · 16 chunks · no mods',
    benchmarkMethod: 'Test conditions', benchmarkDetails: 'Rendered to a texture. One 12-second flyover per scene after a 2-second warm-up. Simulation distance 12 chunks, VSync off. Village uses a reference value of 1474 FPS.',
    benchmarkLow: 'The lower FPS figure is calculated from the average frame time of the slowest 1% of frames.',
    aboutTitle: ['Your world.', 'Your instance.'], aboutDescription: 'Minecraft on a native Rust foundation. Atlas brings the game and launcher together: less time managing files, more room for your ideas.',
    landscapeAlt: 'A mountainous Minecraft valley at sunset', worldCaption: 'It starts with a single world.',
    instancesTitle: 'Keep worlds separate', instancesText: 'Separate worlds, resource packs and settings. Switch between your instances in the launcher.',
    modsTitle: 'Room for mods', modsText: 'Browse compatible native mods in the Atlas catalogue and install them into your instance.',
    updatesTitle: 'Always in progress', updatesText: 'Open source, with updates from GitHub. Follow the changes — or take a look inside.',
    downloadTitle: 'See you in-game.', downloadDescription: 'A portable build for Windows. Download, extract and start with your first instance.', allReleases: 'Releases and details',
    stepOneTitle: 'Extract the archive', stepOneText: 'Extract the entire ZIP into a separate folder.',
    stepTwoTitle: 'Open Atlas Launcher.exe', stepTwoText: 'Create a profile and your first instance.',
    stepThreeTitle: 'Press Play', stepThreeText: 'If game files are needed, the launcher will offer to download them.',
    bootstrap: {
      downloadDescription: 'The Windows loader installs Atlas and checks for updates at launch.',
      stepOneTitle: 'Download Atlas-Launcher.exe', stepOneText: 'After setup, open Atlas from the desktop.',
      stepTwoTitle: 'Run the loader', stepTwoText: 'The launcher opens automatically when the files are ready.'
    },
    madeBy: 'Developed by', basedOn: 'Based on', legal: '. An unofficial project, not affiliated with Mojang or Microsoft.', footerNote: 'Made for your worlds.',
    pageTitle: 'Atlas — Minecraft on Rust', metaDescription: 'Atlas is an independent Rust Minecraft client. Its own launcher, instances and native mods. Download for Windows and explore the source.'
  }
};

let bootstrapDownload = false;

function setLanguage(language) {
  const lang = Object.hasOwn(messages, language) ? language : 'ru';
  const copy = messages[lang];
  document.documentElement.lang = lang;
  document.title = copy.pageTitle;
  document.querySelector('meta[name="description"]').content = copy.metaDescription;
  for (const element of document.querySelectorAll('[data-i18n]')) {
    const key = element.dataset.i18n;
    const value = (bootstrapDownload ? copy.bootstrap[key] : undefined) ?? copy[key];
    if (Array.isArray(value)) {
      element.replaceChildren(document.createTextNode(value[0]), document.createElement('br'), document.createTextNode(value[1]));
    } else if (typeof value === 'string') element.textContent = value;
  }
  for (const element of document.querySelectorAll('[data-i18n-aria]')) element.setAttribute('aria-label', copy[element.dataset.i18nAria]);
  for (const element of document.querySelectorAll('[data-i18n-alt]')) element.alt = copy[element.dataset.i18nAlt];
  for (const button of document.querySelectorAll('[data-language]')) button.setAttribute('aria-pressed', String(button.dataset.language === lang));
  try { localStorage.setItem('atlas-site-language', lang); } catch { /* Language selection works without storage. */ }
}

let initialLanguage = navigator.language.toLowerCase().startsWith('ru') || navigator.language.toLowerCase().startsWith('uk') ? 'ru' : 'en';
try { initialLanguage = localStorage.getItem('atlas-site-language') || initialLanguage; } catch { /* Storage can be unavailable in private browsers. */ }
setLanguage(initialLanguage);
for (const button of document.querySelectorAll('[data-language]')) button.addEventListener('click', () => setLanguage(button.dataset.language));

const benchmarkScenes = document.querySelector('.benchmark-scenes');
const benchmarkMotion = matchMedia('(prefers-reduced-motion: reduce)');
const benchmarkDuration = 3500;
const benchmarkResults = new Map();
let benchmarkFrame;

for (const scene of document.querySelectorAll('.benchmark-results')) {
  const rows = Array.from(scene.querySelectorAll('.benchmark-row'), row => {
    const meter = row.querySelector('meter');
    const number = row.querySelector('strong');
    const accessibleNumber = document.createElement('span');
    accessibleNumber.className = 'benchmark-static-value';
    accessibleNumber.textContent = number.textContent;
    number.setAttribute('aria-hidden', 'true');
    number.before(accessibleNumber);
    return { meter, number, target: meter.value };
  });
  benchmarkResults.set(scene.dataset.scene, rows);
}

function animateBenchmark() {
  cancelAnimationFrame(benchmarkFrame);
  const scene = benchmarkScenes.querySelector(':checked').value;
  const rows = benchmarkResults.get(scene);
  const start = performance.now();

  function render(progress) {
    const eased = progress * progress * (3 - 2 * progress);
    for (const { meter, number, target } of rows) {
      meter.value = target * eased;
      number.textContent = Math.floor(meter.value);
    }
  }

  if (benchmarkMotion.matches) {
    render(1);
    return;
  }

  render(0);
  function tick(now) {
    const progress = Math.max(0, Math.min(1, (now - start) / benchmarkDuration));
    render(progress);
    if (progress < 1) benchmarkFrame = requestAnimationFrame(tick);
  }
  benchmarkFrame = requestAnimationFrame(tick);
}

benchmarkScenes.addEventListener('change', animateBenchmark);
benchmarkMotion.addEventListener('change', animateBenchmark);
animateBenchmark();

async function refreshRelease() {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 5000);
  try {
    const response = await fetch('https://api.github.com/repos/SFrostStar/Atlas/releases/latest', { signal: controller.signal, headers: { Accept: 'application/vnd.github+json' } });
    if (!response.ok) return;
    const release = await response.json();
    if (release.draft || release.prerelease || !Array.isArray(release.assets) || !/^atlas-\d+\.\d+\.\d+$/.test(release.tag_name)) return;
    const version = release.tag_name.slice('atlas-'.length);
    const asset = release.assets.find(file => file.name === 'Atlas-Launcher.exe'
        && Number.isSafeInteger(file.size) && file.size > 0 && file.size <= 50_000_000)
      ?? release.assets.find(asset => asset.name === `Atlas-${version}-Windows-x64.zip`)
      ?? release.assets.find(asset => asset.name === `Atlas-${version}-Update.zip`);
    if (!asset) return;
    const url = new URL(asset.browser_download_url);
    if (url.origin !== 'https://github.com' || url.pathname !== `/SFrostStar/Atlas/releases/download/${release.tag_name}/${asset.name}`
        || url.username || url.password || url.search || url.hash) return;
    for (const link of document.querySelectorAll('[data-download]')) link.href = url.href;
    document.querySelector('[data-version]').textContent = release.tag_name.replace('atlas-', 'v');
    if (Number.isFinite(asset.size) && asset.size > 0) {
      document.querySelector('[data-size]').textContent = asset.size < 1048576
        ? `${Math.ceil(asset.size / 1024)} KB` : `${Math.round(asset.size / 1048576)} MB`;
    }
    bootstrapDownload = asset.name === 'Atlas-Launcher.exe';
    setLanguage(document.documentElement.lang);
  } catch { /* The verified published download remains available if GitHub is unreachable. */ }
  finally { clearTimeout(timeout); }
}

refreshRelease();
