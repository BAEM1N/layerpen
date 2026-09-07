import {languages,loadLanguages,setLanguage,t,localizeDocument} from './i18n.js';
import {activeZoom,sourcePoint,screenPoint,regionView,sourceWidth} from './zoom.js';
import {visibleFading,boardBackground} from './lecture.js';
import {shortcutDefinitions,defaultBindings,keyChord,displayChord,matchShortcut} from './shortcuts.js';
import {exportGif} from './replay.js';
import {pickStroke,translateStroke,strokeBounds,resizeStroke} from './selection.js';
import { normalizedPoint, redraw, exportLayer } from './drawing.js';
import { hits } from './hit-test.js';

const params = new URLSearchParams(location.search);
const view = params.get('view') || 'settings';
document.body.dataset.view = view;
const root = document.querySelector('#app');
const native = window.__TAURI__;
let state;
let localization;
let localeChoice;
let draft = null;
let drawingMonitor = null;
let pointer = null;
let frame = 0;
let committing = false;
let previewState;
let moveDraft=null,selectedIndex=-1,gestureStart=0;
let gifRunning=false,gifCancelled=false;
let zoomRegion=null,zoomImage=null,zoomImageId=null,zoomLoadingId=null;
const inputPoint=e=>sourcePoint(e.clientX,e.clientY,innerWidth,innerHeight,activeZoom(state));
const widths=[2,4,8,16,24];
const widthNames=['아주 가늘게','가늘게','보통','굵게','아주 굵게'];
const defaults={language:'auto',fade_seconds:3,layout:'horizontal',pen_width:4,marker_width:16,eraser_width:24,marker_opacity:0.3,shortcut:'CommandOrControl+Shift+D',capture_dir:'',capture_layer_only:false,gif_speed:1,gif_background:'white',gif_repeat:true,tool_shortcuts:true,keybindings:defaultBindings,global_shortcut_enabled:true,palette:['#8b5cf6','#f43f5e','#fbbf24','#38bdf8','#f8fafc']};
const preferences=()=>({...defaults,...state?.preferences});
const shortcutLabel=value=>value.replace('CommandOrControl',/Mac/.test(navigator.platform)?'⌘':'Ctrl').replace('Shift','⇧').replaceAll('+',' ');
function sizes(name,value){return `<div class="size-choices" data-size-group="${name}">${widths.map((w,i)=>`<button type="button" class="size-choice ${value===w?'selected':''}" data-size="${w}" title="${widthNames[i]}" aria-label="${widthNames[i]}" aria-pressed="${value===w}"><span style="--dot:${Math.max(3,w*.65)}px"></span></button>`).join('')}</div>`;}
const icons = {
  zoom:'<circle cx="10" cy="10" r="7"/><path d="m15 15 7 7M6 10h8m-4-4v8"/>',
  zoomReset:'<circle cx="10" cy="10" r="7"/><path d="m15 15 7 7M6 10h8"/>',
  board:'<rect x="2" y="3" width="20" height="15" rx="2"/><path d="M8 22h8m-4-4v4M5 14l5-5 4 3 5-6"/>',
  fade:'<path d="m14 3 6 6-11 11H3v-6L14 3Z"/><path d="M3 22h2m3 0h2m3 0h1m3 0h.1"/>',
  select:'<path d="M3 3h7M3 3v7m18-7h-7m7 0v7M3 21h7m-7 0v-7m18 7h-7m7 0v-7M8 12h8m-4-4v8"/>',
  gif:'<rect x="2" y="4" width="20" height="16" rx="3"/><path d="m10 8 6 4-6 4Z"/>',
  pen:'<path d="m15 3 6 6-12 12H3v-6L15 3Zm-9 9 6 6M13 5l6 6"/>',
  marker:'<path d="m14 3 7 7-9 9-7-7 9-9ZM5 12l-2 7 2 2 7-2M3 21h7"/>',
  eraser:'<path d="m14 3 7 7-11 11H6l-5-5L14 3Zm-8 8 8 8M10 21h11"/>',
  mouse:'<path d="m5 3 15 10-7 1-4 7L5 3Z"/>',
  settings:'<circle cx="12" cy="12" r="3"/><path d="m9 3-1 3-3 1-2 3 2 2-1 3 3 3 3-1 2 2 3-1 1-3 3-1 1-3-2-2 1-3-3-2-3 1-2-2H9Z"/>',
  undo:'<path d="M8 5 3 10l5 5M3 10h11a6 6 0 0 1 0 12"/>',
  redo:'<path d="m16 5 5 5-5 5m5-5H10a6 6 0 0 0 0 12"/>',
  trash:'<path d="M3 6h18M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7m4-7v7"/>',
  close:'<path d="m6 6 12 12M6 18 18 6"/>',
  monitor:'<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8m-4-4v4"/>',
  line:'<path d="m4 20 16-16"/>',rectangle:'<rect x="3" y="5" width="18" height="14" rx="1"/>',ellipse:'<ellipse cx="12" cy="12" rx="9" ry="7"/>',
  camera:'<path d="M3 6h5l2-3h4l2 3h5v15H3Z"/><circle cx="12" cy="13" r="4"/>',
  eye:'<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z"/><circle cx="12" cy="12" r="3"/>',
  eyeOff:'<path d="m3 3 18 18M10 5c7-1 12 7 12 7a23 23 0 0 1-4 4M6 6a23 23 0 0 0-4 6s4 7 10 7c2 0 3-1 4-1M10 10a3 3 0 0 0 4 4"/>',
};
function icon(name) { return `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icons[name] || ''}</svg>`; }
function error(message) {
  const toast = document.querySelector('#toast'); toast.textContent = message instanceof Error ? message.message : String(message); toast.hidden = false;
  clearTimeout(error.timer); error.timer = setTimeout(() => toast.hidden = true, 7000);
}
async function invoke(command, args = {}) {
  if (native) return native.core.invoke(command, args);
  if (params.get('preview') !== '1') throw new Error('데스크톱 앱으로 실행하세요.');
  // Explicit UI preview only. This never pretends to control native windows.
  if (!previewState) previewState = {
    monitors:[{id:'display-a',name:'DELL U2723QE',width:3840,height:2160,scale:1.5,x:0,y:0},{id:'display-b',name:'LG ULTRAGEAR',width:2560,height:1440,scale:1,x:3840,y:0}],
    selected:'display-b',connected:true,drawing:false,tool:'pen',color:'#8b5cf6',width:4,strokes:[],canUndo:false,canRedo:false,preferences:{...defaults},warning:'브라우저 UI 미리보기 · 실제 화면 오버레이는 데스크톱 앱에서 작동합니다.'
  };
  if(command==='action'&&args.name.startsWith('board')){previewState.board=args.name==='board'?({screen:'white',white:'black',black:'screen'}[previewState.board||'screen']):args.name.slice(6);previewState.drawing=true;previewState.visible=true;}
  if (command === 'snapshot') return structuredClone(previewState);
  if (command === 'select_monitor') previewState.selected = args.id;
  if (command === 'set_tool') Object.assign(previewState,args);
  if (command === 'configure') previewState.preferences={...args.preferences};
  if (command === 'action' && args.name === 'toggle') previewState.drawing = !previewState.drawing;
  if (command === 'action' && args.name === 'toggle' && previewState.drawing) previewState.visible=true;
  if (command === 'action' && args.name === 'toggle_visibility'){previewState.visible=previewState.visible===false;previewState.drawing=false;}
  if (command === 'action' && args.name === 'mouse') previewState.drawing = false;
  if (command === 'add_stroke') { previewState.strokes.push(args.stroke); previewState.canUndo = true; }
  if (command === 'action' && args.name === 'clear') previewState.strokes = [];
  if (command === 'action' && args.name === 'settings') location.href = 'index.html?view=settings&preview=1';
  receive(structuredClone(previewState));
}
async function run(command, args) { try { return await invoke(command,args); } catch(e) { error(e); return null; } }
async function loadZoomImage(id){
 if(!id){zoomImage=null;zoomImageId=null;return;}if(zoomImageId===id||zoomLoadingId===id)return;zoomLoadingId=id;
 try{const png=await invoke('zoom_image',{id});const image=new Image();image.src=png;await image.decode();if(state?.zoom?.id===id){zoomImage=png;zoomImageId=id;schedulePaint();}}catch(e){if(state?.zoom?.id===id){error(e);await run('action',{name:'zoom_reset'});}}finally{if(zoomLoadingId===id)zoomLoadingId=null;}
}
function receive(next) {
  if (draft && (!next.drawing || next.selected !== drawingMonitor || next.tool !== state?.tool)) cancelDraft();
  if(moveDraft&&(!next.drawing||next.selected!==drawingMonitor||next.tool!=='select'))cancelDraft();
  if(!moveDraft&&selectedIndex>=0&&(!next.drawing||next.tool!=='select'||next.selected!==state?.selected||JSON.stringify(next.strokes?.[selectedIndex])!==JSON.stringify(state?.strokes?.[selectedIndex])))selectedIndex=-1;
  if(next.zoom?.id!==state?.zoom?.id&&view==='overlay')cancelDraft();
  state = next;
  const language=next.preferences?.language||'auto';
  if(language!==localeChoice){
    localeChoice=language;setLanguage(language);
    document.title=t(view==='settings'?'LayerPen · 설정':view==='overlay'?'LayerPen · 필기':'LayerPen');
    if(native) native.window.getCurrentWindow().setTitle(document.title).catch(error);
  }
  if(view==='overlay')loadZoomImage(next.zoom?.id);
  if (view === 'overlay') { document.body.classList.toggle('drawing',state.drawing&&state.visible!==false);document.body.classList.toggle('selecting',state.tool==='select'); schedulePaint(); }
  else if (view === 'toolbar') updateToolbar();
  else if (view === 'settings') updateSettings();
  localization?.refresh();
}
function button(name, title, extra = '') { return `<button type="button" title="${title}" aria-label="${title}" ${extra}>${icon(name)}</button>`; }

function mountToolbar() {
  root.innerHTML = `<div class="toolbar">
    <span class="grip" title="도구막대 이동">⠿</span>${button('close','앱 종료','data-action="quit"')}
    <div class="mode-group">${button('mouse','마우스 모드','id="mode"')}${button('eye','판서 숨기기','id="visibility"')}</div>
    <div class="tool-group">${button('pen','펜','data-tool="pen"')}${button('marker','형광펜','data-tool="marker"')}${button('eraser','지우개','data-tool="eraser"')}${button('select','획 선택 / 이동 / 크기 (V)','data-tool="select"')}${button('line','직선','data-tool="line"')}${button('rectangle','사각형','data-tool="rectangle"')}${button('ellipse','원 / 타원','data-tool="ellipse"')}${button('fade','사라지는 잉크','data-tool="fade"')}${button('board','화면 / 화이트 / 블랙보드','id="board"')}${button('zoom','부분 확대','data-tool="zoom"')}${button('zoomReset','확대 종료','id="zoomReset"')}
    <div class="thickness-control"><button id="thickness" aria-label="굵기 선택" title="굵기 선택" aria-expanded="false" aria-controls="thicknessMenu"><span class="thickness-dot"></span><small>⌄</small></button><div id="thicknessMenu" hidden>${sizes('active',4)}</div></div></div>
    <div class="palette">${defaults.palette.map((c,i)=>`<button class="swatch" data-slot="${i}" data-color="${c}" style="--swatch:${c}" title="색상 ${c}" aria-label="색상 ${c}"></button>`).join('')}<label class="custom-color" title="자유색"><input type="color" id="customColor" aria-label="자유색" value="#8b5cf6"></label></div>
    <div class="history-group">${button('undo','실행 취소','data-action="undo" id="undo"')}${button('redo','다시 실행','data-action="redo" id="redo"')}</div>
    <div class="utility-group">${button('trash','현재 화면 필기 전체 지우기','id="clear"')}${button('settings','설정','data-action="settings"')}</div>
    <div class="export-tools">${button('camera','스크린샷 캡처','id="capture"')}${button('gif','GIF 내보내기','id="gifExport"')}</div>
    </div><div class="toolbar-caption"><span id="modeLabel"></span><span id="toolbarShortcut"></span></div>`;
  root.querySelector('.grip').addEventListener('pointerdown', () => { if(native) native.window.getCurrentWindow().startDragging().catch(error); });
  root.querySelector('#mode').onclick = () => run('action',{name:'mouse'});
  root.querySelector('#zoomReset').onclick=()=>run('action',{name:'zoom_reset'});
  root.querySelector('#board').onclick=()=>run('action',{name:'board'});
  root.querySelector('#visibility').onclick=()=>run('action',{name:'toggle_visibility'});
  root.querySelectorAll('[data-action]').forEach(el => el.onclick = () => run('action',{name:el.dataset.action}));
  root.querySelector('#clear').onclick = () => {
    // A second click in three seconds avoids modal webview dialogs over the desktop.
    const el = root.querySelector('#clear');
    if (el.dataset.armed) { delete el.dataset.armed; el.classList.remove('armed'); run('action',{name:'clear'}); }
    else { el.dataset.armed = '1'; el.classList.add('armed'); error('전체 지우기를 한 번 더 누르면 현재 화면의 필기와 GIF 기록이 초기화됩니다.'); setTimeout(()=>{delete el.dataset.armed;el.classList.remove('armed');},3000); }
  };
  root.querySelectorAll('[data-tool]').forEach(el => el.onclick = async () => {
    if (!state) return;
    const p=preferences();
    await run('set_tool',{tool:el.dataset.tool,color:state.color,width:el.dataset.tool === 'eraser' ? p.eraser_width : el.dataset.tool === 'marker' ? p.marker_width : p.pen_width});
    if(!state.drawing) await run('action',{name:'toggle'});
  });
  root.querySelectorAll('[data-color]').forEach(el => el.onclick = () => state && run('set_tool',{tool:state.tool,color:el.dataset.color,width:state.width}));
  root.querySelector('#customColor').oninput=event=>state&&run('set_tool',{tool:state.tool,color:event.target.value,width:state.width});
  const thickness=root.querySelector('#thickness'),menu=root.querySelector('#thicknessMenu');
  const closeSizes=()=>{menu.hidden=true;thickness.setAttribute('aria-expanded','false');};
  thickness.onclick=()=>{
    if(!menu.hidden){closeSizes();return;}
    menu.hidden=false;thickness.setAttribute('aria-expanded','true');
    const r=thickness.getBoundingClientRect();
    menu.style.left=Math.max(4,Math.min(r.x,innerWidth-menu.offsetWidth-4))+'px';
    menu.style.top=Math.max(4,Math.min(r.bottom+4,innerHeight-menu.offsetHeight-4))+'px';
  };
  document.addEventListener('pointerdown',e=>{if(!e.target.closest('.thickness-control'))closeSizes();});
  document.addEventListener('keydown',e=>{if(e.key==='Escape'&&!menu.hidden){e.preventDefault();e.stopImmediatePropagation();closeSizes();thickness.focus();}},true);
  window.addEventListener('blur',closeSizes);window.addEventListener('resize',closeSizes);
  root.querySelectorAll('[data-size]').forEach(el=>el.onclick=async()=>{if(state)await run('set_tool',{tool:state.tool,color:state.color,width:Number(el.dataset.size)});closeSizes();thickness.focus();});
  root.querySelector('#capture').onclick=()=>run('request_capture');
  root.querySelector('#gifExport').onclick=startGif;
}
function updateToolbar() {
  document.body.dataset.layout=preferences().layout;
  root.querySelector('#board').classList.toggle('active',state.board==='white'||state.board==='black');root.querySelector('#board').disabled=state.capturing||!state.connected;root.querySelector('#board').title='보드 전환 · 현재 '+({white:'화이트보드',black:'블랙보드'}[state.board]||'화면');
  root.querySelector('#zoomReset').disabled=!state.zoom||state.capturing;
  root.querySelector('#mode').classList.toggle('active',!state.drawing);
  root.querySelector('#mode').disabled = !state.connected;
  const eye=root.querySelector('#visibility'),visible=state.visible!==false;
  eye.innerHTML=icon(visible?'eye':'eyeOff');eye.title=visible?'판서 숨기기':'판서 보이기';eye.setAttribute('aria-label',eye.title);eye.setAttribute('aria-pressed',String(visible));eye.classList.toggle('active',!visible);eye.disabled=state.capturing||!state.connected;
  root.querySelectorAll('[data-tool]').forEach(el=>{const binding=preferences().keybindings[el.dataset.tool],label=shortcutDefinitions.find(([id])=>id===el.dataset.tool)?.[1]||el.dataset.tool;el.title=label+(preferences().tool_shortcuts&&binding?.enabled?' ('+displayChord(binding.key)+')':'');el.classList.toggle('active',state.drawing && el.dataset.tool===state.tool);el.disabled=!state.connected;});
  root.querySelectorAll('[data-slot]').forEach(el=>{const c=preferences().palette[Number(el.dataset.slot)];el.dataset.color=c;el.style.setProperty('--swatch',c);el.title=t('색상 {value}').replace('{value}',c);el.setAttribute('aria-label',el.title);el.classList.toggle('selected',c.toLowerCase()===state.color.toLowerCase());});
  root.querySelector('.thickness-dot').style.setProperty('--dot',Math.max(3,state.width*.65)+'px');
  root.querySelector('#thickness').disabled=!state.connected;
  root.querySelector('.custom-color').classList.toggle('selected',!preferences().palette.some(c=>c.toLowerCase()===state.color.toLowerCase()));
  root.querySelectorAll('[data-size]').forEach(el=>{el.classList.toggle('selected',Number(el.dataset.size)===state.width);el.setAttribute('aria-pressed',String(Number(el.dataset.size)===state.width));});
  root.querySelector('#customColor').value=state.color;
  root.querySelector('#capture').disabled=state.capturing||!state.connected;
  const gif=root.querySelector('#gifExport');gif.disabled=!gifRunning&&(state.capturing||!state.hasRecording);gif.title=gifRunning?'GIF 내보내기 취소':'GIF 내보내기';
  root.querySelector('#toolbarShortcut').textContent=preferences().global_shortcut_enabled?shortcutLabel(preferences().shortcut):'전역 단축키 꺼짐';
  root.querySelector('#undo').disabled=!state.canUndo; root.querySelector('#redo').disabled=!state.canRedo;
  const monitor = state.monitors.find(m=>m.id===state.selected);
  root.querySelector('#modeLabel').textContent = monitor ? `${!visible?'◌ 판서 숨김':state.tool==='zoom'&&state.drawing?'확대할 영역을 드래그':state.drawing ? '● 필기 중' : '○ 마우스 모드'}${state.zoom?' · x'+state.zoom.scale.toFixed(1):''} · ${monitor.name}` : '모니터 연결 끊김 · 설정에서 선택';
}
function mountSettings() {
  root.innerHTML = `<div class="settings"><header><div class="logo">il<span>•</span></div><div><h1>LayerPen</h1><p>필요한 화면에만, 가볍게.</p></div><span class="version">v0.1.0</span></header>
  <section class="language-setting"><label for="language">언어</label><select id="language">${Object.entries(languages).map(([code,label])=>`<option value="${code}" ${code==='auto'?'':'data-i18n-skip'}>${code==='auto'?'시스템 언어':label}</option>`).join('')}</select><p class="hint">언어는 모든 창에 즉시 적용되며 다음 실행에도 유지됩니다.</p></section>
  <div class="section-heading"><div><span class="eyebrow">WORKSPACE</span><h2>필기할 모니터</h2></div><button id="identify" class="outline">${icon('monitor')} 화면 식별</button></div>
  <p class="description">선택한 화면에만 필기창이 표시됩니다.<br>다른 모니터는 평소처럼 사용할 수 있어요.</p>
  <div id="monitors" role="radiogroup" aria-label="필기할 모니터"></div>
  <div id="connection" class="connection"></div><p id="warning" class="warning" hidden></p>
  <section class="preference-section"><h2>도구막대</h2><div class="setting-row"><span>방향</span><div class="segmented"><button data-layout="horizontal">가로</button><button data-layout="vertical">세로</button></div></div><p class="hint">도구막대의 점 무늬를 잡고 원하는 위치로 이동하세요.</p></section>
  <section class="preference-section"><h2>색상 팔레트</h2><p class="hint">앞의 5칸은 자주 쓰는 색으로 설정하세요. 마지막 칸은 도구막대에서 자유색을 선택합니다.</p><div class="palette-settings">${defaults.palette.map((c,i)=>`<label class="preset-setting"><input type="color" data-preset="${i}" aria-label="기본 색상 ${i+1}" value="${c}"><span>색상 ${i+1}</span></label>`).join('')}<div class="preset-setting picker-info"><span class="picker-sample"></span><span>자유색 · 컬러 피커</span></div></div></section>
  <section class="preference-section"><h2>필기 도구</h2>${[['pen_width','펜 · 도형'],['marker_width','형광펜'],['eraser_width','지우개']].map(([key,label])=>`<div class="setting-row"><span>${label} 기본 굵기</span>${sizes(key,defaults[key])}</div>`).join('')}
  <div class="setting-row"><label for="opacity">형광펜 불투명도</label><div class="range-value"><input id="opacity" type="range" min="10" max="80" step="5"><output id="opacityValue"></output></div></div></section>
  <section class="preference-section"><h2>스크린샷</h2><label class="check-row"><input type="checkbox" id="layerOnly"> 필기 레이어만 캡처</label><p class="hint">기본값은 선택한 화면 + 필기입니다. 레이어만 저장하면 배경이 투명한 PNG가 만들어집니다.</p><label class="field-label" for="captureDir">기본 저장 위치</label><div class="folder-row"><input id="captureDir" readonly aria-label="기본 저장 위치"><button class="outline" id="chooseFolder">폴더 선택</button></div><p class="hint">파일명은 날짜·시간으로 자동 생성됩니다.<br><span id="filenameSample"></span></p><button class="outline" id="captureNow">${icon('camera')} 지금 캡처</button><p class="hint" id="lastCapture" role="status"></p></section>
  <section class="preference-section"><h2>부분 확대 + 판서</h2><p class="hint">돋보기로 영역을 드래그하면 현재 화면을 멈춰 최대 x6까지 확대합니다. 클릭만 하면 x2로 확대합니다. 확대 종료 버튼 또는 마우스 모드로 원래 화면에 돌아갑니다.</p><p class="hint">선택한 굵기는 확대 화면에서 보이는 굵기입니다. x2에서 굵기 8로 그리면 원래 화면에는 굵기 4로 남습니다. PNG는 현재 확대 영역을, GIF는 원래 화면 좌표 전체를 사용합니다. 화면 이동·동영상은 확대 중 갱신되지 않습니다.</p></section>
  <section class="preference-section"><h2>보드와 강조</h2><div class="setting-row"><span>판서 배경</span><div class="segmented"><button data-board="screen">화면</button><button data-board="white">화이트</button><button data-board="black">블랙</button></div></div><p class="hint">배경을 바꿔도 판서는 유지됩니다. 마우스 모드에서는 보드 배경을 내리고, 다시 필기하면 복원합니다.</p><div class="setting-row"><label for="fadeSeconds">강조 잉크 유지 시간</label><select id="fadeSeconds">${[1,2,3,5,10].map(n=>`<option value="${n}">${n}초</option>`).join('')}</select></div><p class="hint">펜을 뗀 뒤 지정한 시간 동안 보이다가 사라집니다. 일반 판서의 선택·실행 취소와 분리하며, GIF에는 강조와 사라지는 과정이 포함됩니다.</p></section>
  <section class="preference-section"><h2>GIF 내보내기</h2><p class="hint">모두 지우기 이후의 필기·이동·크기 변경·삭제를 재생합니다. 모니터별로 기록하며 앱 종료 시 기록은 사라집니다. 긴 변 최대 960px, 기본 10fps. 긴 기록은 프레임 간격을 조정합니다.</p><div class="setting-row"><label for="gifSpeed">재생 속도</label><select id="gifSpeed">${[0.5,1,2,4].map(v=>`<option value="${v}">x${v}</option>`).join('')}</select></div><div class="setting-row"><label for="gifBackground">배경</label><select id="gifBackground"><option value="screen">현재 화면 / 보드 + 판서</option><option value="white">흰색</option><option value="dark">어두운색</option><option value="transparent">투명 (형광펜 반투명 제한)</option></select></div><label class="check-row"><input type="checkbox" id="gifRepeat"> 반복 재생</label><p class="hint">스크린샷과 같은 폴더에 날짜·시간.gif로 저장합니다. 현재 화면 + 판서를 선택하면 내보내기 시점의 화면을 정지 배경으로 사용합니다. 도구막대·기존 판서는 배경 캡처에서 제외합니다. 화면 동영상은 녹화하지 않습니다.</p><button class="outline" id="gifExport">${icon('gif')} GIF 내보내기</button><p class="hint" id="gifProgress" role="status"></p></section>
  <section class="preference-section"><h2>단축키</h2>
  <p class="hint">키 칸을 클릭한 뒤 원하는 키 조합을 누르세요. Escape는 입력 취소입니다. 켜진 항목끼리 같은 키를 사용할 수 없습니다.</p>
  <div class="keymap-row"><span>필기 / 마우스 전환<small>전역 · 다른 앱에서도 작동</small></span><input class="key-input" data-binding-key="global" readonly aria-label="필기 마우스 전환 단축키"><label class="switch"><input type="checkbox" role="switch" data-binding-enabled="global" aria-label="전역 단축키 사용"><span></span></label></div>
  <p class="hint">전역 키는 Ctrl/⌘·Alt 조합 또는 F1–F12를 사용하세요.</p>
  <div class="keymap-row"><span>도구 단축키 전체<small>필기창·도구막대에 포커스가 있을 때</small></span><span></span><label class="switch"><input type="checkbox" role="switch" id="toolShortcuts" aria-label="도구 단축키 전체 사용"><span></span></label></div>
  ${shortcutDefinitions.map(([id,label])=>`<div class="keymap-row"><span>${label}</span><input class="key-input" data-binding-key="${id}" readonly aria-label="${label} 단축키"><label class="switch"><input type="checkbox" role="switch" data-binding-enabled="${id}" aria-label="${label} 단축키 사용"><span></span></label></div>`).join('')}
  <p class="hint">Escape는 언제나 마우스 모드로 돌아갑니다. 크기 조절 중 Shift는 비율 유지에 사용됩니다.</p></section>
  <section class="shortcut-card"><div class="shortcut-icon">${icon('pen')}</div><div><h3>필기와 작업 사이, 단축키 하나</h3><p>필기를 남겨두고 아래 프로그램을 조작하세요.</p></div><kbd id="shortcut"></kbd></section>
  <footer><span class="status-dot"></span><span id="saved">선택한 모니터를 자동으로 기억합니다</span><button id="quit" class="text-button">앱 종료</button></footer></div>`;
  root.querySelector('#identify').onclick=()=>run('identify');
  root.querySelector('#quit').onclick=()=>run('action',{name:'quit'});
  const update=async patch=>{const result=await run('configure',{preferences:{...preferences(),...patch}});if(result!==null)root.querySelector('#saved').textContent='저장됨 · 다음 실행에도 유지됩니다';else updateSettings();};
  root.querySelector('#language').onchange=event=>update({language:event.target.value});
  root.querySelectorAll('[data-preset]').forEach(el=>el.onchange=()=>{const palette=[...preferences().palette];palette[Number(el.dataset.preset)]=el.value;update({palette});});
  root.querySelectorAll('[data-layout]').forEach(el=>el.onclick=()=>update({layout:el.dataset.layout}));
  root.querySelectorAll('[data-size-group]').forEach(group=>group.querySelectorAll('[data-size]').forEach(el=>el.onclick=()=>update({[group.dataset.sizeGroup]:Number(el.dataset.size)})));
  root.querySelector('#fadeSeconds').onchange=e=>update({fade_seconds:Number(e.target.value)});
  root.querySelectorAll('[data-board]').forEach(el=>el.onclick=()=>run('action',{name:'board_'+el.dataset.board}));
  root.querySelector('#gifSpeed').onchange=e=>update({gif_speed:Number(e.target.value)});
  root.querySelector('#gifBackground').onchange=e=>update({gif_background:e.target.value});
  root.querySelector('#gifRepeat').onchange=e=>update({gif_repeat:e.target.checked});
  root.querySelector('#toolShortcuts').onchange=e=>update({tool_shortcuts:e.target.checked});
  root.querySelector('#gifExport').onclick=startGif;
  root.querySelector('#opacity').oninput=e=>root.querySelector('#opacityValue').textContent=e.target.value+'%';
  root.querySelector('#opacity').onchange=e=>update({marker_opacity:Number(e.target.value)/100});
  root.querySelector('#layerOnly').onchange=e=>update({capture_layer_only:e.target.checked});
  const editBinding=(id,patch)=>id==='global'?update({...(patch.key?{shortcut:patch.key.replace('Mod','CommandOrControl')}:{global_shortcut_enabled:patch.enabled})}):update({keybindings:{...preferences().keybindings,[id]:{...preferences().keybindings[id],...patch}}});
  root.querySelectorAll('[data-binding-key]').forEach(el=>{
    const current=()=>el.dataset.bindingKey==='global'?preferences().shortcut:preferences().keybindings[el.dataset.bindingKey].key;
    el.onfocus=()=>{el.value=t('키를 누르세요…');};el.onblur=()=>{el.value=displayChord(current());};
    el.onkeydown=async event=>{event.preventDefault();event.stopPropagation();if(event.key==='Escape'){el.blur();return;}const key=keyChord(event);if(!key)return;el.blur();await editBinding(el.dataset.bindingKey,{key});};
  });
  root.querySelectorAll('[data-binding-enabled]').forEach(el=>el.onchange=()=>editBinding(el.dataset.bindingEnabled,{enabled:el.checked}));
  root.querySelector('#chooseFolder').onclick=async()=>{const folder=await run('choose_capture_folder',{title:t('스크린샷 저장 폴더')});if(folder)await update({capture_dir:folder});};
  root.querySelector('#captureNow').onclick=()=>run('request_capture');
  root.querySelector('#filenameSample').textContent=new Date().toLocaleDateString('sv-SE')+'_'+new Date().toTimeString().slice(0,8).replaceAll(':','-')+'.png';
}
let monitorSignature = '';
function updateSettings() {
  const p=preferences();
  root.querySelector('#language').value=p.language;
  root.querySelector('#fadeSeconds').value=p.fade_seconds;root.querySelectorAll('[data-board]').forEach(el=>{el.classList.toggle('active',el.dataset.board===(state.board||'screen'));el.disabled=state.capturing||!state.connected;});
  root.querySelectorAll('[data-preset]').forEach(el=>{el.value=p.palette[Number(el.dataset.preset)];});
  root.querySelectorAll('[data-layout]').forEach(el=>{el.classList.toggle('active',el.dataset.layout===p.layout);el.setAttribute('aria-pressed',String(el.dataset.layout===p.layout));});
  root.querySelectorAll('[data-size-group]').forEach(group=>group.querySelectorAll('[data-size]').forEach(el=>{const chosen=Number(el.dataset.size)===p[group.dataset.sizeGroup];el.classList.toggle('selected',chosen);el.setAttribute('aria-pressed',String(chosen));}));
  if(document.activeElement!==root.querySelector('#opacity'))root.querySelector('#opacity').value=Math.round(p.marker_opacity*100);
  root.querySelector('#opacityValue').textContent=Math.round(p.marker_opacity*100)+'%';
  root.querySelector('#layerOnly').checked=p.capture_layer_only;
  root.querySelector('#gifSpeed').value=p.gif_speed;root.querySelector('#gifBackground').value=p.gif_background;root.querySelector('#gifRepeat').checked=p.gif_repeat;root.querySelector('#toolShortcuts').checked=p.tool_shortcuts;
  root.querySelector('#gifExport').disabled=!gifRunning&&(state.capturing||!state.hasRecording);
  root.querySelector('#captureDir').value=p.capture_dir;
  root.querySelectorAll('[data-binding-key]').forEach(el=>{const id=el.dataset.bindingKey,enabled=id==='global'?p.global_shortcut_enabled:p.keybindings[id].enabled;el.disabled=!enabled;if(document.activeElement!==el)el.value=displayChord(id==='global'?p.shortcut:p.keybindings[id].key);});
  root.querySelectorAll('[data-binding-enabled]').forEach(el=>{el.checked=el.dataset.bindingEnabled==='global'?p.global_shortcut_enabled:p.keybindings[el.dataset.bindingEnabled].enabled;});
  root.querySelector('#shortcut').textContent=shortcutLabel(p.shortcut);
  root.querySelector('#captureNow').disabled=state.capturing||!state.connected;
  const signature = JSON.stringify([state.monitors,state.selected]);
  if(signature !== monitorSignature) {
    monitorSignature=signature;
    const list=root.querySelector('#monitors'); list.replaceChildren();
    state.monitors.forEach((m,i)=>{
      const selected=m.id===state.selected;
      const el=document.createElement('button');el.type='button';el.className=`monitor-card ${selected?'chosen':''}`;
      el.setAttribute('role','radio');el.setAttribute('aria-checked',String(selected));
      el.innerHTML=`<span class="display-picture">${icon('monitor')}<span>${i+1}</span></span><span class="monitor-copy"><strong></strong><small></small></span><span class="selection-dot">${selected?'✓':''}</span>`;
      el.querySelector('strong').textContent=m.name;
      el.querySelector('small').textContent=`${m.width} × ${m.height} · ${Math.round(m.scale*100)}%`;
      el.onclick=async()=>{ el.disabled=true; const result=await run('select_monitor',{id:m.id}); el.disabled=false; if(result!==null)root.querySelector('#saved').textContent='저장됨 · 다음 실행에도 이 화면을 사용합니다'; };
      list.append(el);
    });
    if(!state.monitors.length)list.textContent='연결된 모니터를 찾지 못했습니다.';
  }
  root.querySelector('#connection').textContent=state.connected?'✓  선택한 모니터에서만 필기합니다':'선택한 모니터가 연결되어 있지 않아 필기를 중지했습니다. 다른 화면을 선택하세요.';
  root.querySelector('#connection').classList.toggle('missing',!state.connected);
  root.querySelector('#warning').hidden=!state.warning;root.querySelector('#warning').textContent=state.warning||'';
}
function schedulePaint() {
 if(frame)return;frame=requestAnimationFrame(()=>{
  frame=0;let strokes=state?.strokes||[];if(moveDraft){strokes=[...strokes];strokes[moveDraft.index]=moveDraft.after;}
  const canvas=root.querySelector('canvas'),fading=visibleFading(state?.fading),view=activeZoom(state);canvas.style.backgroundColor=boardBackground(state);
  canvas.style.backgroundImage=view&&zoomImageId===view.id&&state?.visible!==false?`url("${zoomImage}")`:'none';canvas.style.backgroundSize=view?`${innerWidth*view.scale}px ${innerHeight*view.scale}px`:'';canvas.style.backgroundPosition=view?`${-view.x*innerWidth*view.scale}px ${-view.y*innerHeight*view.scale}px`:'';
  redraw(canvas,state?.visible===false?[]:[...strokes,...fading],state?.visible===false?null:draft,devicePixelRatio,view);
  const region=root.querySelector('.zoom-region');if(region){region.hidden=!zoomRegion;if(zoomRegion)Object.assign(region.style,{left:Math.min(zoomRegion.start.x,zoomRegion.end.x)*innerWidth+'px',top:Math.min(zoomRegion.start.y,zoomRegion.end.y)*innerHeight+'px',width:Math.abs(zoomRegion.end.x-zoomRegion.start.x)*innerWidth+'px',height:Math.abs(zoomRegion.end.y-zoomRegion.start.y)*innerHeight+'px'});}
  const box=root.querySelector('.selection-box'),selected=strokes[selectedIndex];
  if(box){box.hidden=!selected||!state?.drawing||state?.tool!=='select'||state.visible===false;
   if(!box.hidden){const xs=selected.points.map(p=>screenPoint(p,innerWidth,innerHeight,view).x),ys=selected.points.map(p=>screenPoint(p,innerWidth,innerHeight,view).y),minX=xs.reduce((a,b)=>Math.min(a,b),Infinity),maxX=xs.reduce((a,b)=>Math.max(a,b),0),minY=ys.reduce((a,b)=>Math.min(a,b),Infinity),maxY=ys.reduce((a,b)=>Math.max(a,b),0),pad=selected.width*(view?.scale||1)/2+5;Object.assign(box.style,{left:Math.max(6,minX-pad)+'px',top:Math.max(6,minY-pad)+'px',width:Math.max(1,Math.min(innerWidth-6,maxX+pad)-Math.max(6,minX-pad))+'px',height:Math.max(1,Math.min(innerHeight-6,maxY+pad)-Math.max(6,minY-pad))+'px'});}
  }
  if(state?.visible!==false&&fading.length)schedulePaint();
 });
}
function cancelDraft() { zoomRegion=null;draft=null;moveDraft=null;selectedIndex=-1;pointer=null;drawingMonitor=null;if(view==='overlay')schedulePaint(); }
function updateTransform(move,event){
 const point=inputPoint(event),dx=point.x-move.start.x,dy=point.y-move.start.y;
 if(move.handle){const b=strokeBounds(move.before);move.target={x:(move.handle.endsWith('e')?b.maxX:b.minX)+dx,y:(move.handle.startsWith('s')?b.maxY:b.minY)+dy};move.after=resizeStroke(move.before,move.handle,move.target.x,move.target.y,event.shiftKey,innerWidth,innerHeight);}
 else move.after=translateStroke(move.before,dx,dy);
}
function mountOverlay() {
  root.innerHTML='<canvas aria-label="화면 필기 영역"></canvas><div class="drawing-edge"></div><div class="zoom-region" hidden></div><div class="selection-box" hidden><span class="resize-handle" data-handle="nw" title="크기 조절 · Shift: 비율 유지"></span><span class="resize-handle" data-handle="ne" title="크기 조절 · Shift: 비율 유지"></span><span class="resize-handle" data-handle="sw" title="크기 조절 · Shift: 비율 유지"></span><span class="resize-handle" data-handle="se" title="크기 조절 · Shift: 비율 유지"></span></div>';
  const canvas=root.querySelector('canvas');
  root.onpointerdown=event=>{
    if(!state?.drawing||pointer!==null||committing||event.button!==0)return;
    if(activeZoom(state)&&zoomImageId!==state.zoom.id)return;
    pointer=event.pointerId;drawingMonitor=state.selected;gestureStart=performance.now();
    if(state.tool==='zoom'){const point=normalizedPoint(event.clientX,event.clientY,innerWidth,innerHeight);zoomRegion={start:point,end:point};canvas.setPointerCapture(pointer);schedulePaint();return;}
    if(state.tool==='select'){
      const point=inputPoint(event),handle=event.target.dataset.handle;
      if(!handle)selectedIndex=pickStroke(state.strokes||[],point,innerWidth,innerHeight,activeZoom(state)?.scale||1);
      if(selectedIndex>=0){const before=state.strokes[selectedIndex];moveDraft={index:selectedIndex,before,after:before,start:point,handle};canvas.setPointerCapture(pointer);}else{pointer=null;drawingMonitor=null;}
      schedulePaint();return;
    }
    draft={times:[0],tool:state.tool,color:state.color,width:sourceWidth(state.width,activeZoom(state)),opacity:state.tool==='marker'?preferences().marker_opacity:1,points:[inputPoint(event)]};
    canvas.setPointerCapture(pointer);schedulePaint();
  };
  root.onpointermove=event=>{
    if(event.pointerId!==pointer)return;
    if(zoomRegion){zoomRegion.end=normalizedPoint(event.clientX,event.clientY,innerWidth,innerHeight);schedulePaint();return;}
    if(moveDraft){updateTransform(moveDraft,event);schedulePaint();return;}
    if(!draft)return;
    const events=event.getCoalescedEvents?.()||[event];
    for(const p of events.length?events:[event]) {
      if(draft.points.length>=100000)break;
      const point=inputPoint(p);
      const time=Math.max(draft.times.at(-1),Math.round(performance.now()-gestureStart));
      if(['line','rectangle','ellipse'].includes(draft.tool)){draft.points[1]=point;draft.times[1]=time;}else{draft.points.push(point);draft.times.push(time);}
    }
    schedulePaint();
  };
  root.onpointerup=async event=>{
    if(event.pointerId!==pointer)return;
    if(zoomRegion){const region=zoomRegion,monitor=drawingMonitor;region.end=normalizedPoint(event.clientX,event.clientY,innerWidth,innerHeight);cancelDraft();committing=true;try{await invoke('start_zoom',{monitor,...regionView(region.start,region.end,innerWidth,innerHeight)});}catch(e){error(e);}finally{committing=false;receive(await invoke('snapshot'));}return;}
    if(moveDraft){
      const move=moveDraft,monitor=drawingMonitor;
      updateTransform(move,event);
      const dx=move.after.points[0].x-move.before.points[0].x,dy=move.after.points[0].y-move.before.points[0].y;
      moveDraft=null;pointer=null;drawingMonitor=null;committing=true;
      try{const duration=Math.round(performance.now()-gestureStart);if(move.handle&&(Math.abs(inputPoint(event).x-move.start.x)*innerWidth+Math.abs(inputPoint(event).y-move.start.y)*innerHeight)*(activeZoom(state)?.scale||1)<.5){}else if(move.handle)await invoke('resize_stroke',{monitor,index:move.index,handle:move.handle,x:move.target.x,y:move.target.y,uniform:event.shiftKey,duration});else await invoke('move_stroke',{monitor,index:move.index,dx,dy,duration});}catch(e){error(e);}finally{committing=false;receive(await invoke('snapshot'));if(state.selected===monitor&&state.drawing&&state.tool==='select'&&state.strokes?.[move.index])selectedIndex=move.index;schedulePaint();}return;
    }
    if(!draft)return;
    const end=inputPoint(event),time=Math.round(performance.now()-gestureStart);
    if(['line','rectangle','ellipse'].includes(draft.tool)){draft.points[1]=end;draft.times[1]=time;}
    else if(draft.points.length<100000){draft.points.push(end);draft.times.push(time);}
    const stroke=draft,monitor=drawingMonitor;pointer=null;draft=null;drawingMonitor=null;committing=true;
    // Paint a local optimistic copy until the authoritative Rust snapshot arrives.
    if(stroke.tool==='fade'){state.fading=[...(state.fading||[]),{stroke,born:Date.now(),life:preferences().fade_seconds*1000}];}else state.strokes=stroke.tool==='eraser'?(state.strokes||[]).filter(s=>!hits(s,stroke,innerWidth,innerHeight)):[...(state.strokes||[]),stroke];schedulePaint();
    try { await invoke('add_stroke',{monitor,stroke}); } catch(e) { error(e); }
    finally { committing=false;const next=await run('snapshot');if(next)receive(next); }
  };
  root.onpointercancel=cancelDraft;
  window.addEventListener('blur',()=>{if(pointer!==null)cancelDraft();});
  window.addEventListener('resize',schedulePaint);
  canvas.oncontextmenu=e=>e.preventDefault();
}
async function chooseTool(tool){if(!state||state.capturing)return;const p=preferences();await run('set_tool',{tool,color:state.color,width:tool==='eraser'?p.eraser_width:tool==='marker'?p.marker_width:p.pen_width});if(!state.drawing)await run('action',{name:'toggle'});}
let clearArmed=0;
function clearRecording(){if(Date.now()<clearArmed){clearArmed=0;cancelDraft();run('action',{name:'clear'});}else{clearArmed=Date.now()+3000;error('같은 단축키를 한 번 더 누르면 필기와 GIF 기록을 초기화합니다.');}}
async function startGif(){
 if(gifRunning){gifCancelled=true;return;}if(!state||state.capturing)return;
 if(!native){error('GIF 저장은 데스크톱 앱에서 사용할 수 있습니다.');return;}
 gifRunning=true;gifCancelled=false;const initial=structuredClone(state);initial.preferences=preferences();
 const report=text=>{if(view==='settings')root.querySelector('#gifProgress').textContent=text;else{const el=root.querySelector('#gifExport');el.title=t('{value} · 클릭하면 취소').replace('{value}',t(text));root.querySelector('#modeLabel').textContent=text;}};
 try{const path=await exportGif({invoke,state:initial,onProgress:p=>report(`GIF 저장 ${p}%`),cancelled:()=>gifCancelled});report('저장됨: '+path);error('GIF 저장 완료');}catch(e){report(e instanceof Error?e.message:String(e));error(e);}finally{gifRunning=false;receive(await invoke('snapshot'));}
}
document.addEventListener('keydown',event=>{
 if(event.target.closest('input,select,textarea')||view==='settings'||event.repeat)return;
 if(event.key==='Escape'){event.preventDefault();cancelDraft();run('action',{name:'mouse'});return;}
 if(!preferences().tool_shortcuts||state?.capturing)return;
 const action=matchShortcut(event,preferences().keybindings);if(!action)return;event.preventDefault();
 if(['pen','marker','fade','eraser','select','line','rectangle','ellipse','zoom'].includes(action)){cancelDraft();chooseTool(action);}
 else if(action==='mouse'){cancelDraft();run('action',{name:'mouse'});}
 else if(action==='visibility'){cancelDraft();run('action',{name:'toggle_visibility'});}
 else if(action==='undo'||action==='redo'){cancelDraft();run('action',{name:action});}
 else if(action==='zoom_reset'){cancelDraft();run('action',{name:'zoom_reset'});}
 else if(action==='board'){cancelDraft();run('action',{name:'board'});}
 else if(action==='capture')run('request_capture');
 else if(action==='gif')startGif();
 else if(action==='clear')clearRecording();
 else if(action.startsWith('color'))run('set_tool',{tool:state.tool,color:preferences().palette[Number(action.at(-1))-1],width:state.width});
 else if(action==='thinner'||action==='thicker'){const i=widths.indexOf(state.width),width=widths[Math.max(0,Math.min(4,(i<0?1:i)+(action==='thicker'?1:-1)))];run('set_tool',{tool:state.tool,color:state.color,width});}

});
async function start() {
  await loadLanguages();
  localization=localizeDocument();
  if(view==='identify'){
    if(native){const snapshot=await invoke('snapshot');setLanguage(snapshot.preferences?.language||'auto');await native.window.getCurrentWindow().setTitle(t('LayerPen · 화면 식별'));}
    root.innerHTML='<div class="identify-number"></div>';root.firstChild.textContent=params.get('number')||'1';return;
  }
  if(!native&&params.get('preview')!=='1') {root.innerHTML='<div class="launch-message"><h1>LayerPen</h1><p>이 화면은 데스크톱 앱에서 실행해야 합니다.</p><p>README의 실행 방법을 확인하세요.</p></div>';return;}
  if(view==='toolbar')mountToolbar();else if(view==='overlay')mountOverlay();else mountSettings();
  if(native){
    await native.event.listen('session',event=>receive(event.payload));
    await native.event.listen('capture-saved',event=>{if(view==='toolbar')error('캡처 저장됨');if(view==='settings')root.querySelector('#lastCapture').textContent=t('저장됨: {value}').replace('{value}',event.payload);});
    await native.event.listen('capture-error',event=>error(event.payload));
    if(view==='overlay')await native.event.listen('capture-request',async()=>{
      try{cancelDraft();receive(await invoke('snapshot'));const canvas=root.querySelector('canvas');redraw(canvas,state.visible===false?[]:[...(state.strokes||[]),...visibleFading(state.fading)],null,devicePixelRatio,activeZoom(state));await invoke('save_capture',{monitor:state.selected,png:exportLayer(canvas,[...(state.strokes||[]),...visibleFading(state.fading)],activeZoom(state))});}catch(e){await run('cancel_capture',{message:String(e)});}
    });
  }
  const next=await invoke('snapshot');receive(next);
}
start().catch(error);
