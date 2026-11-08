'use strict';
let state;
let currentPage = 'home';
let currentTab = 'overview';
let busy = false;
let pendingGameAction = null;
let selectedGameSessionId = null;
let contextMenuState = null;
let contextMenuSignature = '';
let deleteInstanceId = null;
let editInstanceId=null;
let renameOnly=false;
let toastTimer;
let installIntent = null;
let installActionPending = false;
let modsSource = 'atlas';
let modsSearch = '';
let modsBusy = false;
let settingsAnchor = 'appearance';
let instancesTab = 'files';
let renderedView = null;
let viewAnimation;
let accountPickerPage = "list";
let accountPickerSignature = "";
let accountContextId = null;

function motionReduced() {
  return state?.settings?.reducedMotion || matchMedia('(prefers-reduced-motion: reduce)').matches;
}
function positionAccountPicker() {
  const picker=document.getElementById('account-picker'),rect=document.getElementById('profile-button').getBoundingClientRect();
  picker.style.top=`${rect.bottom+10}px`;
  picker.style.right=`${Math.max(16,innerWidth-rect.right)}px`;
  picker.style.maxHeight=`${Math.max(200,innerHeight-rect.bottom-30)}px`;
}
function renderAccountPicker() {
  const picker=document.getElementById('account-picker'),account=state.accounts;
  if(!account)return;
  const signature=JSON.stringify([accountPickerPage,account,state.settings.language]);
  if(signature===accountPickerSignature)return;
  accountPickerSignature=signature;
  const back=button('picker-back','Назад','arrow-left','quiet');
  let content='';
  if(accountPickerPage==='list') {
    content=`<div class="picker-heading"><strong>Аккаунты</strong>${button('picker-add','Добавить','plus','quiet')}</div><div class="picker-list">${account.accounts.map(a=>`<button class="picker-account" data-action="picker-select" data-id="${escapeHtml(a.id)}" aria-pressed="${a.id===account.selectedId}"><span class="profile-avatar" data-user-text>${escapeHtml(a.name.slice(0,1).toUpperCase())}</span><span class="picker-account-copy"><strong data-user-text>${escapeHtml(a.name)}</strong><small>${a.type==='microsoft'?'Microsoft':'Локальный профиль'}</small></span>${a.id===account.selectedId?icon('check'):''}</button>`).join('')}</div>`;
  } else if(accountPickerPage==='add') {
    content=`<div class="picker-heading">${back}<strong>Добавить аккаунт</strong></div>${button('picker-local','Офлайн-профиль','user-round','secondary picker-choice')}${button('picker-microsoft','Microsoft','log-in','secondary picker-choice')}`;
  } else if(accountPickerPage==='local') {
    content=`<div class="picker-heading">${back}<strong>Офлайн-профиль</strong></div><form id="picker-local-form"><label class="field">Ник игрока<input name="name" pattern="[A-Za-z0-9_]{3,16}" minlength="3" maxlength="16" required autocomplete="off"></label><p id="picker-error" class="form-error" role="alert"></p><button class="button primary picker-choice" type="submit">${icon('plus')}Добавить</button></form>`;
  } else {
    const waiting=['starting','waiting','verifying'].includes(account.status);
    content=`<div class="picker-heading">${back}<strong>Microsoft</strong></div>${!account.configured?'<p class="subtitle">Для Microsoft-входа нужен Client ID приложения Atlas.</p>':!account.storageAvailable?'<p class="subtitle">Защищённое хранилище Windows недоступно.</p>':''}${button('signin-microsoft',waiting?'Вход…':'Войти через Microsoft','log-in','primary picker-choice',!account.configured||!account.storageAvailable||waiting?'disabled':'')}${account.device?`<div class="signin-device"><span>Код входа</span><strong data-user-text>${escapeHtml(account.device.userCode)}</strong>${button('open-microsoft','Открыть Microsoft','external-link')}</div>`:''}${waiting?button('cancel-signin','Отменить вход','x','quiet'):''}${account.error?`<p class="form-error">${escapeHtml(AtlasI18n.accountError(account.error,state.settings.language))}</p>`:''}`;
  }
  closeAccountContext();
  document.getElementById('account-picker-content').innerHTML=content;
  hydrateIcons(); AtlasI18n.translateTree(picker,state.settings.language);
  positionAccountPicker();
}
function closeAccountContext(restoreFocus=false) {
  const id=accountContextId;
  accountContextId=null;
  const menu=document.getElementById('account-context-menu');
  if(menu.matches(':popover-open'))menu.hidePopover();
  if(restoreFocus&&id)document.querySelector(`.picker-account[data-id="${CSS.escape(id)}"]`)?.focus();
}
function openAccountContext(row,x,y) {
  accountContextId=row.dataset.id;
  const menu=document.getElementById('account-context-menu');
  if(!menu.matches(':popover-open'))menu.showPopover();
  const rect=menu.getBoundingClientRect();
  menu.style.left=`${Math.max(8,Math.min(x,innerWidth-rect.width-8))}px`;
  menu.style.top=`${Math.max(40,Math.min(y,innerHeight-rect.height-8))}px`;
  menu.querySelector('button').focus();
}
document.getElementById('account-picker').addEventListener('contextmenu',event=>{
  const row=event.target.closest('.picker-account');if(!row)return;
  event.preventDefault();event.stopPropagation();
  const rect=row.getBoundingClientRect();
  openAccountContext(row,event.clientX||rect.left+20,event.clientY||rect.bottom);
});
document.getElementById('account-picker').addEventListener('keydown',event=>{
  const row=event.target.closest('.picker-account');
  if(row&&(event.key==='ContextMenu'||event.shiftKey&&event.key==='F10')){
    event.preventDefault();const rect=row.getBoundingClientRect();openAccountContext(row,rect.left+20,rect.bottom);
  }
});
document.addEventListener('keydown',event=>{
  if(!accountContextId)return;
  if(event.key==='Escape'){event.preventDefault();event.stopImmediatePropagation();closeAccountContext(true);}
  else if(event.key==='Tab')closeAccountContext();
},true);
document.addEventListener('pointerdown',event=>{if(accountContextId&&!event.target.closest('#account-context-menu'))closeAccountContext();},true);
document.getElementById('account-picker').addEventListener('scroll',()=>closeAccountContext());
window.addEventListener('resize',()=>closeAccountContext());
document.getElementById('account-picker').addEventListener('toggle',event=>{
  document.getElementById('profile-button').setAttribute('aria-expanded',String(event.newState==='open'));
  if(event.newState!=='open')closeAccountContext();
});
window.addEventListener('resize',positionAccountPicker);
document.addEventListener('click',async event=>{
  const control=event.target.closest('[data-action]');if(!control)return;
  const action=control.dataset.action;
  if(action==='picker-remove-account'){
    const id=accountContextId;closeAccountContext();
    if(id){const result=await invoke(()=>window.atlas.removeAccount(id));if(result)(document.querySelector('.picker-account[aria-pressed="true"]')||document.querySelector('#account-picker button'))?.focus();}
    return;
  }
  if(action==='picker-select'){const result=await invoke(()=>window.atlas.selectAccount(control.dataset.id));if(result)document.getElementById('account-picker').hidePopover();return;}
  const next={'picker-add':'add','picker-local':'local','picker-microsoft':'microsoft','picker-back':accountPickerPage==='add'?'list':'add'}[action];
  if(next){accountPickerPage=next;renderAccountPicker();document.querySelector('#account-picker input')?.focus();}
});
document.addEventListener('submit',async event=>{
  if(event.target.id!=='picker-local-form')return;
  event.preventDefault();const submit=event.target.querySelector('[type="submit"]');submit.disabled=true;
  try{state=await window.atlas.addLocalAccount(new FormData(event.target).get('name'));accountPickerPage='list';render();}
  catch(error){document.getElementById('picker-error').textContent=error.message;submit.disabled=false;}
});
function syncSceneFocus() {
  document.body.dataset.focusSurface = String(currentPage !== 'home' || currentTab !== 'overview' || Boolean(contextMenuState));
}
const titles = { home: 'Главная', instances: 'Мои сборки', workshop: 'Мастерская', settings: 'Настройки', logs: 'Журнал запуска' };
const accents = AtlasInstanceOptions.colors;
const escapeHtml = value => String(value ?? '').replace(/[&<>"']/g, c => ({ '&':'&amp;', '<':'&lt;', '>':'&gt;', '"':'&quot;', "'":'&#39;' }[c]));
const icon = name => `<i data-lucide="${name}" aria-hidden="true"></i>`;
const selected = () => state.instances.find(i => i.id === state.selectedId) || state.instances[0];
const gameSessions = () => state.game?.sessions || [];
const activeGameSession = () => gameSessions().find(session => session.id === selectedGameSessionId)
  || gameSessions().find(session => session.instanceId === state.selectedId)
  || gameSessions()[0];
const sessionDisplayName = session => session.isolated ? session.name.replace(/ · ещё окно$/, ` · ${tr('ещё окно')}`) : session.name;
const selectedInstanceRunning = () => gameSessions().length
  ? gameSessions().some(session => session.instanceId === state.selectedId)
  : Boolean(state.game?.running && state.game.instanceId === state.selectedId);
const accent = instance => accents[instance.accent] || accents.amber;
const cube = (instance, extra = '') => `<span class="instance-cube ${extra}" style="--accent:${accent(instance)}">${icon(AtlasInstanceOptions.icons.includes(instance.icon)?instance.icon:'box')}</span>`;
const button = (action, label, glyph = '', kind = 'secondary', attrs = '') => `<button type="button" class="button ${kind}" data-action="${action}" ${attrs}>${glyph ? icon(glyph) : ''}${label}</button>`;
function hydrateIcons() { if (window.lucide) window.lucide.createIcons({ attrs: { 'stroke-width': 1.65 } }); }
function notify(message, error = false) {
  const element = document.getElementById('toast');
  clearTimeout(toastTimer);
  element.textContent = tr(message.replace(/^Error invoking remote method '[^']+': Error: /, ''));
  element.className = `toast show${error ? ' error' : ''}`;
  toastTimer = setTimeout(() => element.classList.remove('show'), error ? 8000 : 3500);
}
async function invoke(operation, message) {
  try {
    const result = await operation();
    if (result && Array.isArray(result.instances)) { state = result; render(); }
    if (message) notify(message);
    return result;
  } catch (error) { notify(error.message, true); return null; }
}
function installActive() { return Boolean(state?.install?.active || state?.install?.status === 'running'); }
function openInstall(instanceId) {
  installIntent = { instanceId, startRequested: false };
  renderInstall();
  const dialog = document.getElementById('install-dialog');
  if (!dialog.open) dialog.showModal();
}
function closeInstall() {
  if (installActive()) return;
  installIntent = null;
  document.getElementById('install-dialog').close();
}
async function launchWithSetup(instanceId) {
  busy=true; render();
  try {
    const result=await invoke(()=>window.atlas.launch(instanceId));
    if(result?.install?.needsSetup)openInstall(instanceId);
  } finally {busy=false;render();}
}
function renderInstall() {
  const dialog=document.getElementById('install-dialog');
  if(!dialog || (!dialog.open && !installIntent))return;
  const install=state.install||{};
  const phase=install.phase||install.status||'idle';
  const active=installActive();
  const phases={idle:'Файлы игры не установлены',download:'Скачивание файлов',verify:'Проверка файлов',prepare:'Подготовка ресурсов',install:'Установка клиента',ready:'Игра готова',error:'Не удалось подготовить игру',cancelled:'Загрузка отменена'};
  document.getElementById('install-title').textContent=tr(active?'Подготовка игры':phase==='error'?'Не удалось подготовить игру':phase==='cancelled'?'Загрузка отменена':'Файлы игры не установлены');
  const status=document.getElementById('install-status');
  status.hidden=!active && !['cancelled','ready'].includes(phase);
  document.getElementById('install-stage').textContent=tr(phases[phase]||'Подготовка игры');
  const bar=document.getElementById('install-progress');
  const raw=typeof install.progress==='number' && Number.isFinite(install.progress)?install.progress:null;
  const percent=raw===null?null:Math.max(0,Math.min(100,raw<=1?raw*100:raw));
  if(percent===null)bar.removeAttribute('value');else bar.value=percent;
  const count=Number.isFinite(install.total)&&install.total>0?`${Math.min(install.completed||0,install.total)} / ${install.total}`:'';
  document.getElementById('install-progress-value').textContent=percent===null?count:`${Math.floor(percent)}%`;
  const bytes=Number.isFinite(install.bytesTotal)&&install.bytesTotal>0?`${Math.floor((install.bytesDone||0)/1024**2)} / ${Math.ceil(install.bytesTotal/1024**2)} ${tr('МБ')}`:'';
  document.getElementById('install-detail').textContent=active?(bytes||count||tr('Это может занять несколько минут.')):phase==='cancelled'?tr('Повторный запуск продолжит подготовку.'):'';
  const error=document.getElementById('install-error');
  error.hidden=!install.error;
  error.textContent=install.error?tr(install.error.message||install.error):'';
  document.getElementById('install-start-label').textContent=tr(['error','cancelled'].includes(phase)?'Повторить':'Скачать и подготовить');
  dialog.querySelector('[data-action="start-install"]').disabled=active||installActionPending;
  dialog.querySelector('[data-action="import-minecraft"]').disabled=active||installActionPending;
  dialog.querySelector('[data-action="close-install"]').disabled=active||installActionPending;
  const cancel=dialog.querySelector('[data-action="cancel-install"]');
  cancel.disabled=installActionPending||(active&&install.canCancel===false);
  cancel.textContent=tr(active?'Отменить загрузку':'Отмена');
  AtlasI18n.translateTree(dialog,state.settings.language);
  if(installIntent?.startRequested && !active && install.status==='ready' && install.ready && !install.error){
    const id=installIntent.instanceId;
    installIntent=null;
    dialog.close();
    queueMicrotask(()=>launchWithSetup(id));
  }
}
async function runInstall(importExisting=false) {
  if(installActionPending||installActive()||!installIntent)return;
  installActionPending=true;
  installIntent.startRequested=true;
  renderInstall();
  try {
    const result=await invoke(()=>importExisting?window.atlas.chooseMinecraftInstall():window.atlas.startInstall());
    if(!result || (importExisting && result.install?.status==='idle' && !result.install?.active)) {
      if(installIntent)installIntent.startRequested=false;
    }
  } finally {installActionPending=false;renderInstall();}
}
async function cancelInstall() {
  if(installActionPending)return;
  if(installIntent)installIntent.startRequested=false;
  if(!installActive()){closeInstall();return;}
  installActionPending=true;renderInstall();
  try{await invoke(()=>window.atlas.cancelInstall());}
  finally{installActionPending=false;renderInstall();}
}
function intro(title, subtitle, action = '') {
  return `<div class="page-intro"><h1>${title}</h1>${action}</div>`;
}
function homePage() {
  const instance = selected();
  const selectedRunning = selectedInstanceRunning();
  const top = intro('Главная', '');
  const status=selectedRunning?'Игра запущена':installActive()?'Подготовка игры':state.install && !state.install.ready?'Нужна подготовка игры':state.runtimeAvailable?'Готово к запуску':'Выбери клиент в настройках';
  return top + `<div class="home-overview"><section class="hero" aria-label="Пейзаж Atlas"><div class="hero-copy"><p class="hero-status"><span class="status-dot"></span><strong>${status}</strong><span data-user-text> · ${escapeHtml(instance.name)} · ${(instance.memoryMb||4096)/1024} ${tr('ГБ')}</span>${selectedRunning?` · <span class="playtime" data-session-id="${activeGameSession()?.id}"></span>`:''}</p><h2>Твой мир.<br>Твои правила.</h2></div></section>
  <div class="home-launch-actions"><div class="launch-bar">${selectedRunning ? button('game-menu','Игра запущена','chevron-down','running launch-button',`aria-haspopup="menu" aria-expanded="${contextMenuState?.type==='game'}" ${pendingGameAction?'disabled':''}`) : button('launch',busy?'Запускаем…':installActive()?'Подготовка игры':'Играть','play','primary launch-button',busy||pendingGameAction||installActive()?'disabled':'')}</div><div id="home-dock-slot"></div></div></div>`;
}
function modsPage() {
  const data=state.mods||{ports:[],horizon:[]},en=state.settings.language==='en';
  const items=(modsSource==='atlas'?data.ports:data.horizon).filter(m=>`${m.name} ${m.description||m.summary||''}`.toLowerCase().includes(modsSearch.toLowerCase()));
  const blocked=modsBusy||selectedInstanceRunning();
  const action=(m,kind,label,glyph='download')=>button('change-mod',tr(label),glyph,kind==='install'?'primary':'secondary',`data-mod="${escapeHtml(m.id)}" data-source="${modsSource}" data-mod-action="${kind}" ${blocked?'disabled':''}`);
  return `<section class="mods-section"><div class="mods-toolbar"><div class="mods-sources" role="group" aria-label="${tr('Каталог модов')}">${[['atlas','Atlas Rust'],['horizon','Horizon']].map(([id,label])=>`<button type="button" class="tab ${modsSource===id?'active':''}" data-mods-source="${id}">${label}</button>`).join('')}</div><input type="search" name="mod-search" value="${escapeHtml(modsSearch)}" placeholder="${tr('Поиск модов')}" aria-label="${tr('Поиск модов')}">${modsSource==='horizon'?button('refresh-mods',tr(modsBusy?'Загрузка…':'Обновить'),'refresh-cw','secondary',modsBusy?'disabled':''):''}</div>
  <div class="mods-caption"><strong>${escapeHtml(selected().name)}</strong><span>${tr(selectedInstanceRunning()?'Закрой игру для изменения модов.':'Моды применяются при следующем запуске игры.')}</span></div>
  ${modsSource==='horizon'?`<p class="subtitle">${tr('Каталог Horizon. Доступные аналоги Atlas отмечены отдельно; остальным нужна адаптация и нативная сборка.')}</p>`:''}
  ${data.error?`<p class="mods-error">${escapeHtml(tr(data.error))}</p>`:''}
  <div class="mods-grid">${items.map(m=>{
    const adapted=modsSource==='horizon'&&Boolean(m.port),installable=modsSource==='atlas'||adapted;
    const description=modsSource==='atlas'?(en?m.descriptionEn:m.description):adapted?tr('Доступна встроенная версия приближения Atlas по клавише C.'):m.summary;
    const scope=modsSource==='atlas'?(en?m.scopeEn:m.scope):adapted?tr('Аналог Atlas: приближение 4× по C. Исходный мод Horizon не устанавливается.'):tr(m.kind==='native-mod'?'Нужна адаптация нативной сборки.':'WebAssembly-моду нужна новая реализация.');
    return `<article class="mod-card"><div class="mod-card-title"><span class="mod-icon">${icon(m.icon||'package')}</span><div><h3>${escapeHtml(m.name)}</h3><span class="tag">${tr(adapted?'Аналог Atlas':modsSource==='atlas'?'Rust · Atlas':'Horizon')} ${m.version&&!adapted?escapeHtml(m.version):''}</span></div></div><p>${escapeHtml(description)}</p><small>${escapeHtml(scope)}</small><div class="mod-actions">${installable?(m.installed?`${action(m,m.enabled?'disable':'enable',m.enabled?'Выключить':'Включить',m.enabled?'toggle-right':'toggle-left')}${action(m,'remove','Удалить','trash-2')}`:action(m,'install',adapted?'Установить аналог':'Установить')):`<span class="mod-unavailable">${tr('Нужна адаптация')}</span>`}${modsSource==='horizon'?button('mod-page',tr('Страница'),'arrow-up-right','quiet',`data-mod="${escapeHtml(m.id)}"`):''}</div></article>`;
  }).join('')}</div>${!items.length?`<div class="empty-state">${tr(modsSource==='horizon'&&!data.horizon.length?'Нажми «Обновить», чтобы загрузить каталог Horizon.':'Моды не найдены.')}</div>`:''}</section>`;
}
document.addEventListener('input',event=>{
  if(event.target.name==='mod-search'){modsSearch=event.target.value.slice(0,100);render();}
});
function instancesPage() {
  const instance=selected();
  const tiles=state.instances.map(item=>`<button type="button" class="instance-tile ${item.id===state.selectedId?'selected':''}" data-instance="${item.id}" data-action="select-instance" data-id="${item.id}" aria-pressed="${item.id===state.selectedId}" style="--accent:${accent(item)}">${cube(item)}<strong data-user-text>${escapeHtml(item.name)}</strong><span>${item.id===state.selectedId?'Выбрана':`Atlas ${escapeHtml(state.version)}`}</span></button>`).join('');
  const tabs=`<div class="instance-detail-tabs" role="tablist" aria-label="Разделы сборки">${[['files','Файлы сборки'],['mods','Моды']].map(([id,label])=>`<button type="button" class="tab ${instancesTab===id?'active':''}" role="tab" aria-selected="${instancesTab===id}" aria-controls="instance-detail-content" data-instance-tab="${id}">${label}</button>`).join('')}</div>`;
  return `<div class="instances-workspace">${intro('Мои сборки','')}<div class="instance-tiles">${tiles}<button type="button" class="instance-tile instance-tile-new" data-action="new-instance">${icon('plus')}<span>Добавить сборку</span></button></div><section class="instance-details"><h2 data-user-text>${escapeHtml(instance.name)}</h2>${tabs}<div id="instance-detail-content" role="tabpanel" aria-label="${tr(instancesTab==='files'?'Файлы сборки':'Моды')}">${instancesTab==='files'?filePage():modsPage()}</div></section></div>`;
}
function filePage() {
  const entries=[['saves','mountain','Сохранённые миры','saves'],['mods','package','Моды','atlas-mods'],['resourcepacks','package-open','Ресурспаки','resourcepacks'],['shaderpacks','sparkles','Шейдерпаки','shaderpacks'],['config','settings-2','Настройки модов','config'],['logs','file-text','Логи игры','logs']];
  return `<div class="instance-folder-list">${entries.map(([kind,glyph,label,folder])=>`<button type="button" class="file-row" data-open="${kind}" aria-label="${tr('Открыть')}: ${escapeHtml(folder)}">${icon(glyph)}<div><strong>${label}</strong><p data-user-text>${folder}</p></div><span class="file-open">Открыть</span></button>`).join('')}</div>`;
}
function workshopPage() {
  return intro('Мастерская', '', button('open-source','Открыть исходники','folder-code','primary')) + `<section class="panel"><div class="panel-heading">${icon('braces')}<h3>Atlas ${escapeHtml(state.version)}</h3></div><div class="settings-row"><div><strong>Исходники</strong><p class="path">${escapeHtml(state.sourceRoot)}</p></div><span class="tag">${state.sourceAvailable?'Готово':'Не найдены'}</span></div><div class="settings-row"><div><strong>Клиент</strong><p class="path">${escapeHtml(state.runtimePath)}</p></div>${button('choose-runtime','Выбрать .exe','folder')}</div></section><div class="workshop-actions">${button('open-guide','Инструкция по изменению игры','book-open','secondary')}</div>`;
}
function settingsPage() {
  const categories = [['appearance','palette','Оформление'],['memory','memory-stick','Память сборки'],['accounts','circle-user-round','Аккаунты'],['discord','radio','Discord'],['updates','download','Обновления'],['files','folder-code','Клиент и исходники']];
  const section = (id, content) => `<div class="settings-section" id="settings-${id}">${content}</div>`;
  return intro('Настройки', '') + `<div class="settings-layout"><nav class="settings-navigation" aria-label="Настройки">${categories.map(([id,glyph,label])=>`<button type="button" class="settings-category ${id===settingsAnchor?'active':''}" data-settings-anchor="${id}">${icon(glyph)}<span>${label}</span></button>`).join('')}</nav><div class="settings-panels">`
    + section('appearance', appearanceSettingsPanel()) + section('memory', memorySettingsPanel()) + section('accounts', accountSettingsPanel())
    + section('discord', discordSettingsPanel()) + section('updates', updateSettingsPanel()) + section('files', `<section class="panel"><div class="panel-heading">${icon('box')}<h3>Клиент и исходники</h3></div><div class="settings-row"><div><strong>Клиент</strong><p class="path">${escapeHtml(state.runtimePath)}</p></div>${button('choose-runtime','Выбрать .exe','folder')}</div><div class="settings-row"><div><strong>Исходники</strong><p class="path">${escapeHtml(state.sourceRoot)}</p></div><div class="row-actions">${button('open-source','Открыть исходники','arrow-up-right')}${button('choose-source','Выбрать папку','folder-code')}</div></div><div class="settings-row"><div><strong>Скрывать лаунчер во время игры</strong></div><label class="switch"><input type="checkbox" id="close-on-launch" ${state.settings.closeOnLaunch?'checked':''} aria-label="Скрывать лаунчер во время игры"><span class="switch-track"></span></label></div></section><section class="panel"><div class="panel-heading">${icon('hard-drive')}<h3>Папки</h3></div><div class="settings-row"><div><strong>Корневая папка</strong><p class="path">${escapeHtml(state.projectRoot)}</p></div>${button('open-project','Открыть','arrow-up-right')}</div><div class="settings-row"><div><strong>Сборки и настройки</strong><p class="path">${escapeHtml(state.dataRoot)}</p></div><span class="tag">${state.instances.length}</span></div></section>`)
    + '</div></div>';
}

function updateLabel() {
  const update=state.update;
  if(!update)return tr('Автообновление доступно в готовой сборке.');
  const labels={idle:'Обновления проверяются автоматически.',checking:'Проверка обновлений…',available:'Скачивание обновления',downloading:'Скачивание обновления',ready:'Обновление готово',installing:'Обновление устанавливается.',upToDate:'Установлена последняя версия',error:'Не удалось проверить обновления',disabled:'Автообновление доступно в готовой сборке.'};
  const suffix=update.status==='downloading'&&Number.isFinite(update.progress)?` · ${Math.round(update.progress*100)}%`:'';
  return tr(labels[update.status]||labels.idle)+suffix;
}
function updateSettingsPanel() {
  const update=state.update;
  const working=update?.applying||['checking','downloading','installing'].includes(update?.status);
  const ready=update?.status==='ready';
  const blocked=Boolean(state.game?.running||state.game?.busy||installActive());
  const detail=ready?tr(blocked?'Закрой игру для установки обновления.':'Обновление установится при закрытии лаунчера.'):'';
  return `<section class="panel"><div class="panel-heading">${icon('download')}<h3>${tr('Обновления')}</h3><span class="tag">atlas-${escapeHtml(state.version)}</span></div><div class="settings-row"><div><strong>${escapeHtml(updateLabel())}${ready?` · ${escapeHtml(update.version)}`:''}</strong>${detail?`<p>${escapeHtml(detail)}</p>`:''}${update?.error?`<p class="form-error">${escapeHtml(update.error)}</p>`:''}</div>${button(ready?'apply-update':'check-updates',tr(ready?'Перезапустить':'Проверить обновления'),ready?'rotate-cw':'refresh-cw','secondary',!update||working||ready&&blocked?'disabled':'')}</div></section>`;
}
function discordSettingsPanel() {
  const discord=state.discord||{enabled:false,status:'disabled'};
  const labels={disabled:'Выключено',connecting:'Подключение…',connected:'Подключено к Discord',waiting_for_discord:'Открой настольный Discord',needs_application_id:'Приложение Discord не настроено',error:'Не удалось подключиться'};
  const status=labels[discord.status]||'Подключение…';
  return `<section class="panel"><div class="panel-heading">${icon('radio')}<h3>Discord Rich Presence</h3></div><div class="settings-row"><div><strong>Показывать Atlas в Discord</strong><p>${escapeHtml(status)}</p></div><label class="switch"><input type="checkbox" id="discord-presence" ${discord.enabled?'checked':''} aria-label="Показывать Atlas в Discord"><span class="switch-track"></span></label></div><div class="discord-preview"><img src="assets/icon.png" alt=""><div><strong>Atlas</strong><span>The Fastest Minecraft Launcher</span><small>v${escapeHtml(state.version)}</small></div></div>${discord.enabled&&discord.status!=='connected'?`<p class="subtitle">${escapeHtml(discord.error||'')}${button('refresh-discord','Переподключить','refresh-cw','quiet')}</p>`:''}</section>`;
}
function logsPage() {
  return intro('Журнал запуска', '', button('open-logs','Папка логов','folder-open')) + `<div class="log-toolbar"><span class="tag"><span class="status-dot"></span>${state.game?.running?'Игра запущена':'Игра не запущена'}</span></div><div class="log-console" id="log-console">${state.logs?.length?state.logs.map(entry=>escapeHtml(`[${new Date(entry.time).toLocaleTimeString(state.settings.language==='en'?'en-US':'ru-RU')}] ${entry.level==='error'?'ERROR  ':'INFO   '}${entry.message}`)).join('\n'):'Журнал пуст.'}</div>`;
}
function render() {
  if (!state) return;
  const focused = document.activeElement;
  const focusedRange = focused?.matches('input[type="range"]') ? focused.id : null;
  const editable = focused?.matches('input:not([type="checkbox"]):not([type="radio"]):not([type="range"])') && document.getElementById('content').contains(focused)
    ? { name: focused.name, value: focused.value, start: focused.selectionStart, end: focused.selectionEnd } : null;
  const contentScroll = document.getElementById('content').scrollTop;
  applyAppearance();
  document.getElementById('content').dataset.page = currentPage;
  document.body.dataset.page = currentPage;
  document.querySelectorAll('.nav-item[data-page]').forEach(element => {
    const active = element.dataset.page === currentPage;
    element.classList.toggle('active', active);
    if (active) element.setAttribute('aria-current', 'page'); else element.removeAttribute('aria-current');
  });
  document.getElementById('support-notice').hidden = state.supportVisible===false;
  const account = state.accounts?.accounts?.find(item => item.id === state.accounts.selectedId);
  document.getElementById('profile-button').innerHTML = `<span class="profile-avatar" data-user-text>${escapeHtml((account?.name || 'Atlas').slice(0,1).toUpperCase())}</span><span class="profile-copy"><strong data-user-text>${escapeHtml(account?.name || 'Atlas')}</strong><small>${account?.type === 'microsoft' ? 'Microsoft' : tr('Офлайн')}</small></span>`;
  document.getElementById('atlas-version').textContent = state.version;
  const updateNotice=document.getElementById('update-notice');
  updateNotice.hidden=!['downloading','ready'].includes(state.update?.status);
  updateNotice.innerHTML=state.update?.status==='ready'
    ? `<span>${tr('Обновление готово')}</span>${button('apply-update',tr('Перезапустить'),'rotate-cw','quiet',state.game?.running||state.game?.busy||state.update.applying||installActive()?'disabled':'')}`
    : `<span>${escapeHtml(updateLabel())}</span>`;
  document.getElementById('instance-count').textContent = state.instances.length;
  document.getElementById('sidebar-instances').innerHTML = state.instances.map(instance=>`<button class="sidebar-instance ${instance.id===state.selectedId?'selected':''}" data-action="select-instance" data-id="${instance.id}">${cube(instance)}<span>${escapeHtml(instance.name)}</span></button>`).join('');
  const renderer = {home:homePage,instances:instancesPage,workshop:workshopPage,settings:settingsPage,logs:logsPage};
  const dock=document.querySelector('.instance-dock');
  document.querySelector('main').append(dock);
  document.getElementById('content').innerHTML = renderer[currentPage]();
  document.getElementById('home-dock-slot')?.append(dock);
  renderAccountPicker();
  hydrateIcons();
  if (contextMenuState) renderContextMenu();
  renderInstall();
  updatePlaytime();
  AtlasI18n.translateTree(document.body,state.settings.language);
  syncSceneFocus();
  const view = `${currentPage}:${currentPage==='instances'?instancesTab+':'+state.selectedId:''}`;
  if(renderedView!==null && view!==renderedView) {
    viewAnimation?.cancel();
    if(!motionReduced())viewAnimation=document.getElementById('content').animate([
      {opacity:.15,transform:'translateY(10px)'}, {opacity:1,transform:'translateY(0)'}
    ],{duration:280,easing:'cubic-bezier(.2,.8,.2,1)'});
  } else if(motionReduced())viewAnimation?.cancel();
  renderedView=view;
  document.getElementById('content').scrollTop = contentScroll;
  if(focusedRange)document.getElementById(focusedRange)?.focus({preventScroll:true});
  if (editable?.name) {
    const replacement = document.querySelector(`#content input[name="${CSS.escape(editable.name)}"]`);
    if (replacement) { replacement.value=editable.value; replacement.focus({preventScroll:true}); replacement.setSelectionRange(editable.start,editable.end); }
  }
}

function closeContextMenu(restoreFocus = false) {
  const previous = contextMenuState;
  contextMenuState = null;
  contextMenuSignature = '';
  document.getElementById('context-menu').hidden = true;
  syncSceneFocus();
  document.querySelector('[data-action="game-menu"]')?.setAttribute('aria-expanded','false');
  if (restoreFocus && previous) {
    const selector = previous.type==='game' ? '[data-action="game-menu"]' : currentPage==='instances'?`.instance-tile[data-id="${CSS.escape(previous.id)}"]`:`.sidebar-instance[data-id="${CSS.escape(previous.id)}"]`;
    document.querySelector(selector)?.focus();
  }
}
function openContextMenu(type, id, x, y, focus = false) {
  const scrollOffsets=new Map([...document.querySelectorAll('.sidebar-instances, .content'),document].map(element=>[element,element===document?[scrollX,scrollY]:[element.scrollLeft,element.scrollTop]]));
  contextMenuState = {type,id,x,y,scrollOffsets};
  syncSceneFocus();
  contextMenuSignature = '';
  if(type==='game')contextMenuState.sessionId=activeGameSession()?.id;
  renderContextMenu();
  if(focus)document.querySelector('#context-menu button:not(:disabled)')?.focus();
}
function renderContextMenu() {
  const menu = document.getElementById('context-menu');
  const menuState = contextMenuState;
  if(!menuState)return;
  let html;
  const item = (action,label,glyph,attrs='')=>`<button type="button" role="menuitem" class="context-item" data-action="${action}" ${attrs}>${icon(glyph)}<span>${label}</span></button>`;
  if(menuState.type==='instance'){
    const instance=state.instances.find(item=>item.id===menuState.id);
    if(!instance){closeContextMenu();return;}
    const running=gameSessions().some(session=>session.instanceId===instance.id);
    const reason=running?'Сначала закрой окна этой сборки':state.instances.length===1?'Должна остаться хотя бы одна сборка':'';
    html=`<div class="context-heading">${escapeHtml(instance.name)}</div>${item('edit-instance','Редактировать','pencil',`data-id="${instance.id}"`)}${item('rename-instance','Переименовать','text-cursor-input',`data-id="${instance.id}"`)}${item('menu-open-instance','Открыть папку','folder-open',`data-id="${instance.id}"`)}${item('delete-instance','Удалить сборку','trash-2',`data-id="${instance.id}" ${reason?`disabled title="${reason}"`:''}`)}`;
    menu.setAttribute('aria-label','Действия со сборкой');
  }else{
    const session=gameSessions().find(item=>item.id===menuState.sessionId);
    if(!session){closeContextMenu();return;}
    const disabled=busy||pendingGameAction||session.status==='stopping'||session.status==='restarting'?'disabled':'';
    const sessions=gameSessions();
    const choices=sessions.length>1?`${sessions.map((choice,index)=>`<button type="button" role="menuitemradio" aria-checked="${choice.id===session.id}" class="context-item context-session" data-action="choose-game-session" data-session-id="${choice.id}">${icon(choice.id===session.id?'check':'monitor')}<span>${escapeHtml(sessionDisplayName(choice))} · ${index+1}</span></button>`).join('')}<hr>`:'';
    const attrs=`data-session-id="${session.id}" ${disabled}`;
    html=`<div class="context-heading">${escapeHtml(sessionDisplayName(session))}</div>${choices}${item('stop-game','Остановить','square',attrs)}${item('restart-game','Перезапуск','rotate-cw',attrs)}${item('another-game','Ещё одно окно','copy-plus',attrs)}`;
    menu.setAttribute('aria-label','Управление игрой');
  }
  if(html!==contextMenuSignature){menu.innerHTML=html;contextMenuSignature=html;hydrateIcons();}
  AtlasI18n.translateTree(menu,state.settings.language);
  menu.hidden=false;
  const box=menu.getBoundingClientRect();
  menu.style.left=`${Math.max(8,Math.min(menuState.x,innerWidth-box.width-8))}px`;
  menu.style.top=`${Math.max(44,Math.min(menuState.y,innerHeight-box.height-8))}px`;
  document.querySelector('[data-action="game-menu"]')?.setAttribute('aria-expanded',String(menuState.type==='game'));
}
document.addEventListener('contextmenu', event=>{
  const game=event.target.closest('[data-action="game-menu"]');
  const instance=event.target.closest('.sidebar-instance, [data-instance]');
  if(!game&&!instance){closeContextMenu();return;}
  event.preventDefault();
  const target=game||instance;
  const rect=target.getBoundingClientRect();
  openContextMenu(game?'game':'instance',instance?.dataset.id||instance?.dataset.instance,event.clientX||rect.left,event.clientY||rect.bottom,event.clientX===0&&event.clientY===0);
});
document.addEventListener('pointerdown',event=>{if(contextMenuState&&!event.target.closest('#context-menu'))closeContextMenu();},true);
document.addEventListener('scroll',event=>{
  if(!contextMenuState||event.target.closest?.('#context-menu'))return;
  const previous=contextMenuState.scrollOffsets.get(event.target);
  const current=event.target===document?[scrollX,scrollY]:[event.target.scrollLeft,event.target.scrollTop];
  if(previous&&previous[0]===current[0]&&previous[1]===current[1])return;
  closeContextMenu();
},true);
window.addEventListener('resize',()=>closeContextMenu());
window.addEventListener('blur',()=>closeContextMenu());
document.addEventListener('keydown',event=>{
  if(!contextMenuState)return;
  if(event.key==='Escape'){event.preventDefault();closeContextMenu(true);return;}
  if(event.key==='Tab'){closeContextMenu();return;}
  if(!['ArrowDown','ArrowUp','Home','End'].includes(event.key))return;
  event.preventDefault();
  const items=[...document.querySelectorAll('#context-menu button:not(:disabled)')];
  const index=items.indexOf(document.activeElement);
  const next=event.key==='Home'?0:event.key==='End'?items.length-1:event.key==='ArrowDown'?(index+1)%items.length:(index-1+items.length)%items.length;
  items[next]?.focus();
});
document.addEventListener('click', async event => {
  const control = event.target.closest('button,a');
  if (!control) return;
  if (control.dataset.action === 'account-settings') {
    const picker=document.getElementById('account-picker');
    if(!picker.matches(':popover-open')){accountPickerPage='list';renderAccountPicker();}
    picker.togglePopover(); positionAccountPicker(); if(picker.matches(':popover-open'))(picker.querySelector('.picker-account[aria-pressed="true"]')||picker.querySelector('button'))?.focus(); return;
  }
  if (control.dataset.settingsAnchor) {
    const anchor = control.dataset.settingsAnchor;
    settingsAnchor = anchor;
    document.getElementById(`settings-${anchor}`)?.scrollIntoView({ behavior: state.settings.reducedMotion || matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth', block: 'start' });
    document.querySelectorAll('.settings-category').forEach(item => item.classList.toggle('active', item.dataset.settingsAnchor === anchor));
    return;
  }
  if (control.dataset.window) { await invoke(()=>window.atlas.windowControl(control.dataset.window)); return; }
  if (control.dataset.page || control.classList.contains('brand')) {
    currentPage = control.dataset.page || 'home';
    if(currentPage==='home' && control.closest('.primary-navigation'))currentTab='overview';
    if (currentPage === 'settings') settingsAnchor = 'appearance';
    document.getElementById('content').scrollTop = 0;
    render(); return;
  }
  if (control.dataset.tab) { currentTab = control.dataset.tab; render(); return; }
  if (control.dataset.instanceTab) { instancesTab=control.dataset.instanceTab; render(); return; }
  if(control.dataset.modsSource){modsSource=control.dataset.modsSource;render();if(modsSource==='horizon'&&!state.mods?.horizon?.length&&!modsBusy){modsBusy=true;render();try{await invoke(()=>window.atlas.refreshMods());}finally{modsBusy=false;render();}}return;}
  if (control.dataset.open) { await invoke(()=>window.atlas.openFolder(control.dataset.open,control.dataset.id||state.selectedId)); return; }
  const action = control.dataset.action;
  if(action==='refresh-mods'){modsBusy=true;render();try{await invoke(()=>window.atlas.refreshMods());}finally{modsBusy=false;render();}return;}
  if(action==='mod-page'){await invoke(()=>window.atlas.modPage(control.dataset.mod));return;}
  if(action==='change-mod'){modsBusy=true;render();try{await invoke(()=>window.atlas.changeMod(state.selectedId,control.dataset.source,control.dataset.mod,control.dataset.modAction),'Моды сборки обновлены.');}finally{modsBusy=false;render();}return;}
  if(control.closest('#context-menu')&&action!=='choose-game-session')closeContextMenu();
  if (action === 'new-instance') {
    openInstanceEditor();
  } else if(action==='edit-instance'||action==='rename-instance'){openInstanceEditor(control.dataset.id,action==='rename-instance');}
  else if(action==='support-gift')await invoke(()=>window.atlas.supportGift());
  else if(action==='dismiss-support')await invoke(()=>window.atlas.dismissSupport());
  else if (action === 'close-dialog') document.getElementById('instance-dialog').close();
  else if (action === 'close-delete') document.getElementById('delete-dialog').close();
  else if (action === 'instance-menu' || action === 'game-menu') {
    const rect=control.getBoundingClientRect();
    openContextMenu(action==='game-menu'?'game':'instance',control.dataset.id,rect.right-280,rect.bottom+6,true);
  }
  else if (action === 'choose-game-session') {
    selectedGameSessionId=control.dataset.sessionId;
    contextMenuState.sessionId=selectedGameSessionId;
    renderContextMenu();
  }
  else if (action === 'menu-open-instance') await invoke(()=>window.atlas.openFolder('instance',control.dataset.id));
  else if (action === 'delete-instance') {
    const instance=state.instances.find(item=>item.id===control.dataset.id);
    if(!instance)return;
    deleteInstanceId=instance.id;
    document.getElementById('delete-name').textContent=instance.name;
    document.getElementById('delete-error').textContent='';
    document.getElementById('delete-dialog').showModal();
  }
  else if (action === 'select-instance') { const keepPage=currentPage==='instances'; const next = await invoke(()=>window.atlas.selectInstance(control.dataset.id)); if(next){if(!keepPage)currentPage='home';currentTab='overview';render();} }
  else if (action === 'launch' && !busy) {
    await launchWithSetup(state.selectedId);
  }
  else if(action==='start-install')await runInstall();
  else if(action==='import-minecraft')await runInstall(true);
  else if(action==='cancel-install')await cancelInstall();
  else if(action==='close-install')closeInstall();
  else if (['stop-game','restart-game','another-game'].includes(action) && !pendingGameAction) {
    const session = control.dataset.sessionId ? gameSessions().find(item=>item.id===control.dataset.sessionId) : activeGameSession();
    if(!session){notify('Это окно уже закрыто.',true);return;}
    const previousIds = new Set(gameSessions().map(item=>item.id));
    pendingGameAction = action==='stop-game'?'stop':action==='restart-game'?'restart':'another';
    render();
    try {
      const result = await invoke(() => action==='stop-game'
        ? window.atlas.stopGame(session?.id)
        : action==='restart-game'
          ? window.atlas.restartGame(session?.id)
          : window.atlas.launchAnother(session?.instanceId || state.selectedId));
      if (result && action!=='stop-game') {
        const added = result.game?.sessions?.find(item=>!previousIds.has(item.id));
        if (added) selectedGameSessionId = added.id;
      }
    } finally { pendingGameAction=null;render(); }
  }
  else if (action === 'choose-runtime') await invoke(()=>window.atlas.chooseRuntime());
  else if (action === 'choose-source') await invoke(()=>window.atlas.chooseSource());
  else if (action === 'open-source') await invoke(()=>window.atlas.openFolder('source'));
  else if (action === 'open-project') await invoke(()=>window.atlas.openFolder('project'));
  else if (action === 'open-logs') await invoke(()=>window.atlas.openFolder('logs'));
  else if (action === 'open-guide') await invoke(()=>window.atlas.openFolder('guide'));
  else if (action === 'refresh-discord') await invoke(()=>window.atlas.refreshDiscord());
  else if (action === 'check-updates') await invoke(()=>window.atlas.checkUpdates());
  else if (action === 'apply-update') await invoke(()=>window.atlas.applyUpdate());
});
document.addEventListener('change', async event => {
  const element = event.target;
  if (element.id === 'game-window') { selectedGameSessionId=element.value;render();return; }
  if (element.id === 'close-on-launch') await invoke(()=>window.atlas.saveSettings({closeOnLaunch:element.checked}));
  if (element.id === 'discord-presence') await invoke(()=>window.atlas.saveSettings({discordRichPresence:element.checked}));
});
document.getElementById('instance-form').addEventListener('submit', async event => {
  event.preventDefault();
  const data = new FormData(event.currentTarget);
  const submit = event.currentTarget.querySelector('[type="submit"]');submit.disabled=true;
  try {
    const patch={name:data.get('name'),description:data.get('description'),accent:data.get('accent'),icon:data.get('icon')};
    state=editInstanceId?await window.atlas.updateInstance(editInstanceId,renameOnly?{name:patch.name}:patch):await window.atlas.createInstance(patch);
    document.getElementById('instance-dialog').close(); if(!editInstanceId)currentPage='instances';render();notify(editInstanceId?'Сборка обновлена':'Сборка создана');
  } catch(error) { document.getElementById('form-error').textContent=tr(error.message.replace(/^Error invoking remote method '[^']+': Error: /,'')); }
  finally { submit.disabled=false; }
});
document.getElementById('instance-dialog').addEventListener('click',event=>{ if(event.target===event.currentTarget){const r=event.currentTarget.getBoundingClientRect();if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom)event.currentTarget.close();} });
document.getElementById('delete-form').addEventListener('submit',async event=>{
  event.preventDefault();
  const submit=event.currentTarget.querySelector('[type="submit"]');
  submit.disabled=true;
  try{
    state=await window.atlas.deleteInstance(deleteInstanceId);
    document.getElementById('delete-dialog').close();
    deleteInstanceId=null;
    render();notify('Сборка перемещена в корзину');
  }catch(error){document.getElementById('delete-error').textContent=tr(error.message.replace(/^Error invoking remote method '[^']+': Error: /,''));}
  finally{submit.disabled=false;}
});
window.atlas.onGameEvent(next=>{state=next;if(next.game?.running)busy=false;render();});
document.getElementById('install-dialog').addEventListener('cancel',event=>{event.preventDefault();cancelInstall();});
const initialStateReady = (async()=>{try{state=await window.atlas.getState();render();}catch(error){document.getElementById('content').textContent='Не удалось загрузить Atlas: '+error.message;}})();
{
  const splash = document.getElementById('boot-splash');
  const deadline = Number(new URLSearchParams(location.search).get('startupDeadline'));
  const remaining = deadline > 0 ? Math.max(0, Math.min(1000, deadline - Date.now())) : 1000;
  splash?.style.setProperty('--boot-offset', `${remaining - 1000}ms`);
  const startedAt = performance.now();
  setTimeout(async () => {
    await initialStateReady;
    splash?.remove();
    document.querySelector('.app-shell').inert = false;
    performance.mark('atlas:ready');
    window.dispatchEvent(new CustomEvent('atlas:ready', { detail: { splashDuration: performance.now() - startedAt } }));
  }, remaining);
}

function openInstanceEditor(id=null,onlyName=false){
 editInstanceId=id;renameOnly=onlyName;
 const instance=id?state.instances.find(i=>i.id===id):null;
 const form=document.getElementById('instance-form');form.reset();
 document.getElementById('instance-dialog-title').textContent=tr(id?(onlyName?'Переименовать сборку':'Редактировать сборку'):'Новая сборка');
 document.getElementById('form-error').textContent='';
 form.elements.name.value=instance?.name||'';form.elements.description.value=instance?.description||'';
 const colors={amber:'Оранжевый',mint:'Шалфейный',blue:'Синий',violet:'Фиолетовый',rose:'Розовый'};
 document.getElementById('instance-colors').innerHTML=Object.entries(accents).map(([name,color])=>`<label><input type="radio" name="accent" value="${name}" aria-label="${tr(colors[name])}" ${(instance?.accent||'amber')===name?'checked':''}><span class="color-swatch" style="background:${color}"></span></label>`).join('');
 document.getElementById('instance-icons').innerHTML=AtlasInstanceOptions.icons.map(name=>`<label><input type="radio" name="icon" value="${name}" aria-label="${name}" ${(instance?.icon||'box')===name?'checked':''}><span>${icon(name)}</span></label>`).join('');
 for(const element of [form.elements.description.closest('.field'),form.querySelector('.color-field'),form.querySelector('.icon-field')])element.hidden=onlyName;
 const submit=form.querySelector('[type="submit"]');submit.textContent=tr(id?'Сохранить':'Создать сборку');
 hydrateIcons();AtlasI18n.translateTree(document.body,state.settings.language);
 document.getElementById('instance-dialog').showModal();document.getElementById('instance-name').focus();
}
function updatePlaytime(){
 for(const element of document.querySelectorAll('.playtime')){
  const session=gameSessions().find(item=>item.id===element.dataset.sessionId);
  const seconds=Math.max(0,Math.floor((Date.now()-(session?.startedAt||Date.now()))/1000));
  const h=Math.floor(seconds/3600),m=Math.floor(seconds/60)%60,s=seconds%60;
  element.textContent=(h?String(h)+':':'')+String(m).padStart(2,'0')+':'+String(s).padStart(2,'0');
 }
}
setInterval(updatePlaytime,1000);
