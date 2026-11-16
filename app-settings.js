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
  const current = Math.max(1, Math.min(32, Math.round((instance.memoryMb || 4096) / 1024)));
  return `<section class="panel"><div class="panel-heading">${icon('memory-stick')}<h3>Память сборки</h3><span class="tag" data-user-text>${escapeHtml(instance.name)}</span></div>
  <div class="memory-control"><div class="memory-heading"><label for="instance-memory">Лимит RAM</label><output id="memory-value" for="instance-memory" data-user-text>${current} ${tr('ГБ')}</output></div>
  <input type="range" id="instance-memory" name="instance-memory" min="1" max="32" step="1" value="${current}" style="--memory-fill:${(current-1)/31*100}%" aria-valuetext="${current} ${tr('ГБ')}" aria-describedby="memory-note">
  <div class="memory-scale" aria-hidden="true"><span>1 ${tr('ГБ')}</span><span>32 ${tr('ГБ')}</span></div><p id="memory-note" class="subtitle">Применится при следующем запуске.</p></div></section>`;
}
document.addEventListener('input',event=>{
  if(event.target.id!=='instance-memory')return;
  const value=Number(event.target.value);
  event.target.style.setProperty('--memory-fill',`${(value-1)/31*100}%`);
  event.target.setAttribute('aria-valuetext',`${value} ${tr('ГБ')}`);
  document.getElementById('memory-value').value=`${value} ${tr('ГБ')}`;
});
document.addEventListener('change',async event=>{
  const element=event.target;
  if (element.dataset.setting) {
    const value=element.type==='checkbox'?(element.dataset.invert?!element.checked:element.checked):element.value;
    await invoke(()=>window.atlas.saveSettings({[element.dataset.setting]:value}));
  }
  if (element.id==='instance-memory') await invoke(()=>window.atlas.setInstanceMemory(state.selectedId,Number(element.value)*1024));
});
document.addEventListener('click',async event=>{
  const target=event.target.closest('[data-action]'); if(!target)return;
  if(target.dataset.action==='set-accent')await invoke(()=>window.atlas.saveSettings({accentColor:target.dataset.accent}));
});
