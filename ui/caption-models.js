import {icon} from './icons.js';

const $=id=>document.getElementById(id);
const localProvider=provider=>['whisper','qwen'].includes(provider);
export function resolvedAccelerator(config, devices) {
  if(config.provider==='qwen')return 'cpu';
  if(config.accelerator!=='auto')return config.accelerator;
  const available=devices.filter(device=>device.providers.includes(config.provider));
  for(const prefix of ['cuda','openvino:GPU','openvino:NPU','cpu']) {
    const match=available.find(device=>device.id.startsWith(prefix));
    if(match)return match.id;
  }
  return 'cpu';
}
export function formatBytes(value) {
  if(!Number.isFinite(value)||value<0)return '확인 중 / Checking';
  return value>=1e9?(value/1e9).toFixed(2)+' GB':(value/1e6).toFixed(1)+' MB';
}
const messages={
  runtime_missing:'음성 엔진이 필요합니다. Python이 없다면 Python 3.10 이상을 설치한 뒤 다시 시도하세요. / Speech runtime required; install Python 3.10+ if missing.',
  python_version:'Python 3.10 이상이 필요합니다. / Python 3.10+ is required.',
  runtime_setup_failed:'음성 엔진 설치에 실패했습니다. 네트워크와 저장 공간을 확인하고 다시 시도하세요. / Runtime setup failed; check network and disk space, then retry.',
  runtime_override:'별도로 지정한 Python 환경을 사용 중입니다. 그 환경에 음성 패키지를 설치하거나 STT_PYTHON 지정을 해제하세요. / Prepare the configured Python environment or unset the STT_PYTHON override.',
  model_required:'선택한 모델을 먼저 다운로드하세요. / Download this model first.',
  unsupported_model:'목록의 모델을 고르거나 준비된 로컬 모델 폴더를 입력하세요. / Choose a listed model or a prepared local model folder.',
  model_unsupported:'목록의 모델을 고르거나 준비된 로컬 폴더를 입력하세요. OpenVINO는 tiny/base/small을 지원합니다. / Choose a listed model or a prepared folder; OpenVINO supports tiny/base/small.',
  download_failed:'다운로드에 실패했습니다. 연결을 확인한 뒤 다시 시도하면 받은 파일을 재사용합니다. / Download failed; retry to reuse completed files.',
  busy:'다른 음성 작업이 진행 중입니다. / Another speech task is running.',
};
const phases={
  starting:'준비 중 / Preparing',
  runtime_create:'음성 엔진 공간 준비 중 / Preparing speech runtime',
  runtime_setup:'음성 엔진 준비 중 / Preparing speech runtime',
  runtime_bootstrap:'음성 엔진 설치 도구 준비 중 / Preparing installer',
  runtime_install:'음성 엔진 다운로드·설치 중 / Installing speech runtime',
  runtime_verify:'음성 엔진 확인 중 / Checking speech runtime',
  downloading:'모델 다운로드 중 / Downloading model',
  download:'모델 다운로드 중 / Downloading model',
  verifying:'받은 모델 확인 중 / Verifying model',
};

export function createModelSetup({invoke,getConfig,getDevices,onChange}) {
  let info=null,busy=false,query=0,timer,captionActive=false;
  const config=()=>{const value=getConfig();return localProvider(value.provider)?{provider:value.provider,model:value.model,accelerator:value.accelerator,resolvedAccelerator:resolvedAccelerator(value,getDevices())}:{provider:'whisper',model:'base',accelerator:'cpu',resolvedAccelerator:'cpu'};};
  function label(id,name,text){$(id).innerHTML=icon(name);const span=document.createElement('span');span.textContent=text;$(id).append(span);}
  label('prepareRuntime','download','음성 엔진 설치 / Prepare runtime');
  label('downloadModel','download','모델 다운로드 / Download model');
  label('cancelModel','close','취소 / Cancel');
  label('modelRefresh','refresh','상태 확인 / Check');
  function controls(){
    const local=localProvider(getConfig().provider);$('modelSetup').hidden=!local&&!busy&&!!info?.runtimeReady;
    $('modelHeading').textContent=local?'로컬 모델 준비 / Local model setup':'음성 엔진 준비 / Speech runtime setup';
    document.querySelectorAll('[data-model-only]').forEach(element=>element.hidden=!local);
    $('modelSetupHint').textContent=local?'첫 사용은 음성 엔진과 모델을 한 번 내려받습니다. 준비 중에는 마이크를 사용하지 않습니다. / Prepare the runtime and model once; setup does not use your microphone.':'API 자막은 음성 엔진만 설치하면 됩니다. 로컬 모델 파일은 받지 않습니다. / API captions need the speech runtime, without a local model download.';
    $('prepareRuntime').hidden=!!info?.runtimeReady;
    $('prepareRuntime').disabled=busy||captionActive||!info||!!info.code;
    $('downloadModel').hidden=!local||!!info?.ready;
    $('downloadModel').disabled=!local||busy||captionActive||!info?.runtimeReady||info?.managed===false||info?.downloadReady===false;
    $('cancelModel').hidden=!busy;$('cancelModel').disabled=!busy;
    $('modelRefresh').disabled=busy||captionActive;
    onChange({busy,ready:!!(info?.runtimeReady&&(!local||info?.ready))});
  }
  function showInfo(value){
    info=value;
    $('modelStatus').textContent=value.code?messages[value.code]||value.text||value.code:!value.runtimeReady?'음성 엔진을 먼저 준비하세요. / Prepare the speech runtime first.':value.ready?'준비 완료 · 인터넷 없이 사용 가능 / Ready for offline use':'아직 받지 않은 모델입니다. / Model has not been downloaded.';
    $('modelSize').textContent=value.downloadBytes!=null?formatBytes(value.downloadBytes):'—';
    $('modelEngine').textContent=value.accelerator||value.engine||'—';
    $('modelPath').textContent=value.path||'모델 다운로드 후 표시 / Shown after download';
    const source=value.source||value.sourceUrl;
    const url=typeof source==='string'&&/^https:\/\/huggingface\.co\//.test(source)?source:null;
    $('modelSource').hidden=!url;if(url)$('modelSource').href=url;
    const license=typeof value.license==='string'?value.license:value.license?.name;
    $('modelLicense').textContent=license?`모델 라이선스 / License: ${license}`:'';
    controls();
  }
  async function refresh(){
    clearTimeout(timer);const revision=++query;
    if(busy)return;
    info=null;$('modelStatus').textContent='준비 상태 확인 중 / Checking model';controls();
    try{const value=await invoke('caption_model_status',{config:config()});if(revision!==query||busy)return;showInfo(value);}
    catch(error){if(revision!==query||busy)return;showInfo({code:'runtime_missing',text:String(error),ready:false,runtimeReady:false});}
  }
  function changed(){
    ++query;info=null;controls();clearTimeout(timer);
    if(!busy)timer=setTimeout(refresh,250);
  }
  async function receive(event){
    if(!event?.type)return;
    if(event.active===true&&['model_ready','runtime_ready','model_error','model_status'].includes(event.type)){
      ++query;busy=true;$('modelProgressArea').hidden=false;$('modelProgress').removeAttribute('value');
      $('modelStatus').textContent=event.type==='model_error'?(messages[event.code]||event.text):'준비 결과 확인 중 / Finishing preparation';controls();return;
    }
    if(event.type==='model_progress'){
      ++query;busy=true;$('modelProgressArea').hidden=false;
      const phase=phases[event.phase]||'모델 다운로드 중 / Downloading model';
      $('modelStatus').textContent=phase;
      const completed=Number(event.completedBytes),total=Number(event.totalBytes);
      // A completed-file counter must not imply byte-level progress inside a large file.
      if(event.byteProgress===true&&total>0&&completed>=0){$('modelProgress').max=total;$('modelProgress').value=completed;}
      else $('modelProgress').removeAttribute('value');
      $('modelProgressText').textContent=event.completedBytes!=null&&event.totalBytes!=null?`${formatBytes(completed)} / ${formatBytes(total)} · 완료한 파일 기준 / completed files`:(event.model||'');
      controls();return;
    }
    if(['model_ready','runtime_ready','model_error'].includes(event.type)||event.status==='cancelled'){
      ++query;busy=false;$('modelProgressArea').hidden=true;controls();
      if(event.type==='model_error'){
        $('modelStatus').textContent=messages[event.code]||event.text||'준비에 실패했습니다. / Preparation failed.';
        return;
      }
      if(event.status==='cancelled'){
        await refresh();$('modelStatus').textContent='취소됨 · 다시 다운로드하면 받은 파일을 재사용합니다. / Cancelled; retry to reuse completed files.';
      }else await refresh();
    }
  }
  async function start(command){
    if(busy||captionActive)return;
    ++query;busy=true;controls();$('modelProgressArea').hidden=false;$('modelProgress').removeAttribute('value');
    $('modelStatus').textContent='준비 중 / Preparing';$('modelProgressText').textContent='';
    try {const result=await invoke(command,{config:config()});if(result?.type==='model_error')await receive(result);else await receive(await invoke('caption_model_snapshot'));}
    catch(error){await receive({type:'model_error',text:String(error)});}
  }
  $('prepareRuntime').onclick=()=>start('caption_runtime_setup');
  $('downloadModel').onclick=()=>start('caption_model_download');
  $('modelRefresh').onclick=refresh;
  $('cancelModel').onclick=async()=>{ $('cancelModel').disabled=true;try{const event=await invoke('caption_model_cancel');if(event?.type)await receive(event);}catch(error){$('modelStatus').textContent=String(error);$('cancelModel').disabled=false;} };
  return {changed,refresh,receive,config,isReady:()=>!!(info?.runtimeReady&&(!localProvider(getConfig().provider)||info?.ready)),get busy(){return busy;},setCaptionActive(value){captionActive=value;controls();}};
}
