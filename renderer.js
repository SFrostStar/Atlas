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
function tabNavigation() {
  return `<div class="home-tabs" role="tablist" aria-label="Разделы сборки">${[['overview','Обзор'],['files','Файлы сборки']].map(([id,name]) => `<button type="button" class="tab ${currentTab===id?'active':''}" role="tab" aria-selected="${currentTab===id}" data-tab="${id}">${name}</button>`).join('')}</div>`;
}
function homePage() {
  const instance = selected();
  const selectedRunning = selectedInstanceRunning();
  const top = intro('Главная', '') + tabNavigation();
  if (currentTab === 'files') return top + filePage();
  return top + `<section class="hero" aria-label="Пейзаж Atlas"><div class="hero-copy"><h2>Твой мир.<br>Твои правила.</h2></div></section>
  <div class="launch-bar"><div class="launch-main">${cube(instance)}<div class="launch-info"><strong>${escapeHtml(instance.name)}</strong><span><span class="status-dot"></span>${selectedRunning ? 'Игра запущена' : installActive()?'Подготовка игры':state.install && !state.install.ready?'Нужна подготовка игры':state.runtimeAvailable ? 'Готово к запуску' : 'Выбери клиент в настройках'}${selectedRunning?` · <span class="playtime" data-session-id="${activeGameSession()?.id}"></span>`:''}</span></div>${selectedRunning ? button('game-menu','Игра запущена','chevron-down','running launch-button',`aria-haspopup="menu" aria-expanded="${contextMenuState?.type==='game'}" ${pendingGameAction?'disabled':''}`) : button('launch',busy?'Запускаем…':installActive()?'Подготовка игры':'Играть','play','primary launch-button',busy||pendingGameAction||installActive()?'disabled':'')}</div></div>
  <div class="quick-grid"><button class="quick-card" data-open="project">${icon('folder-open')}<strong>Корневая папка</strong>${icon('arrow-up-right').replace('<i ','<i class="arrow" ')}</button><button class="quick-card" data-page="workshop">${icon('code-2')}<strong>Исходники</strong>${icon('arrow-up-right').replace('<i ','<i class="arrow" ')}</button><button class="quick-card" data-open="resourcepacks">${icon('package-open')}<strong>Ресурспаки</strong>${icon('arrow-up-right').replace('<i ','<i class="arrow" ')}</button></div>
  `;
}
function instancesPage() {
  return intro('Мои сборки', '', button('new-instance','Новая сборка','plus')) + `<div class="instance-grid">${state.instances.map(instance => `<article class="instance-card" data-instance="${instance.id}" style="--accent:${accent(instance)}"><div class="instance-cover">${icon(AtlasInstanceOptions.icons.includes(instance.icon)?instance.icon:'box')}${instance.id===state.selectedId?'<span class="tag">Выбрана</span>':''}</div><div class="instance-body"><h3>${escapeHtml(instance.name)}</h3>${instance.description?`<p>${escapeHtml(instance.description)}</p>`:''}<div class="instance-card-footer"><small>Atlas 26.3</small><div><button class="bare" data-open="instance" data-id="${instance.id}" aria-label="Открыть папку ${escapeHtml(instance.name)}">${icon('folder-open')}</button>${button('select-instance',instance.id===state.selectedId?'Открыть':'Выбрать','arrow-up-right','secondary',`data-id="${instance.id}"`)}<button class="bare" data-action="instance-menu" data-id="${instance.id}" aria-label="Действия со сборкой ${escapeHtml(instance.name)}" aria-haspopup="menu">${icon('ellipsis-vertical')}</button></div></div></div></article>`).join('')}<button class="instance-new" data-action="new-instance">${icon('plus')}<span>Новая сборка</span></button></div>`;
}
function filePage() {
  const instance = selected();
  const entries = [['instance','folder-open','Папка сборки',instance.path],['saves','mountain','Сохранённые миры','Отдельные сохранения этой сборки'],['resourcepacks','package-open','Ресурспаки','Добавляй пакеты ресурсов в формате .zip'],['logs','file-text','Логи игры','Журнал работы клиента и ошибки запуска']];
  return `<div class="section-heading"><h3>${escapeHtml(instance.name)}</h3></div><div class="file-list">${entries.map(([kind,glyph,title,description])=>`<button class="file-row" data-open="${kind}">${icon(glyph)}<div><strong>${title}</strong>${kind==='instance'?`<p>${escapeHtml(description)}</p>`:''}</div>${icon('arrow-up-right')}</button>`).join('')}</div>`;
}
function workshopPage() {
  return intro('Мастерская', '', button('open-source','Открыть исходники','folder-code','primary')) + `<section class="panel"><div class="panel-heading">${icon('braces')}<h3>Atlas 26.3</h3></div><div class="settings-row"><div><strong>Исходники</strong><p class="path">${escapeHtml(state.sourceRoot)}</p></div><span class="tag">${state.sourceAvailable?'Готово':'Не найдены'}</span></div><div class="settings-row"><div><strong>Клиент</strong><p class="path">${escapeHtml(state.runtimePath)}</p></div>${button('choose-runtime','Выбрать .exe','folder')}</div></section><div class="workshop-actions">${button('open-guide','Инструкция по изменению игры','book-open','secondary')}</div>`;
}
function settingsPage() {
  return intro('Настройки', '') + appearanceSettingsPanel() + memorySettingsPanel() + accountSettingsPanel() + discordSettingsPanel() + `<section class="panel"><div class="panel-heading">${icon('box')}<h3>Клиент и исходники</h3></div><div class="settings-row"><div><strong>Клиент</strong><p class="path">${escapeHtml(state.runtimePath)}</p></div>${button('choose-runtime','Выбрать .exe','folder')}</div><div class="settings-row"><div><strong>Исходники</strong><p class="path">${escapeHtml(state.sourceRoot)}</p></div>${button('choose-source','Выбрать папку','folder-code')}</div><div class="settings-row"><div><strong>Скрывать лаунчер во время игры</strong></div><label class="switch"><input type="checkbox" id="close-on-launch" ${state.settings.closeOnLaunch?'checked':''} aria-label="Скрывать лаунчер во время игры"><span class="switch-track"></span></label></div></section><section class="panel"><div class="panel-heading">${icon('hard-drive')}<h3>Папки</h3></div><div class="settings-row"><div><strong>Корневая папка</strong><p class="path">${escapeHtml(state.projectRoot)}</p></div>${button('open-project','Открыть','arrow-up-right')}</div><div class="settings-row"><div><strong>Сборки и настройки</strong><p class="path">${escapeHtml(state.dataRoot)}</p></div><span class="tag">${state.instances.length}</span></div></section>`;
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
  const editable = focused?.matches('input:not([type="checkbox"]):not([type="radio"])') && document.getElementById('content').contains(focused)
    ? { name: focused.name, value: focused.value, start: focused.selectionStart, end: focused.selectionEnd } : null;
  const contentScroll = document.getElementById('content').scrollTop;
  applyAppearance();
  document.querySelectorAll('.nav-item[data-page]').forEach(element => element.classList.toggle('active', element.dataset.page===currentPage));
  document.getElementById('support-notice').hidden = state.supportVisible===false;
  document.getElementById('instance-count').textContent = state.instances.length;
  document.getElementById('sidebar-instances').innerHTML = state.instances.map(instance=>`<button class="sidebar-instance ${instance.id===state.selectedId?'selected':''}" data-action="select-instance" data-id="${instance.id}">${cube(instance)}<span>${escapeHtml(instance.name)}</span></button>`).join('');
  const renderer = {home:homePage,instances:instancesPage,workshop:workshopPage,settings:settingsPage,logs:logsPage};
  document.getElementById('content').innerHTML = renderer[currentPage]();
  hydrateIcons();
  if (contextMenuState) renderContextMenu();
  renderInstall();
  updatePlaytime();
  AtlasI18n.translateTree(document.body,state.settings.language);
  document.getElementById('content').scrollTop = contentScroll;
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
  document.querySelector('[data-action="game-menu"]')?.setAttribute('aria-expanded','false');
  if (restoreFocus && previous) {
    const selector = previous.type==='game' ? '[data-action="game-menu"]' : `.sidebar-instance[data-id="${CSS.escape(previous.id)}"]`;
    document.querySelector(selector)?.focus();
  }
}
function openContextMenu(type, id, x, y, focus = false) {
  const scrollOffsets=new Map([...document.querySelectorAll('.sidebar-instances, .content'),document].map(element=>[element,element===document?[scrollX,scrollY]:[element.scrollLeft,element.scrollTop]]));
  contextMenuState = {type,id,x,y,scrollOffsets};
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
  if (control.dataset.window) { await invoke(()=>window.atlas.windowControl(control.dataset.window)); return; }
  if (control.dataset.page || control.classList.contains('brand')) {
    currentPage = control.dataset.page || 'home';
    document.getElementById('content').scrollTop = 0;
    render(); return;
  }
  if (control.dataset.tab) { currentTab = control.dataset.tab; render(); return; }
  if (control.dataset.open) { await invoke(()=>window.atlas.openFolder(control.dataset.open,control.dataset.id||state.selectedId)); return; }
  const action = control.dataset.action;
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
  else if (action === 'select-instance') { const next = await invoke(()=>window.atlas.selectInstance(control.dataset.id)); if(next){currentPage='home';currentTab='overview';render();} }
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
