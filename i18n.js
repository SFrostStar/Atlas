'use strict';
(function (root) {
  const accountErrors = {
    not_configured: ['Microsoft sign-in needs the Atlas application Client ID. See docs/ACCOUNTS.md.', 'Для входа Microsoft нужен Client ID приложения Atlas. Инструкция — в docs/ACCOUNTS.md.'],
    storage_unavailable: ['Windows secure storage is unavailable. Microsoft tokens have not been saved.', 'Защищённое хранилище Windows недоступно. Данные входа Microsoft не сохранены.'],
    storage_locked: ['This account cannot be decrypted by the current Windows user. Sign in again.', 'Текущий пользователь Windows не может прочитать данные аккаунта. Войди заново.'],
    network: ['The account service did not respond. Check your connection and try again.', 'Сервис аккаунтов не ответил. Проверь интернет и повтори попытку.'],
    invalid_response: ['The account service returned an invalid response.', 'Сервис аккаунтов вернул некорректный ответ.'],
    invalid_client: ['Microsoft refused the Atlas app registration. Check its Client ID and public client settings.', 'Microsoft отклонил регистрацию Atlas. Проверь Client ID и настройку public client flows.'],
    authorization_declined: ['Microsoft sign-in was declined.', 'Вход Microsoft отклонён.'],
    expired_token: ['The sign-in code expired. Start sign-in again.', 'Код входа истёк. Начни вход заново.'],
    bad_verification_code: ['Microsoft refused this sign-in code. Start sign-in again.', 'Microsoft отклонил код входа. Начни вход заново.'],
    invalid_grant: ['Your Microsoft sign-in expired. Sign in again.', 'Сессия Microsoft истекла. Войди заново.'],
    xbox_profile: ['Create an Xbox profile at xbox.com, then sign in again.', 'Создай профиль Xbox на xbox.com, затем войди заново.'],
    xbox_region: ['Xbox Live is unavailable in this region.', 'Xbox Live недоступен в этом регионе.'],
    xbox_family: ['This Xbox account needs family or adult verification at xbox.com.', 'Для аккаунта Xbox нужно подтверждение взрослого или настройка семейной группы на xbox.com.'],
    xbox: ['Xbox Live refused the sign-in.', 'Xbox Live отклонил вход.'],
    app_not_approved: ['Minecraft refused the Atlas app registration. Its Minecraft API access may need approval.', 'Minecraft отклонил регистрацию Atlas. Возможно, приложению ещё нужно одобрение доступа к Minecraft API.'],
    rate_limited: ['Too many sign-in requests. Wait a minute and try again.', 'Слишком много попыток входа. Подожди минуту и попробуй снова.'],
    minecraft: ['Minecraft services refused the sign-in.', 'Сервисы Minecraft отклонили вход.'],
    no_ownership: ['This Microsoft account does not own Minecraft: Java Edition.', 'На этом Microsoft-аккаунте нет Minecraft: Java Edition.'],
    no_profile: ['Create your Minecraft: Java Edition profile at minecraft.net, then sign in again.', 'Создай профиль Minecraft: Java Edition на minecraft.net, затем войди заново.'],
    invalid_name: ['A local name must contain 3–16 letters, digits or underscores.', 'Ник должен содержать от 3 до 16 латинских букв, цифр или знаков подчёркивания.'],
    not_found: ['This account no longer exists.', 'Этот аккаунт больше не существует.'],
    session_security: ['Atlas could not protect the game session file. The game was not started.', 'Atlas не удалось защитить файл игровой сессии. Игра не запущена.'],
  };
  function accountError(error, language = 'ru') {
    const message = typeof error === 'string' ? error : error?.message || '';
    const translation = accountErrors[error?.code] || Object.values(accountErrors).find(([english, russian]) => message === english || message === russian);
    return translation ? translation[language === 'en' ? 0 : 1] : message;
  }
  const en = {
    'Главная':'Home','Мои сборки':'Instances','Мастерская':'Workshop','Настройки':'Settings','Журнал запуска':'Launch log',
    'Разделы сборки':'Instance sections','Обзор':'Overview','Файлы сборки':'Instance files','Новая сборка':'New instance',
    'Пейзаж Atlas':'Atlas landscape','Твой мир.':'Your world.','Твои правила.':'Your rules.','Игра запущена':'Game running',
    'Готово к запуску':'Ready to play','Выбери клиент в настройках':'Choose a client in Settings','Запускаем…':'Starting…','Играть':'Play',
    'Корневая папка':'Root folder','Исходники':'Source code','Ресурспаки':'Resource packs','Выбрана':'Selected','Открыть папку':'Open folder',
    'Открыть':'Open','Выбрать':'Select','Действия со сборкой':'Instance actions','Папка сборки':'Instance folder','Сохранённые миры':'Saved worlds',
    'Отдельные сохранения этой сборки':'Worlds for this instance','Добавляй пакеты ресурсов в формате .zip':'Add resource packs as .zip files',
    'Логи игры':'Game logs','Журнал работы клиента и ошибки запуска':'Client output and launch errors','Открыть исходники':'Open source code',
    'Готово':'Ready','Не найдены':'Not found','Клиент':'Client','Выбрать .exe':'Choose .exe','Инструкция по изменению игры':'Game editing guide',
    'Клиент и исходники':'Client and source code','Выбрать папку':'Choose folder','Скрывать лаунчер во время игры':'Hide launcher while playing',
    'Папки':'Folders','Сборки и настройки':'Instances and settings','Выключено':'Off','Подключение…':'Connecting…','Подключено к Discord':'Connected to Discord',
    'Открой настольный Discord':'Open desktop Discord','Приложение Discord не настроено':'Discord application is not configured',
    'Не удалось подключиться':'Could not connect','Показывать Atlas в Discord':'Show Atlas in Discord','Переподключить':'Reconnect',
    'Папка логов':'Logs folder','Игра не запущена':'Game is not running','Журнал пуст.':'No logs yet.','Запущено окон:':'Running windows:',
    'Сначала закрой окна этой сборки':'Close this instance’s windows first','Должна остаться хотя бы одна сборка':'Keep at least one instance',
    'Удалить сборку':'Delete instance','Остановить':'Stop','Перезапуск':'Restart','Ещё одно окно':'Another window','Управление игрой':'Game controls',
    'Это окно уже закрыто.':'This window has already closed.','Сборка создана':'Instance created','Сборка перемещена в корзину':'Instance moved to Recycle Bin',
    'Не удалось загрузить Atlas:':'Could not load Atlas:','Загрузка Atlas':'Loading Atlas','Свернуть':'Minimize','Развернуть':'Maximize','Закрыть':'Close',
    'Atlas — главная':'Atlas — Home','Основная навигация':'Main navigation','СБОРКИ':'INSTANCES','Добавить сборку':'Add instance',
    'Название':'Name','Название сборки':'Instance name','Описание':'Description','необязательно':'optional','Цвет сборки':'Instance color',
    'Отмена':'Cancel','Создать сборку':'Create instance','Удалить сборку?':'Delete instance?',
    'Миры и настройки сборки, включая дополнительные окна, будут перемещены в корзину.':'Worlds and settings, including additional windows, will be moved to Recycle Bin.',
    'В корзину':'Move to Recycle Bin','Оформление':'Appearance','Язык лаунчера':'Launcher language','Тема':'Theme','Тёмная':'Dark','Светлая':'Light',
    'Акцентный цвет':'Accent color','Оранжевый':'Orange','Синий':'Blue','Зелёный':'Green','Фиолетовый':'Violet','Розовый':'Rose',
    'Персонализация':'Personalization','Фон главной':'Home background','Пейзаж':'Landscape','Закат':'Sunset','Минимализм':'Minimal',
    'Текст на главной':'Home headline','Анимации интерфейса':'Interface animations','Память сборки':'Instance memory','Лимит RAM':'RAM limit',
    'Без ограничения':'Unlimited','ГБ':'GB','МБ':'MB','При следующем запуске. При достижении лимита игре может не хватить памяти.':'Applies on next launch. The game may run out of memory at the limit.',
    'Аккаунты':'Accounts','Локальный профиль':'Local profile','Ник игрока':'Player name','Добавить':'Add','Войти через Microsoft':'Sign in with Microsoft',
    'Выбран':'Selected','Удалить':'Remove','Вход…':'Signing in…','Войти':'Sign in','Код входа':'Sign-in code','Отменить вход':'Cancel sign-in',
    'Открыть Microsoft':'Open Microsoft','Для Microsoft-входа нужен Client ID приложения Atlas.':'Microsoft sign-in requires an Atlas application Client ID.',
    'Проверяем аккаунт…':'Verifying account…','Защищённое хранилище Windows недоступно.':'Windows secure storage is unavailable.',
    'Игра использует выбранный профиль при следующем запуске.':'The selected profile applies on next launch.','Не удалось сохранить настройки.':'Could not save settings.',
    'Некорректная настройка оформления.':'Invalid appearance setting.','Выбери допустимый объём памяти.':'Choose a valid memory amount.',
    'Укажи название сборки: от 1 до 48 символов.':'Enter an instance name: 1–48 characters.','Сборка с таким названием уже есть.':'An instance with this name already exists.',
    'Описание должно быть короче 160 символов.':'Description must be under 160 characters.','Эта сборка не найдена.':'Instance not found.',
    'Последнюю сборку удалить нельзя. Сначала создай другую.':'Create another instance before deleting the last one.',
    'Закрой все окна этой сборки перед удалением.':'Close all windows of this instance before deleting it.',
    'Клиент не найден. Выбери atlas-client.exe в настройках.':'Client not found. Choose atlas-client.exe in Settings.',
    'Подготовленные ресурсы Minecraft 26.3 не найдены. Проверь папку ресурсов установленной игры.':'Prepared Minecraft 26.3 resources were not found. Check the installed game resources.',
    'Atlas Launcher готов. Сборки хранятся в отдельных папках.':'Atlas Launcher is ready. Instances are stored in separate folders.',
    'Выбрать клиент Atlas':'Choose Atlas client','Клиент Atlas':'Atlas client','Папка исходников Atlas':'Atlas source folder',
    'Эта папка ещё не создана.':'This folder has not been created yet.','В выбранной папке нет Cargo.toml проекта игры.':'The selected folder does not contain the game’s Cargo.toml.',
    'Твоя первая сборка Atlas':'Your first Atlas instance', 'Вернуться':'Back', 'ещё окно':'another window',
    'Песочный':'Sand', 'Шалфейный':'Sage'
  };
  Object.assign(en, {
    'Редактировать':'Edit','Переименовать':'Rename','Редактировать сборку':'Edit instance','Переименовать сборку':'Rename instance','Сохранить':'Save','Значок сборки':'Instance icon','Сборка обновлена':'Instance updated',
    'Некорректный цвет сборки.':'Invalid instance color.','Некорректный значок сборки.':'Invalid instance icon.','Некорректные параметры сборки.':'Invalid instance settings.',
    'Поддержать подарком':'Support with a gift','Скрыть предложение поддержки':'Dismiss support notice',
    'Файлы игры не установлены':'Game files are not installed','Скачать и подготовить':'Download and prepare','Выбрать установленный Minecraft':'Choose installed Minecraft',
    'Подготовка игры':'Setting up game','Нужна подготовка игры':'Game setup required','Скачивание файлов':'Downloading files','Проверка файлов':'Verifying files','Подготовка ресурсов':'Preparing resources','Установка клиента':'Installing client',
    'Игра готова':'Game is ready','Не удалось подготовить игру':'Game setup failed','Загрузка отменена':'Download cancelled','Это может занять несколько минут.':'This can take a few minutes.',
    'Скачивание файлов игры':'Downloading game files',
    'Отменить загрузку':'Cancel download','Закрой игру перед подготовкой файлов.':'Close the game before setup.','Повторить':'Retry','Начать установку':'Start setup','В этой сборке нет файлов для подготовки игры. Установи полный пакет Atlas.':'This Atlas build is missing its preparation payload. Install the complete launcher package.',
    'В этой сборке нет игрового клиента. Установи полный пакет Atlas.':'This Atlas build is missing its game payload. Install the complete launcher package.',
    'В этой сборке нет atlas-prepare.exe. Установи полный пакет Atlas.':'This Atlas build is missing atlas-prepare.exe. Install the complete launcher package.',
    'Скачанный файл не прошёл проверку целостности.':'Downloaded file failed its checksum check.',
    'Подготовленные ресурсы игры неполные.':'Prepared game resources are incomplete.',
    'Выбери встроенный клиент Atlas перед установкой.':'Choose the bundled Atlas client before setup.',
    'Метаданные Minecraft не соответствуют версии Atlas.':'Minecraft metadata does not match this Atlas version.',
    'У Mojang нет подходящей Java для этой платформы.':'Mojang has no compatible Java runtime for this platform.',
    'Среда Java установлена не полностью.':'Java runtime is incomplete.'
  });
  function t(text, language = 'ru') {
    if (typeof text !== 'string') return text;
    const translatedAccountError = accountError(text, language);
    if (translatedAccountError !== text) return translatedAccountError;
    if (language !== 'en') { const reverse=Object.keys(en).find(key=>en[key]===text.trim()); return reverse?text.replace(text.trim(),reverse):text; }
    const trimmed = text.trim();
    if (Object.hasOwn(en, trimmed)) return text.replace(trimmed, en[trimmed]);
    // Only translate known UI prefixes; account and instance names remain untouched.
    for (const prefix of ['Запущено окон:', 'Игра запущена', 'Открыть папку', 'Действия со сборкой', 'Не удалось загрузить Atlas:']) {
      if (trimmed.startsWith(prefix + ' ')) return text.replace(prefix, en[prefix]);
    }
    return text;
  }
  const originals = new WeakMap();
  const originalAttributes = new WeakMap();
  function translateTree(element, language) {
    if (!element || typeof document === 'undefined') return;
    const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
    let node;
    while ((node = walker.nextNode())) {
      if (node.parentElement.closest('script,style,.path,[data-user-text],.sidebar-instance > span:last-child,.launch-info strong,.instance-body h3,.instance-body p,.section-heading h3,.context-heading,.context-session span,#log-console')) continue;
      const previous = originals.get(node);
      const source = previous && node.nodeValue === previous.last ? previous.source : node.nodeValue;
      const last = t(source, language);
      originals.set(node, {source, last}); node.nodeValue = last;
    }
    const selector = '[aria-label],[placeholder],[title]';
    const items = [...(element.matches?.(selector) ? [element] : []), ...element.querySelectorAll(selector)];
    for (const item of items) {
      const previous = originalAttributes.get(item) || new Map();
      for (const name of ['aria-label','placeholder','title']) {
        if (!item.hasAttribute(name)) continue;
        const current = item.getAttribute(name);
        const original = previous.get(name);
        const source = original && current === original.last ? original.source : current;
        const last = t(source,language);
        previous.set(name,{source,last});
        item.setAttribute(name,last);
      }
      originalAttributes.set(item,previous);
    }
  }
  const api = { t, translateTree, en, accountError };
  if (typeof module !== 'undefined') module.exports = api;
  else root.AtlasI18n = api;
})(typeof window === 'undefined' ? globalThis : window);
