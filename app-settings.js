'use strict';
const launcherAccents = { orange: ['#ff8a3d','#ffa363','Оранжевый'], blue: ['#70a7ff','#97bfff','Синий'], green: ['#55c69a','#7eddb6','Зелёный'], violet: ['#b298f9','#c8b5ff','Фиолетовый'], rose: ['#f28cb0','#ffb0cb','Розовый'] };
const tr = text => AtlasI18n.t(text,state?.settings?.language || 'ru');
function applyAppearance() {
  const settings = state.settings;
  const root = document.documentElement;
  root.lang = settings.language;
  root.dataset.theme = settings.theme;
  root.dataset.hero = settings.heroStyle;
  root.dataset.heroText = String(settings.showHeroText);
  root.dataset.reducedMotion = String(settings.reducedMotion);
  const colors = launcherAccents[settings.accentColor] || launcherAccents.orange;
  root.style.setProperty('--orange', colors[0]);
  root.style.setProperty('--orange-hover', colors[1]);
}
function settingsSelect(key, values, labels) {
  return `<select data-setting="${key}" aria-label="${escapeHtml(labels.title)}">${values.map((value,index)=>`<option value="${value}" ${state.settings[key]===value?'selected':''}>${escapeHtml(labels[index])}</option>`).join('')}</select>`;
}
function appearanceSettingsPanel() {
  return `<section class="panel"><div class="panel-heading">${icon('palette')}<h3>Оформление</h3></div>
  <div class="settings-row"><strong>Язык лаунчера</strong>${settingsSelect('language',['ru','en'],{0:'Русский',1:'English',title:'Язык лаунчера'})}</div>
  <div class="settings-row"><strong>Тема</strong>${settingsSelect('theme',['dark','light'],{0:'Тёмная',1:'Светлая',title:'Тема'})}</div>
  <div class="settings-row"><strong>Акцентный цвет</strong><div class="accent-options" role="group" aria-label="Акцентный цвет">${Object.entries(launcherAccents).map(([id,[color,,name]])=>`<button type="button" class="accent-choice" style="--swatch:${color}" data-action="set-accent" data-accent="${id}" aria-label="${name}" aria-pressed="${state.settings.accentColor===id}">${state.settings.accentColor===id?icon('check'):''}</button>`).join('')}</div></div></section>
  <section class="panel"><div class="panel-heading">${icon('sliders-horizontal')}<h3>Персонализация</h3></div>
  <div class="settings-row"><strong>Фон главной</strong>${settingsSelect('heroStyle',['landscape','sunset','minimal'],{0:'Пейзаж',1:'Закат',2:'Минимализм',title:'Фон главной'})}</div>
  ${appearanceToggle('showHeroText','Текст на главной',state.settings.showHeroText)}
  ${appearanceToggle('reducedMotion','Анимации интерфейса',!state.settings.reducedMotion,true)}</section>`;
}
function appearanceToggle(key,label,checked,invert=false) {
  return `<div class="settings-row"><strong>${label}</strong><label class="switch"><input type="checkbox" data-setting="${key}" ${invert?'data-invert="true"':''} ${checked?'checked':''} aria-label="${label}"><span class="switch-track"></span></label></div>`;
}
function memorySettingsPanel() {
  const instance = selected();
  const maximum = state.memory.maximumMb;
  const current = instance.memoryMb || 0;
  const steps = [...new Set([0,2048,4096,6144,8192,12288,16384,24576,32768,current].filter(n=>n===0||n<=maximum))].sort((a,b)=>a-b);
  return `<section class="panel"><div class="panel-heading">${icon('memory-stick')}<h3>Память сборки</h3><span class="tag" data-user-text>${escapeHtml(instance.name)}</span></div>
  <div class="settings-row"><div><strong>Лимит RAM</strong><p>При следующем запуске. При достижении лимита игре может не хватить памяти.</p></div><select id="instance-memory" aria-label="Лимит RAM">${steps.map(n=>`<option value="${n}" ${current===n?'selected':''}>${n===0?'Без ограничения':`${n/1024} ${tr('ГБ')}`}</option>`).join('')}</select></div></section>`;
}
function accountSettingsPanel() {
  const account = state.accounts;
  if (!account) return '';
  const signingIn = ['starting','waiting','verifying'].includes(account.status);
  const signinLabel = account.status==='verifying'?'Проверяем аккаунт…':signingIn?'Вход…':'Войти через Microsoft';
  return `<section class="panel"><div class="panel-heading">${icon('circle-user-round')}<h3>Аккаунты</h3></div>
  ${account.accounts.map(item=>`<div class="settings-row account-row"><div><strong data-user-text>${escapeHtml(item.name)}</strong><p>${item.type==='local'?'Локальный профиль':'Microsoft'}</p></div><div class="row-actions">${button('select-account',account.selectedId===item.id?'Выбран':'Выбрать',account.selectedId===item.id?'check':'user-round','secondary',`data-id="${escapeHtml(item.id)}" ${account.selectedId===item.id?'disabled':''}`)}${button('remove-account','Удалить','x','quiet',`data-id="${escapeHtml(item.id)}"`)}</div></div>`).join('')}
  <form id="local-account-form" class="account-add"><input name="name" aria-label="Ник игрока" placeholder="Ник игрока" pattern="[A-Za-z0-9_]{3,16}" minlength="3" maxlength="16" required autocomplete="off"><button type="submit" class="button secondary">${icon('plus')}Добавить</button></form>
  <div class="account-signin">${button('signin-microsoft',signinLabel,'log-in','secondary',!account.configured||!account.storageAvailable||signingIn?'disabled':'')}${!account.configured?'<p class="subtitle">Для Microsoft-входа нужен Client ID приложения Atlas.</p>':!account.storageAvailable?'<p class="subtitle">Защищённое хранилище Windows недоступно.</p>':''}${signingIn&&!account.device?button('cancel-signin','Отменить вход','x','quiet'):''}</div>
  ${account.device?`<div class="signin-device"><span>Код входа</span><strong data-user-text>${escapeHtml(account.device.userCode)}</strong>${button('open-microsoft','Открыть Microsoft','external-link')}${button('cancel-signin','Отменить вход','x','quiet')}</div>`:''}
  ${account.error?`<p class="form-error">${escapeHtml(AtlasI18n.accountError(account.error,state.settings.language))}</p>`:''}<p class="subtitle">Игра использует выбранный профиль при следующем запуске.</p></section>`;
}
document.addEventListener('change',async event=>{
  const element=event.target;
  if (element.dataset.setting) {
    const value=element.type==='checkbox'?(element.dataset.invert?!element.checked:element.checked):element.value;
    await invoke(()=>window.atlas.saveSettings({[element.dataset.setting]:value}));
  }
  if (element.id==='instance-memory') await invoke(()=>window.atlas.setInstanceMemory(state.selectedId,Number(element.value)));
});
document.addEventListener('click',async event=>{
  const target=event.target.closest('[data-action]'); if(!target)return;
  const action=target.dataset.action;
  if(action==='set-accent')await invoke(()=>window.atlas.saveSettings({accentColor:target.dataset.accent}));
  if(action==='select-account')await invoke(()=>window.atlas.selectAccount(target.dataset.id));
  if(action==='remove-account')await invoke(()=>window.atlas.removeAccount(target.dataset.id));
  if(action==='signin-microsoft')await invoke(()=>window.atlas.startSignIn());
  if(action==='cancel-signin')await invoke(()=>window.atlas.cancelSignIn());
  if(action==='open-microsoft')await invoke(()=>window.atlas.openMicrosoft());
});
document.addEventListener('submit',async event=>{
  if(event.target.id!=='local-account-form')return;
  event.preventDefault();
  const name=new FormData(event.target).get('name');
  await invoke(()=>window.atlas.addLocalAccount(name));
});
