// Opt-in native fixture suite. Python helpers use no audio, network, or installers.
(async()=>{
 if(window.__POINTORY_VALIDATION_RUNNING)return;window.__POINTORY_VALIDATION_RUNNING=true;
 const api=window.__TAURI__,call=(name,args={})=>api.core.invoke(name,args);
 const report={suite:'models',platform:navigator.platform,userAgent:navigator.userAgent,started:new Date().toISOString(),checks:[],errors:[]};
 const pending=new Map(),events=[];let sequence=0;
 const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
 const assert=(value,message)=>{if(!value)throw Error(message);};
 const wait=async(test,message)=>{for(let i=0;i<150;i++){const value=await test();if(value)return value;await sleep(100);}throw Error(message);};
 const run=async(name,fn)=>{await api.event.emit('pointory-validation-progress',name);const started=performance.now();try{const detail=await fn();report.checks.push({name,status:'passed',durationMs:Math.round(performance.now()-started),detail});}catch(e){report.checks.push({name,status:'failed',durationMs:Math.round(performance.now()-started),error:String(e)});await call('caption_model_cancel').catch(()=>{});await call('caption_stop').catch(()=>{});}};
 const rejects=async(fn,pattern)=>{try{await fn();}catch(error){assert(pattern.test(String(error)),'Unexpected rejection: '+error);return String(error);}throw Error('Expected command to reject');};
 const config=(model='base',extra={})=>({provider:'whisper',model,accelerator:'cpu',...extra});
 const snapshot=()=>call('caption_model_snapshot');
 const active=async phase=>{const value=await snapshot();return value.active===true&&(!phase||value.phase===phase)?value:null;};
 const done=async type=>{const value=await snapshot();return value.active===false&&value.type===type?value:null;};
 const remote=(label,script)=>new Promise((resolve,reject)=>{
  const id=String(++sequence),timer=setTimeout(()=>{pending.delete(id);reject(Error('WebView timeout: '+label));},10000);
  pending.set(id,{resolve:value=>{clearTimeout(timer);resolve(value);},reject:error=>{clearTimeout(timer);reject(error);}});
  api.event.emit('pointory-validation-eval',{id,label,script}).catch(error=>{const p=pending.get(id);pending.delete(id);p?.reject(error);});
 });
 const uiError=event=>report.errors.push(String(event.reason||event.error||event.message||'Uncaught WebView error'));
 window.addEventListener('error',uiError);window.addEventListener('unhandledrejection',uiError);
 try{
  await api.event.listen('pointory-validation-response',event=>{const p=pending.get(event.payload.id);if(!p)return;pending.delete(event.payload.id);event.payload.error?p.reject(Error(event.payload.error)):p.resolve(event.payload.data);});
  await api.event.listen('pointory-validation-ui-error',event=>report.errors.push(event.payload));
  await api.event.listen('caption-model',event=>events.push(event.payload));
  await wait(()=>document.querySelector('#orientationToggle'),'Toolbar did not load');
  report.initial=await call('snapshot');
  await call('configure',{preferences:{...report.initial.preferences,global_shortcut_enabled:false}});
  await run('native-offline-status-and-sanitized-request',async()=>{
   const value=await call('caption_model_status',{config:config('base',{api_key:'fixture-key-must-be-removed',source:'microphone',device:'fixture-device',runtimeDir:'C:/must-not-use',engine:'qwen'})});
   assert(value.type==='model_status'&&!value.ready&&value.runtimeReady,'Status did not use the fixture');
   assert(value.fixtureRequest.apiKeyReceived===false,'Model status received an API key');
   assert(JSON.stringify(value.fixtureRequest.receivedKeys)===JSON.stringify(['accelerator','command','model','provider']),'Model status received unrelated settings');
   assert(!(await snapshot()).active,'Status started a background download');return value;
  });
  await run('download-progress-snapshot-and-exclusive-work',async()=>{
   const started=await call('caption_model_download',{config:config()});assert(started.active,'Download did not start');
   const progress=await wait(()=>active('downloading'),'Native model progress missing');
   const second=await rejects(()=>call('caption_model_download',{config:config('tiny')}),/already running/i);
   const runtime=await rejects(()=>call('caption_runtime_setup',{config:config()}),/already running/i);
   const caption=await rejects(()=>call('caption_start',{config:config('base',{source:'microphone'})}),/preparation/i);
   await call('caption_stop');assert((await snapshot()).active,'Stopping captions also stopped the model download');
   return {progress,second,runtime,caption,captionStopPreservedDownload:true};
  });
  await run('caption-settings-close-and-reopen-preserves-download',async()=>{
   assert((await snapshot()).active,'Expected ongoing download');await call('caption_open');await sleep(400);
   await wait(()=>remote('caption-settings',"return !!document.querySelector('#cancelModel');"),'Caption settings did not load');
   const before=await remote('caption-settings',"return {visible:await window.__TAURI__.window.getCurrentWindow().isVisible(),busy:!document.querySelector('#cancelModel').hidden,startDisabled:document.querySelector('#start').disabled};");
   assert(before.visible&&before.busy&&before.startDisabled,'Opened settings did not restore active progress');
   await remote('caption-settings',"await window.__TAURI__.window.getCurrentWindow().close();return true;");
   await wait(()=>remote('caption-settings',"return !(await window.__TAURI__.window.getCurrentWindow().isVisible());"),'CloseRequested did not hide caption settings');
   assert((await snapshot()).active,'Closing settings cancelled the download');
   await call('caption_open');
   const after=await wait(async()=>{const value=await remote('caption-settings',"return {visible:await window.__TAURI__.window.getCurrentWindow().isVisible(),busy:!document.querySelector('#cancelModel').hidden,startDisabled:document.querySelector('#start').disabled};");return value.visible&&value.busy?value:null;},'Reopened settings lost active progress');
   return {before,after,progress:await snapshot()};
  });
  await run('cancel-kills-download-and-allows-replacement',async()=>{
   const before=await snapshot(),cancelled=await call('caption_model_cancel');assert(cancelled.status==='cancelled'&&!cancelled.active,'Cancel did not detach worker');
   await call('caption_model_download',{config:config('tiny')});const ready=await wait(()=>done('model_ready'),'Replacement download did not finish');
   assert(ready.ready&&ready.model==='tiny','Wrong replacement completed');await sleep(350);
   assert((await snapshot()).type==='model_ready'&&(await snapshot()).model==='tiny','A stale cancelled event replaced the new result');
   return {cancelledWorker:before.fixturePid,cancelledChild:before.fixtureChildPid,ready};
  });
  await run('oversized-worker-protocol-is-terminated',async()=>{
   await call('caption_model_download',{config:config('small')});const error=await wait(()=>done('model_error'),'Oversized worker did not terminate');
   assert(error.code==='worker_protocol','Oversized JSONL was accepted');return error;
  });
  await run('stdout-closed-worker-remains-cancellable',async()=>{
   await call('caption_model_download',{config:config('medium')});const progress=await wait(()=>active('fixture_stdout_closed'),'Closed-stdout fixture did not start');
   await sleep(200);const before=performance.now();const cancelled=await call('caption_model_cancel');
   assert(cancelled.status==='cancelled'&&!cancelled.active,'Closed-stdout worker could not be cancelled');
   assert(performance.now()-before<5000,'Cancel blocked on a child wait');return {progress,cancelMs:Math.round(performance.now()-before)};
  });
  await run('mock-runtime-setup-uses-managed-directory',async()=>{
   await call('caption_runtime_setup',{config:config('base',{api_key:'fixture-key-must-be-removed',runtimeDir:'C:/must-not-use',engine:'qwen'})});
   const ready=await wait(()=>done('runtime_ready'),'Runtime fixture did not finish');
   assert(ready.engine==='faster-whisper'&&ready.operation==='runtime','Runtime engine did not follow selected provider');
   assert(JSON.stringify(ready.fixtureRequest.receivedKeys)===JSON.stringify(['engine','runtimeDir']),'Runtime setup received unrelated settings');
   assert(/[/\\]Pointory[/\\]stt-venv$/.test(ready.path)&&!ready.path.includes('must-not-use'),'Runtime directory did not come from the native host');
   return ready;
  });
  await run('runtime-cancellation-shares-the-download-lock',async()=>{
   await call('caption_runtime_setup',{config:{provider:'qwen',model:'Qwen/Qwen3-ASR-0.6B',accelerator:'cpu'}});
   const progress=await wait(()=>active('runtime_install'),'Runtime fixture did not start');
   await rejects(()=>call('caption_model_download',{config:config()}),/already running/i);
   const cancelled=await call('caption_model_cancel');assert(cancelled.status==='cancelled'&&!cancelled.active,'Runtime cancellation failed');return {progress,cancelled};
  });
  await run('simulated-caption-worker-blocks-model-preparation',async()=>{
   await call('caption_start',{config:config('base',{source:'microphone'})});
   await wait(async()=>{const value=await call('caption_snapshot');return value.text==='Fixture captions; no audio';},'Simulated caption worker did not start');
   await rejects(()=>call('caption_model_download',{config:config()}),/stop live captions/i);
   await rejects(()=>call('caption_runtime_setup',{config:config()}),/stop live captions/i);
   await call('caption_stop');return {simulated:true,audioOpened:false,stopped:await call('caption_snapshot')};
  });
  await run('pending-worker-for-app-exit-cleanup',async()=>{
   await call('caption_model_download',{config:config()});const progress=await wait(()=>active('downloading'),'Exit fixture did not start');
   return {fixturePid:progress.fixturePid,fixtureChildPid:progress.fixtureChildPid,verification:'The external runner checks these owned processes after the app exits.'};
  });
  report.final=await call('snapshot');
  report.status=report.checks.every(check=>check.status==='passed')&&report.errors.length===0?'passed':'failed';
 }catch(error){report.status='failed';report.errors.push(String(error));}
 report.modelEvents=events;report.finished=new Date().toISOString();
 await api.event.emit('pointory-validation-finished',report);
})()
