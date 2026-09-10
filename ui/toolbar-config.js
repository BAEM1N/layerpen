// Keep one live button per action so moving it preserves native event handlers.
export const toolbarItems = [
  ['visibility','판서 숨기기','#visibility'],
  ['mouse','마우스 모드','#mode'],
  ['pen','펜','[data-tool="pen"]'],
  ['marker','형광펜','[data-tool="marker"]'],
  ['text','텍스트','[data-tool="text"]'],
  ['eraser','지우개','[data-tool="eraser"]'],
  ['shapes','도형과 추가 도구','#shapesToggle'],
  ['style','색상과 굵기','#styleToggle'],
  ['undo','실행 취소','#undo'],
  ['redo','다시 실행','#redo'],
  ['spotlight','스포트라이트 · Spotlight','#spotlightToggle'],
  ['captions','실시간 자막','#captionOpen'],
  ['capture','스크린샷 캡처','#capture'],
  ['board','화면 / 화이트 / 블랙보드','#board'],
  ['zoom_reset','확대 종료','#zoomReset'],
  ['gif','GIF 내보내기','#gifExport'],
  ['share','자료 공유','#shareOpen'],
  ['clear','현재 화면 필기 전체 지우기','#clear'],
];
export const defaultToolbarItems = toolbarItems.slice(0,14).map(([id])=>id);
const knownItems = new Set(toolbarItems.map(([id])=>id));
export function normalizeToolbarItems(items) {
  return Array.isArray(items) ? [...new Set(items.filter(id=>knownItems.has(id)))] : [...defaultToolbarItems];
}
export function moveToolbarItem(items, id, direction) {
  const next=normalizeToolbarItems(items), from=next.indexOf(id), to=from+direction;
  if(from>=0 && to>=0 && to<next.length && (direction===-1 || direction===1)) [next[from],next[to]]=[next[to],next[from]];
  return next;
}
const toolbarMounts = new WeakMap();
export function applyToolbarConfig(root, items) {
  const rail=root.querySelector('.toolbar');
  if(!rail) return false;
  let mounted=toolbarMounts.get(root);
  if(mounted?.rail!==rail) {
    const primary=rail.querySelector('.drawing-tools'), system=rail.querySelector('.toolbar-system');
    const more=root.querySelector('#morePanel .panel-tools');
    const buttons=new Map(toolbarItems.map(([id,,selector])=>[id,root.querySelector(selector)]));
    if(!primary || !system || !more || [...buttons.values()].some(button=>!button)) throw new Error('Toolbar controls are incomplete');
    const moreToggle=rail.querySelector('[data-panel="morePanel"]');
    system.prepend(moreToggle);
    primary.classList.add('toolbar-custom-tools');
    primary.setAttribute('aria-label','도구 바로가기');
    mounted={rail,primary,system,more,buttons,key:null};
    toolbarMounts.set(root,mounted);
  }
  const normalized=normalizeToolbarItems(items), key=JSON.stringify(normalized), changed=mounted.key!==key;
  const selected=new Set(normalized);
  if(changed) {
    normalized.forEach(id=>mounted.primary.append(mounted.buttons.get(id)));
    toolbarItems.filter(([id])=>!selected.has(id)).forEach(([id])=>mounted.more.append(mounted.buttons.get(id)));
    rail.querySelectorAll('.toolbar-group').forEach(group=>{
      if(group!==mounted.primary && group!==mounted.system) group.remove();
    });
    let empty=mounted.more.querySelector('.toolbar-more-empty');
    if(!empty) {empty=root.ownerDocument.createElement('p');empty.className='hint toolbar-more-empty';empty.textContent='모든 도구가 도구막대에 표시되어 있습니다.';mounted.more.append(empty);}
    empty.hidden=selected.size!==toolbarItems.length;
    mounted.key=key;
  }
  // State updates replace some button contents (visibility and shape icons).
  for(const [id,label] of toolbarItems) {
    const button=mounted.buttons.get(id);
    button.dataset.toolbarItem=id;
    let caption=button.querySelector(':scope > .toolbar-item-label');
    if(!caption) {
      caption=id==='style' ? null : button.querySelector(':scope > span');
      if(!caption) {caption=root.ownerDocument.createElement('span');button.append(caption);}
      caption.classList.add('toolbar-item-label');
    }
    caption.textContent=id==='visibility' ? (button.getAttribute('aria-label') || label) : label;
  }
  return changed;
}

// A panel trigger inside the closed More panel has no on-screen bounds.
export function toolbarPanelAnchor(root, panelId) {
  return root.querySelector(`.toolbar [aria-controls="${panelId}"]`) || root.querySelector('.toolbar [aria-controls="morePanel"]');
}

export function toolbarSettingsMarkup() {
  return `<details class="toolbar-config"><summary>바로가기 표시와 순서</summary><p class="hint">체크한 도구는 도구막대에 표시됩니다. 화살표로 순서를 바꾸세요. 숨긴 도구는 더보기에서 사용할 수 있습니다.</p><fieldset class="toolbar-config-controls"><legend class="visually-hidden">도구 바로가기</legend><div class="toolbar-config-list"></div><button type="button" class="outline" data-toolbar-reset>기본 배치로 복원</button></fieldset><p class="hint">더보기, 방향 전환, 설정, 앱 종료는 항상 표시됩니다.</p></details>`;
}
const settingsMounts=new WeakMap();
const escape=value=>String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
export function updateToolbarSettings(container, items) {
  const list=container.querySelector('.toolbar-config-list');
  if(!list) return;
  const selected=normalizeToolbarItems(items), signature=JSON.stringify(selected);
  if(list.dataset.selection===signature) return;
  const focused=container.ownerDocument.activeElement;
  const restore=focused?.closest('.toolbar-config') ? focused.dataset.toolbarControl : null;
  const labels=new Map(toolbarItems.map(([id,label])=>[id,label]));
  const ordered=[...selected,...toolbarItems.map(([id])=>id).filter(id=>!selected.includes(id))];
  list.innerHTML=ordered.map(id=>{
    const index=selected.indexOf(id), on=index>=0;
    return `<div class="toolbar-config-row${on?' selected':''}" data-toolbar-row="${id}" role="group" aria-label="${escape(labels.get(id))}"><label><input type="checkbox" data-toolbar-toggle="${id}" data-toolbar-control="${id}:toggle"${on?' checked':''}><span>${escape(labels.get(id))}</span></label><div class="toolbar-reorder"><button type="button" data-toolbar-move="${id}" data-direction="-1" data-toolbar-control="${id}:up" title="앞으로 이동" aria-label="앞으로 이동"${!on||index===0?' disabled':''}>↑</button><button type="button" data-toolbar-move="${id}" data-direction="1" data-toolbar-control="${id}:down" title="뒤로 이동" aria-label="뒤로 이동"${!on||index===selected.length-1?' disabled':''}>↓</button></div></div>`;
  }).join('');
  list.dataset.selection=signature;
  if(restore) container.querySelector(`[data-toolbar-control="${restore}"]`)?.focus({preventScroll:true});
}
export function bindToolbarSettings(container, getItems, onChange) {
  if(settingsMounts.has(container)) return;
  const control=container.querySelector('.toolbar-config-controls');
  let busy=false;
  const save=async items=>{
    if(busy) return;
    const focusKey=container.ownerDocument.activeElement?.dataset.toolbarControl;
    busy=true;control.disabled=true;
    try {await onChange(items);} finally {
      busy=false;control.disabled=false;
      updateToolbarSettings(container,getItems());
      if(focusKey) {
        const previous=container.querySelector(`[data-toolbar-control="${focusKey}"]`);
        const target=previous?.disabled ? container.querySelector(`[data-toolbar-control="${focusKey.split(':')[0]}:toggle"]`) : previous;
        target?.focus({preventScroll:true});
      }
    }
  };
  container.addEventListener('change',event=>{
    const input=event.target.closest('[data-toolbar-toggle]');
    if(!input) return;
    const items=normalizeToolbarItems(getItems()), id=input.dataset.toolbarToggle;
    void save(input.checked ? [...items.filter(item=>item!==id),id] : items.filter(item=>item!==id));
  });
  container.addEventListener('click',event=>{
    const reset=event.target.closest('[data-toolbar-reset]');
    if(reset) {void save([...defaultToolbarItems]);return;}
    const button=event.target.closest('[data-toolbar-move]');
    if(button && !button.disabled) void save(moveToolbarItem(getItems(),button.dataset.toolbarMove,Number(button.dataset.direction)));
  });
  settingsMounts.set(container,true);
  updateToolbarSettings(container,getItems());
}
