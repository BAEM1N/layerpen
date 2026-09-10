import {applyTheme} from './theme.js';
import {icon} from './icons.js';
import {createModelSetup,resolvedAccelerator} from './caption-models.js';
const $=id=>document.getElementById(id);
const native=window.__TAURI__;
const overlay=new URLSearchParams(location.search).has('overlay');
const defaults={whisper:'base',qwen:'Qwen/Qwen3-ASR-0.6B',openai:'gpt-live-transcribe',gemini:'gemini-3.5-transcribe-live',elevenlabs:'scribe_v2_realtime'};
let devices=[],accelerators=[],active=false,clearTimer;
let modelSetup,modelBusy=false,modelReady=false;
let saved={};try{saved=JSON.parse(localStorage.getItem('pointory-caption-settings')||localStorage.getItem('onpen-caption-settings')||localStorage.getItem('layerpen-caption-settings')||'{}');}catch{}
const invoke=(name,args={})=>native.core.invoke(name,args);
function syncControls(){$('config').disabled=active||modelBusy;$('audioConfig').disabled=active||modelBusy;$('saveSettings').disabled=active||modelBusy;$('start').disabled=!native||active||modelBusy||(!overlay&&!modelReady);$('stop').disabled=!active;}
function state(running){active=running;syncControls();modelSetup?.setCaptionActive(running);}
function labelButton(id,name,label){$(id).innerHTML=icon(name);const span=document.createElement('span');span.textContent=label;$(id).append(span);}
function liveLabel(text){$('liveBadge').innerHTML=icon('captions');const span=document.createElement('span');span.textContent=text;$('liveBadge').append(span);}
$('captionBadge').innerHTML=icon('captions');
labelButton('refresh','refresh','장치 새로고침 / Refresh devices');labelButton('saveSettings','save','설정 저장 / Save settings');
labelButton('start','play','자막 시작 / Start');labelButton('stop','stop','중지 / Stop');
liveLabel('LIVE');
function receive(e){
  if(!e?.type)return;
  if(e.type==='partial'||e.type==='final'){
    state(true);
    const target=$(overlay?'captionText':'previewText');
    target.textContent=e.text||'';target.classList.toggle('partial',e.type==='partial');
    if(overlay)target.scrollTop=target.scrollHeight;
    clearTimeout(clearTimer);
    clearTimer=setTimeout(()=>{target.textContent='';},e.type==='final'?8000:15000);
    if(!overlay)$('status').textContent=e.inference_ms!=null?`Live · 추론 ${e.inference_ms} ms · 대기 포함 ${e.lag_ms} ms`:'Live';
  }else if(e.type==='accelerator'){
    $('runtimeDevice').textContent='사용 중 / Active: '+e.text;
    if(overlay)liveLabel('LIVE · '+e.text);
  }else if(e.type==='error'){
    $('error').textContent=e.text;
    if(e.code==='model_required')modelSetup?.refresh();
    if(overlay){$('captionText').textContent='';liveLabel('Error');}
  }else if(e.type==='stopped'){
    state(false);$('status').textContent='Stopped';
    clearTimeout(clearTimer);
    $('previewText').textContent='';
    if(overlay){$('captionText').textContent='';liveLabel('Stopped');}
  }else if(e.type==='status'){
    state(true);$('status').textContent=e.text;
    if(overlay)liveLabel('LIVE · '+e.text);
  }
}
function listDevices(){
  const previous=$('device').value,system=$('source').value==='system';
  $('device').replaceChildren(new Option('시스템 기본 장치 / System default',''));
  for(const d of devices.filter(d=>d.loopback===system))$('device').add(new Option(d.name,d.id));
  if([...$('device').options].some(o=>o.value===previous))$('device').value=previous;
}
function listAccelerators(){
  const old=$('accelerator').value||saved.accelerator||'auto';
  $('accelerator').replaceChildren(new Option('자동 선택 / Auto','auto'));
  const options=accelerators.length?accelerators:[{id:'cpu',name:'CPU',providers:['whisper','qwen']}];
  for(const d of options.filter(d=>d.providers.includes($('provider').value)))$('accelerator').add(new Option(d.name,d.id));
  $('accelerator').value=[...$('accelerator').options].some(o=>o.value===old)?old:'auto';
}
function configValues(){return {provider:$('provider').value,model:$('model').value.trim(),accelerator:$('accelerator').value,language:$('language').value,source:$('source').value,device:$('device').value,threshold:Number($('threshold').value)};}
function saveSettings(){try{saved=configValues();localStorage.setItem('pointory-caption-settings',JSON.stringify(saved));$('savedSettings').textContent=' 저장됨 · API 키는 저장하지 않습니다.';}catch{$('savedSettings').textContent=' 설정을 저장하지 못했습니다.';}}
function providerChanged(){
  const p=$('provider').value,local=['whisper','qwen'].includes(p);
  $('model').value=defaults[p];$('keyRow').hidden=local;$('key').value='';$('sensitivity').hidden=!local;
  $('acceleratorRow').hidden=!local;$('acceleratorHint').hidden=!local;listAccelerators();
  $('models').replaceChildren(...(p==='whisper'?['tiny','base','small','medium','large-v3','large-v3-turbo']:p==='qwen'?['Qwen/Qwen3-ASR-0.6B','Qwen/Qwen3-ASR-1.7B']:p==='openai'?['gpt-live-transcribe','gpt-4o-transcribe','gpt-4o-mini-transcribe']:[defaults[p]]).map(x=>new Option(x,x)));
  $('providerHint').textContent=local?'Whisper base는 균형 잡힌 기본 모델, tiny는 빠르고 가벼운 모델입니다. 아래에서 먼저 다운로드하세요. OpenVINO는 tiny/base/small을 지원하며 첫 실행 때 컴파일합니다. Qwen은 현재 CPU를 사용합니다.':'연속 오디오를 WebSocket으로 전송합니다. 모델 접근 권한과 유효한 API 키가 필요합니다.';
  $('privacy').textContent=local?'오디오를 외부 STT 서버로 보내지 않습니다. 원음·자막은 자동 저장하지 않습니다.':'시작하면 선택한 제공자에게 오디오가 전송되고 API 사용료가 발생할 수 있습니다. 키는 이 실행에서만 사용하며 제공자의 데이터 정책이 적용됩니다.';
  modelSetup?.changed();
}
async function refreshDevices(){
  const results=await Promise.allSettled([invoke('caption_devices'),invoke('caption_hardware')]);
  if(results[0].status==='fulfilled'){devices=results[0].value.devices;listDevices();$('error').textContent='';}
  else $('error').textContent='오디오 장치를 확인하지 못했습니다. 음성 엔진을 준비한 뒤 장치를 새로고침하세요. / Prepare the speech runtime, then refresh audio devices.';
  if(results[1].status==='fulfilled'){accelerators=results[1].value.devices;listAccelerators();}
  if($('accelerator').dataset.saved){$('accelerator').value=[...$('accelerator').options].some(o=>o.value===saved.accelerator)?saved.accelerator:'auto';delete $('accelerator').dataset.saved;}
  if(saved.device&&[...$('device').options].some(o=>o.value===saved.device))$('device').value=saved.device;
  await modelSetup?.refresh();
}
if(overlay){document.body.classList.add('is-overlay');$('settings').hidden=true;$('overlay').hidden=false;}
else{
  if(Object.hasOwn(defaults,saved.provider))$('provider').value=saved.provider;
  providerChanged();
  for(const key of ['model','language','source','threshold'])if(saved[key]!=null)$(key).value=saved[key];
  if(saved.accelerator)$('accelerator').dataset.saved=saved.accelerator;
  modelSetup=createModelSetup({invoke,getConfig:configValues,getDevices:()=>accelerators,onChange:({busy,ready})=>{modelBusy=busy;modelReady=ready;syncControls();}});
  $('saveSettings').onclick=saveSettings;
  $('provider').onchange=providerChanged;$('source').onchange=listDevices;
  $('refresh').onclick=refreshDevices;
  $('model').oninput=()=>modelSetup.changed();$('accelerator').onchange=()=>modelSetup.changed();
  $('form').onsubmit=async e=>{
    e.preventDefault();if(active||modelBusy||!modelSetup.isReady())return;
    $('error').textContent='';$('previewText').textContent='';state(true);
    saveSettings();$('runtimeDevice').textContent='';
    const config={...configValues(),api_key:$('key').value.trim()};
    if(['whisper','qwen'].includes(config.provider))config.accelerator=resolvedAccelerator(config,accelerators);
    try{await invoke('caption_start',{config});$('key').value='';}catch(e){state(false);$('error').textContent=String(e);}
  };
  $('stop').onclick=async()=>{try{await invoke('caption_stop');}catch(e){$('error').textContent=String(e);}};
}
if(native){
  await native.event.listen('session',e=>applyTheme(e.payload.preferences?.theme));applyTheme((await invoke('snapshot')).preferences?.theme);
  await native.event.listen('caption',e=>receive(e.payload));receive(await invoke('caption_snapshot'));
  if(!overlay){
    await native.event.listen('caption-model',async e=>{await modelSetup.receive(e.payload);if(e.payload?.type==='runtime_ready')await refreshDevices();});
    await modelSetup.receive(await invoke('caption_model_snapshot'));await refreshDevices();
  }
}else{$('error').textContent='UI preview only · Launch the desktop app to use audio.';syncControls();}
