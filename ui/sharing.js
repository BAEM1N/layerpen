const $=id=>document.getElementById(id);
const invoke=window.__TAURI__?.core?.invoke;
let current={running:false,files:[]};
async function refresh(){
 current=await invoke('share_status');
 $('status').textContent=current.running?'공유 중 · Sharing on':'꺼짐 · Off';
 $('toggle').textContent=current.running?'공유 중지 · Stop sharing':'공유 시작 · Start sharing';
 $('address').disabled=current.running;if(!$('address').value)$('address').value=current.address;
 $('connection').hidden=!current.running;$('url').value=current.url;
 $('liveToggle').disabled=!current.running;$('liveToggle').textContent=current.live?'화면 공유 중지 · Stop live view':'화면 공유 시작 · Start live view';$('liveStatus').textContent=current.live_error||(current.live?'화면 전송 중 · Broadcasting screen':'꺼짐 · Off');
 $('qr').replaceChildren();if(current.qr){const img=new Image();img.alt='수강생 접속 QR · Student access QR';img.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(current.qr);$('qr').append(img);}
 $('files').replaceChildren();for(const file of current.files){const li=document.createElement('li'),name=document.createElement('span'),remove=document.createElement('button');name.textContent=`${file.name} · ${(file.size/1048576).toFixed(1)} MB`;remove.textContent='공유 목록에서 제거 · Remove';remove.onclick=()=>act(()=>invoke('share_remove',{id:file.id}));li.append(name,remove);$('files').append(li);}
 if(!current.files.length){const li=document.createElement('li');li.textContent='아직 자료가 없습니다 · No files selected';$('files').append(li);}
}
async function act(fn){$('error').textContent='';$('toggle').disabled=true;try{await fn();await refresh();}catch(e){$('error').textContent=String(e);}finally{$('toggle').disabled=false;}}
$('add').onclick=()=>act(()=>invoke('share_add'));
$('liveToggle').onclick=()=>act(()=>invoke('share_live',{enabled:!current.live}));
$('toggle').onclick=()=>act(()=>invoke(current.running?'share_stop':'share_start',current.running?{}:{address:$('address').value.trim()}));
$('copy').onclick=async()=>{try{await navigator.clipboard.writeText(current.url);$('status').textContent='주소 복사됨 · URL copied';}catch{$('url').focus();$('url').select();$('status').textContent='Ctrl+C / ⌘C로 복사 · Copy the selected URL';}};
if(invoke){act(async()=>{});}else{$('status').textContent='브라우저 미리보기 · Browser preview';$('toggle').disabled=true;$('add').disabled=true;}
if(invoke)setInterval(()=>refresh().catch(e=>{$('error').textContent=String(e);}),3000);
