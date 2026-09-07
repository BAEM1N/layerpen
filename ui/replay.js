import {fadeOpacity} from './lecture.js';
import {paintStroke} from './drawing.js';
export function normalizeRecording(events){
 let end=0;return events.map(e=>{const at=Math.max(end,e.at||0),duration=(e.kind==='draw'||e.kind==='fade')?(e.stroke.times?.at(-1)||0):e.kind==='move'?e.duration:0;end=at+duration;return {...e,at,duration};});
}
export function partialStroke(stroke,elapsed){
 const times=stroke.times||[];if(!times.length||elapsed>=times.at(-1))return stroke;
 if(['line','rectangle','ellipse'].includes(stroke.tool)){
  const first=stroke.points[0],last=stroke.points.at(-1),t=Math.min(1,elapsed/Math.max(1,times.at(-1)));
  return {...stroke,points:[first,{x:first.x+(last.x-first.x)*t,y:first.y+(last.y-first.y)*t}]};
 }
 let i=0;while(i<times.length&&times[i]<=elapsed)i++;
 const points=stroke.points.slice(0,Math.max(1,i));
 if(i>0&&i<times.length){const a=stroke.points[i-1],b=stroke.points[i],t=(elapsed-times[i-1])/Math.max(1,times[i]-times[i-1]);points.push({x:a.x+(b.x-a.x)*t,y:a.y+(b.y-a.y)*t});}
 return {...stroke,points};
}
export function sceneAt(events,time){
 let strokes=[],fading=[];
 for(const e of events){
  if(e.at>time)break;
  if(e.kind==='fade'){const elapsed=time-e.at-e.duration,opacity=fadeOpacity(Math.max(0,elapsed),e.life);if(opacity>0)fading.push({...partialStroke(e.stroke,time-e.at),opacity});}
  else if(e.kind==='draw')strokes.push(partialStroke(e.stroke,time-e.at));
  else if(e.kind==='replace')strokes=[...e.strokes];
  else if(e.kind==='move'&&strokes[e.index]){
   const t=Math.min(1,(time-e.at)/Math.max(1,e.duration));
   strokes[e.index]={...e.before,width:e.before.width+(e.after.width-e.before.width)*t,points:e.before.points.map((p,i)=>({x:p.x+(e.after.points[i].x-p.x)*t,y:p.y+(e.after.points[i].y-p.y)*t}))};
  }
 }
 return [...strokes,...fading];
}
export async function exportGif({invoke,state,onProgress,cancelled}){
 const m=state.monitors.find(m=>m.id===state.selected);if(!m)throw Error('모니터를 선택하세요.');
 const logicalW=m.width/m.scale,logicalH=m.height/m.scale,scale=Math.min(1,960/Math.max(logicalW,logicalH));
 const canvas=document.createElement('canvas');canvas.width=Math.max(1,Math.round(logicalW*scale));canvas.height=Math.max(1,Math.round(logicalH*scale));
 let started=false;
 try{
  const recording=await invoke('begin_gif',{width:canvas.width,height:canvas.height});started=true;
  const events=normalizeRecording(recording.events);
  let background=null;if(recording.background){background=new Image();background.src=recording.background;await background.decode();}
  const duration=events.reduce((end,e)=>Math.max(end,e.at+e.duration+(e.kind==='fade'?e.life:0)),0)/state.preferences.gif_speed;
  if(duration>600000)throw Error('GIF 재생 시간이 10분을 넘습니다. 재생 속도를 높이거나 모두 지우기로 새 기록을 시작하세요.');
  const interval=Math.max(100,Math.ceil(duration/1200/10)*10),ctx=canvas.getContext('2d'),speed=state.preferences.gif_speed;
  const times=[];for(let t=0;t<duration;t+=interval)times.push(t);times.push(duration);
  for(let i=0;i<times.length;i++){
   if(cancelled())throw Error('GIF 내보내기를 취소했습니다.');
   ctx.setTransform(1,0,0,1,0,0);ctx.clearRect(0,0,canvas.width,canvas.height);
   if(background){ctx.drawImage(background,0,0,canvas.width,canvas.height);}
   else if(state.preferences.gif_background!=='transparent'){ctx.fillStyle=state.preferences.gif_background==='dark'?'#171717':'#ffffff';ctx.fillRect(0,0,canvas.width,canvas.height);}
   ctx.setTransform(canvas.width/logicalW,0,0,canvas.height/logicalH,0,0);
   for(const s of sceneAt(events,times[i]*speed))paintStroke(ctx,s,logicalW,logicalH);
   const delay=i===times.length-1?100:Math.max(2,Math.round((times[i+1]-times[i])/10));
   await invoke('gif_frame',{png:canvas.toDataURL('image/png'),delay});onProgress(Math.round((i+1)/times.length*100));
  }
  if(cancelled())throw Error('GIF 내보내기를 취소했습니다.');
  return await invoke('finish_gif');
 }catch(e){if(started)await invoke('abort_gif').catch(()=>{});throw e;}
}
