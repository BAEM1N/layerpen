import {ensureFonts} from './fonts.js';
import {textStroke,fontSizeForPen,canvasFont} from './text.js';
import {activeZoom,sourcePoint} from './zoom.js';

export function createTextEditor({root,getState,invoke,error,refresh}){
  let editor=null;
  function cancel(){if(editor){const old=editor;editor=null;old.element.remove();}}
  async function finish(save=true){
    if(!editor)return;
    const old=editor,content=old.input.value;cancel();
    if(!save||!content.trim())return;
    const stroke=textStroke(root.querySelector('canvas').getContext('2d'),{...old.options,content});
    try{await invoke('add_stroke',{monitor:old.monitor,stroke});await refresh();}catch(e){error(e);}
  }
  async function begin(event){
    const x=event.clientX,y=event.clientY;
    await finish();
    try{await ensureFonts();}catch(e){error(e);return;}
    const state=getState();if(!state?.drawing||state.tool!=='text')return;
    const zoom=activeZoom(state),size=fontSizeForPen(state.width),font=state.preferences?.text_font||'Malgun Gothic';
    const left=Math.max(1,Math.min(x,innerWidth-size-6)),top=Math.max(1,Math.min(y,innerHeight-size*1.25-35));
    const element=document.createElement('div');element.className='text-editor';
    const input=document.createElement('textarea');input.className='text-input';input.maxLength=4096;input.setAttribute('aria-label','텍스트 입력');input.spellcheck=false;
    Object.assign(element.style,{left:left+'px',top:top+'px',maxWidth:(innerWidth-left-3)+'px'});
    Object.assign(input.style,{fontFamily:JSON.stringify(font)+', sans-serif',fontSize:size+'px',color:state.color,caretColor:state.color});
    const actions=document.createElement('div');actions.className='text-editor-actions';
    const done=document.createElement('button');done.textContent='완료';done.type='button';done.onclick=()=>finish();
    const discard=document.createElement('button');discard.textContent='취소';discard.type='button';discard.onclick=cancel;
    const hint=document.createElement('span');hint.textContent='Enter 완료 · Shift+Enter 줄바꿈 · Esc 취소';
    actions.append(done,discard,hint);element.append(input,actions);root.append(element);
    editor={element,input,monitor:state.selected,zoom:zoom?.id,options:{font,width:state.width,color:state.color,point:sourcePoint(left,top,innerWidth,innerHeight,zoom),canvasWidth:innerWidth,canvasHeight:innerHeight,scale:zoom?.scale||1}};
    element.onpointerdown=e=>e.stopPropagation();
    input.onkeydown=e=>{
      e.stopPropagation();if(e.isComposing||e.keyCode===229)return;
      if(e.key==='Escape'){e.preventDefault();cancel();}
      else if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();finish();}
    };
    input.oninput=()=>{
      const ctx=root.querySelector('canvas').getContext('2d');ctx.save();ctx.font=canvasFont(font,size);
      const measured=Math.max(size*5,...input.value.split('\n').map(line=>ctx.measureText(line).width));ctx.restore();
      input.style.width=Math.min(innerWidth-left-4,measured+8)+'px';
      input.style.height='0px';input.style.height=Math.min(innerHeight-top-32,Math.max(size*1.25+6,input.scrollHeight))+'px';
    };
    input.oninput();input.focus();
  }
  function sync(next){if(editor&&(!next.drawing||next.tool!=='text'||next.selected!==editor.monitor||next.zoom?.id!==editor.zoom))cancel();}
  return {begin,finish,cancel,sync};
}
