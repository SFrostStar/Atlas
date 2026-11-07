'use strict';

const messages = {
  ru: {
    skip: 'К содержимому', navigation: 'Навигация', language: 'Язык', navAbout: 'О проекте', navDownload: 'Установка',
    heroTitle: 'Знакомый Minecraft. Другая основа.', heroDescription: 'Независимый клиент на Rust и свой лаунчер. На основе Horizon, с характером Atlas.',
    download: 'Скачать лаунчер', viewSource: 'Смотреть код', explore: 'Чуть ближе к Atlas',
    aboutTitle: ['Твой мир.', 'Твоя сборка.'], aboutDescription: 'Minecraft на нативной основе Rust. Atlas объединяет игру и лаунчер: меньше возни с файлами, больше места для своих идей.',
    landscapeAlt: 'Горная долина Minecraft на закате', worldCaption: 'Начинается с одного мира.',
    instancesTitle: 'Всё по сборкам', instancesText: 'Отдельные миры, ресурспаки и настройки. Переключайся между сборками в лаунчере.',
    modsTitle: 'Место для модов', modsText: 'Выбирай совместимые нативные моды в каталоге Atlas и устанавливай в свою сборку.',
    updatesTitle: 'Проект развивается', updatesText: 'Открытый код и обновления с GitHub. Можно следить за изменениями — или заглянуть внутрь.',
    downloadTitle: 'Увидимся в игре.', downloadDescription: 'Portable-версия для Windows. Скачай, распакуй и начни со своей первой сборки.', allReleases: 'Релизы и подробности',
    stepOneTitle: 'Распакуй архив', stepOneText: 'Полностью извлеки ZIP в отдельную папку.',
    stepTwoTitle: 'Открой Atlas Launcher.exe', stepTwoText: 'Создай профиль и свою первую сборку.',
    stepThreeTitle: 'Нажми «Играть»', stepThreeText: 'Если нужны игровые файлы, лаунчер предложит их скачать.',
    madeBy: 'Разрабатывает', basedOn: 'На основе', legal: '. Неофициальный проект, не связанный с Mojang или Microsoft.', footerNote: 'Сделано для своих миров.',
    pageTitle: 'Atlas — Minecraft на Rust', metaDescription: 'Atlas — независимый клиент Minecraft на Rust. Свой лаунчер, сборки и нативные моды. Скачать для Windows и посмотреть исходный код.'
  },
  en: {
    skip: 'Skip to content', navigation: 'Navigation', language: 'Language', navAbout: 'About', navDownload: 'Get started',
    heroTitle: 'Familiar Minecraft. A different foundation.', heroDescription: 'An independent Rust client with its own launcher. Based on Horizon, shaped into Atlas.',
    download: 'Download launcher', viewSource: 'View source', explore: 'A little more Atlas',
    aboutTitle: ['Your world.', 'Your instance.'], aboutDescription: 'Minecraft on a native Rust foundation. Atlas brings the game and launcher together: less time managing files, more room for your ideas.',
    landscapeAlt: 'A mountainous Minecraft valley at sunset', worldCaption: 'It starts with a single world.',
    instancesTitle: 'Keep worlds separate', instancesText: 'Separate worlds, resource packs and settings. Switch between your instances in the launcher.',
    modsTitle: 'Room for mods', modsText: 'Browse compatible native mods in the Atlas catalogue and install them into your instance.',
    updatesTitle: 'Always in progress', updatesText: 'Open source, with updates from GitHub. Follow the changes — or take a look inside.',
    downloadTitle: 'See you in-game.', downloadDescription: 'A portable build for Windows. Download, extract and start with your first instance.', allReleases: 'Releases and details',
    stepOneTitle: 'Extract the archive', stepOneText: 'Extract the entire ZIP into a separate folder.',
    stepTwoTitle: 'Open Atlas Launcher.exe', stepTwoText: 'Create a profile and your first instance.',
    stepThreeTitle: 'Press Play', stepThreeText: 'If game files are needed, the launcher will offer to download them.',
    madeBy: 'Developed by', basedOn: 'Based on', legal: '. An unofficial project, not affiliated with Mojang or Microsoft.', footerNote: 'Made for your worlds.',
    pageTitle: 'Atlas — Minecraft on Rust', metaDescription: 'Atlas is an independent Rust Minecraft client. Its own launcher, instances and native mods. Download for Windows and explore the source.'
  }
};

function setLanguage(language) {
  const lang = Object.hasOwn(messages, language) ? language : 'ru';
  const copy = messages[lang];
  document.documentElement.lang = lang;
  document.title = copy.pageTitle;
  document.querySelector('meta[name="description"]').content = copy.metaDescription;
  for (const element of document.querySelectorAll('[data-i18n]')) {
    const value = copy[element.dataset.i18n];
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

async function refreshRelease() {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 5000);
  try {
    const response = await fetch('https://api.github.com/repos/SFrostStar/Atlas/releases/latest', { signal: controller.signal, headers: { Accept: 'application/vnd.github+json' } });
    if (!response.ok) return;
    const release = await response.json();
    if (release.draft || release.prerelease || !Array.isArray(release.assets)) return;
    const portable = release.assets.find(asset => /^Atlas-\d+\.\d+\.\d+-Windows-x64\.zip$/i.test(asset.name));
    if (!portable || !/^atlas-\d+\.\d+\.\d+$/.test(release.tag_name)) return;
    const url = new URL(portable.browser_download_url);
    if (url.origin !== 'https://github.com' || !url.pathname.startsWith('/SFrostStar/Atlas/releases/download/') || url.search || url.hash) return;
    for (const link of document.querySelectorAll('[data-download]')) link.href = url.href;
    document.querySelector('[data-version]').textContent = release.tag_name.replace('atlas-', 'v');
    if (Number.isFinite(portable.size) && portable.size > 0) document.querySelector('[data-size]').textContent = `${Math.round(portable.size / 1048576)} MB`;
  } catch { /* The verified published download remains available if GitHub is unreachable. */ }
  finally { clearTimeout(timeout); }
}

refreshRelease();
