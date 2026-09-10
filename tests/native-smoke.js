// Runs only in a build with --features validation and an isolated profile.
(async()=>{
 if(window.__POINTORY_VALIDATION_RUNNING)return;window.__POINTORY_VALIDATION_RUNNING=true;
 const api=window.__TAURI__,call=(name,args={})=>api.core.invoke(name,args);
 const report={platform:navigator.platform,userAgent:navigator.userAgent,started:new Date().toISOString(),checks:[],errors:[]};
 const pending=new Map();let sequence=0;
 const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
 const assert=(value,message)=>{if(!value)throw Error(message);};
 const wait=async(test,message)=>{for(let i=0;i<150;i++){if(await test())return;await sleep(100);}throw Error(message);};
 const run=async(name,fn)=>{await api.event.emit('pointory-validation-progress',name);const started=performance.now();try{const detail=await fn();report.checks.push({name,status:'passed',durationMs:Math.round(performance.now()-started),detail});}catch(e){report.checks.push({name,status:'failed',durationMs:Math.round(performance.now()-started),error:String(e)});}};
 const remote=(label,script)=>new Promise((resolve,reject)=>{
  const id=String(++sequence),timer=setTimeout(()=>{pending.delete(id);reject(Error('WebView timeout: '+label));},20000);
  pending.set(id,{resolve:value=>{clearTimeout(timer);resolve(value);},reject:error=>{clearTimeout(timer);reject(error);}});
  api.event.emit('pointory-validation-eval',{id,label,script}).catch(error=>{const p=pending.get(id);pending.delete(id);p?.reject(error);});
 });
 const uiError=event=>report.errors.push(String(event.reason||event.error||event.message||'Uncaught WebView error'));
 window.addEventListener('error',uiError);window.addEventListener('unhandledrejection',uiError);
 try{
  await api.event.emit('pointory-validation-progress','boot');
  await api.event.listen('pointory-validation-response',event=>{const p=pending.get(event.payload.id);if(!p)return;pending.delete(event.payload.id);event.payload.error?p.reject(Error(event.payload.error)):p.resolve(event.payload.data);});
  await api.event.listen('capture-error',event=>report.errors.push(event.payload));
  await api.event.listen('pointory-validation-ui-error',event=>report.errors.push(event.payload));
  await wait(()=>document.querySelector('#orientationToggle'),'Toolbar did not load');
  await api.event.emit('pointory-validation-progress','toolbar-ready');
  let s=await call('snapshot');report.initial=s;await api.event.emit('pointory-validation-progress','snapshot-ready');
  if(!s.connected&&s.monitors.length)await call('select_monitor',{id:s.monitors[0].id});
  const configure=async patch=>{const current=await call('snapshot');await call('configure',{preferences:{...current.preferences,...patch}});await sleep(250);};
  const {defaultToolbarItems}=await import('./toolbar-config.js');
  const customToolbarItems=defaultToolbarItems.filter(id=>id!=='capture');
  [customToolbarItems[11],customToolbarItems[12]]=[customToolbarItems[12],customToolbarItems[11]];
  const customWidths={pen_width:7.5,marker_width:12.5,eraser_width:31};
  await configure({language:'ko',layout:'horizontal',toolbar_items:defaultToolbarItems,pen_width:4,marker_width:16,eraser_width:24,global_shortcut_enabled:false,capture_layer_only:true,gif_background:'white',gif_speed:4,spotlight_scale:1});
  await run('native-display-and-brand',async()=>{
   const current=await call('snapshot');assert(current.connected,'No connected display');assert(document.title==='Pointory','Wrong title');
   let brand;await wait(async()=>{brand=await remote('settings',"return {title:document.querySelector('h1')?.textContent,logo:document.querySelector('.logo img')?.naturalWidth};");return brand.logo>0;},'Settings logo did not load');
   assert(brand.title==='Pointory'&&brand.logo>0,'Settings logo did not load');return {monitors:current.monitors,brand};
  });
  await run('horizontal-toolbar-and-panels',async()=>{
   await wait(()=>innerHeight===64,'Horizontal size');const rail=document.querySelector('.toolbar');assert(rail.querySelectorAll('button').length===(await call('snapshot')).preferences.toolbar_items.length+4,'Toolbar control count');assert(rail.scrollWidth<=rail.clientWidth+1,'Horizontal clipping');
   const grip=rail.querySelector('.grip'),gripRect=grip.getBoundingClientRect(),symbol=grip.querySelector('svg');
   assert(Math.abs(gripRect.width-36)<.5&&Math.abs(gripRect.height-36)<.5,'Brand grip must retain a 36 px drag target');
   assert(symbol&&symbol.getBoundingClientRect().width>0&&symbol.getBoundingClientRect().height>0&&getComputedStyle(symbol).display!=='none'&&getComputedStyle(symbol).visibility==='visible','Brand grip symbol is not visible');
   const sourceImage=symbol.querySelector('image');assert(sourceImage?.href.baseVal,'Brand grip image source is missing');
   const symbolImage=new Image();symbolImage.src=sourceImage.href.baseVal;await symbolImage.decode();
   assert(symbolImage.naturalWidth===1254&&symbolImage.naturalHeight===1254,'Brand grip artwork did not decode at its original dimensions');
   assert(/이동|move/i.test(grip.title),'Brand grip tooltip must explain dragging');
   document.querySelector('#shapesToggle').click();await wait(()=>!document.querySelector('#shapesPanel').hidden&&innerHeight>300,'Shapes expansion');document.querySelector('[data-tool=rectangle]').click();
   await wait(async()=>(await call('snapshot')).tool==='rectangle'&&innerHeight===64,'Rectangle selection');document.querySelector('#styleToggle').click();await wait(()=>!document.querySelector('#stylePanel').hidden,'Style panel');
   document.querySelector('[data-slot="1"]').click();document.querySelector('[data-size="16"]').click();await wait(async()=>(await call('snapshot')).width===16,'Pen width');
   const custom=document.querySelector('[data-width-control="active"] [data-width-number]');custom.value='7.5';custom.dispatchEvent(new Event('change',{bubbles:true}));
   await wait(async()=>(await call('snapshot')).width===7.5,'Toolbar custom width did not apply');
   assert(document.querySelector('[data-width-control="active"]').classList.contains('custom-selected'),'Custom width still shows a preset');
   document.body.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));await wait(()=>innerHeight===64,'Escape panel close');return {width:innerWidth,height:innerHeight,customWidth:7.5,grip:{width:gripRect.width,height:gripRect.height,symbolVisible:true,symbolImageDecoded:true,naturalWidth:symbolImage.naturalWidth,naturalHeight:symbolImage.naturalHeight,title:grip.title}};
  });
  await run('vertical-toolbar-and-width-preservation',async()=>{
   document.querySelector('#orientationToggle').click();await wait(()=>innerWidth===64&&document.body.dataset.layout==='vertical','Vertical size');
   assert((await call('snapshot')).width===7.5,'Orientation lost custom width');const rail=document.querySelector('.toolbar');assert(rail.scrollHeight<=rail.clientHeight+1,'Vertical clipping');
   document.querySelector('[data-panel=morePanel]').click();await wait(()=>innerWidth>300,'More panel expansion');document.querySelector('#visibility').click();await wait(async()=>!(await call('snapshot')).visible,'Hide ink');
   await call('action',{name:'toggle_visibility'});assert((await call('snapshot')).visible,'Show ink');return {width:64,height:innerHeight};
  });
  await run('docked-settings-system-fonts-theme',async()=>{
   await call('action',{name:'settings'});const fonts=await call('system_fonts');assert(fonts.length>0,'System fonts empty');
   const family=fonts.includes('Apple SD Gothic Neo')?'Apple SD Gothic Neo':fonts[0];await configure({theme:'teal'});
   await wait(()=>remote('settings',`return [...(document.querySelector('#textFont')?.options||[])].some(option=>option.value===${JSON.stringify(family)});`),'System font dropdown did not populate');
   await remote('settings',`const font=document.querySelector('#textFont');font.value=${JSON.stringify(family)};font.dispatchEvent(new Event('change',{bubbles:true}));return true;`);
   await wait(async()=>(await call('snapshot')).preferences.text_font===family,'Font selection did not save');
   let info;await wait(async()=>{info=await remote('settings',"return {directions:document.querySelectorAll('[data-layout]').length,lines:document.querySelectorAll('[data-lines]').length,selectedFont:document.querySelector('#textFont')?.value,family:getComputedStyle(document.body).fontFamily};");return info.family.includes(family);},'Selected font not applied to settings');
   assert(info.directions===2&&info.lines===0,'Obsolete layout controls');assert(info.selectedFont===family,'Wrong font selected');
   const previousHeight=innerHeight;
   await remote('settings',"document.querySelector('.toolbar-config').open=true;document.querySelector('[data-toolbar-toggle=capture]').click();return true;");
   await wait(async()=>!(await call('snapshot')).preferences.toolbar_items.includes('capture'),'Toolbar visibility choice did not save');
   await wait(()=>document.querySelector('#capture')?.closest('#morePanel')&&innerHeight<previousHeight,'Hidden shortcut did not move to More or resize toolbar');
   await wait(()=>remote('settings',"return !document.querySelector('.toolbar-config-controls').disabled;"),'Toolbar settings stayed disabled');
   await remote('settings',"document.querySelector('[data-toolbar-control=\"board:up\"]').click();return true;");
   await wait(async()=>JSON.stringify((await call('snapshot')).preferences.toolbar_items)===JSON.stringify(customToolbarItems),'Toolbar order did not save');
   await wait(()=>JSON.stringify([...document.querySelectorAll('.toolbar [data-toolbar-item]')].map(button=>button.dataset.toolbarItem))===JSON.stringify(customToolbarItems),'Saved order not applied to toolbar');
   for(const [key,width] of Object.entries(customWidths)) {
    await remote('settings',`const input=document.querySelector('[data-width-control="${key}"] [data-width-number]');input.value='${width}';input.dispatchEvent(new Event('change',{bubbles:true}));return true;`);
    await wait(async()=>(await call('snapshot')).preferences[key]===width,'Custom default did not save: '+key);
   }
   return {fontCount:fonts.length,family,info,toolbarItems:customToolbarItems,customWidths};
  });
  await run('settings-font-size-and-persistence',async()=>{
   const before=await call('snapshot'),toolbarSize={width:innerWidth,height:innerHeight};
   await remote('settings',"const size=document.querySelector('#settingsFontSize');size.value='20';size.dispatchEvent(new Event('change',{bubbles:true}));return true;");
   await wait(async()=>(await call('snapshot')).preferences.settings_font_size===20,'Settings text size did not persist');
   let info;await wait(async()=>{info=await remote('settings',"await document.fonts.ready;const body=getComputedStyle(document.body),select=getComputedStyle(document.querySelector('#settingsFontSize'));return {family:body.fontFamily,size:body.fontSize,controlSize:select.fontSize,scrollWidth:document.documentElement.scrollWidth,width:innerWidth};");return info.size==='20px'&&info.controlSize==='20px';},'Settings text size not applied');
   assert(info.family.includes(before.preferences.text_font),'Selected font not applied to settings');assert(info.size==='20px'&&info.controlSize==='20px','Settings text size not applied');assert(info.scrollWidth<=info.width+1,'Large settings text clips horizontally');
   // Disk persistence and a second process startup are checked by validation.rs.
   await call('action',{name:'settings'});await call('action',{name:'settings'});
   const current=await call('snapshot');assert(current.preferences.settings_font_size===20,'Reopening lost font size');assert(current.width===before.width&&current.preferences.pen_width===before.preferences.pen_width,'Settings size changed pen width');assert(innerWidth===toolbarSize.width&&innerHeight===toolbarSize.height,'Settings size changed toolbar');
   assert(JSON.stringify(current.preferences.toolbar_items)===JSON.stringify(customToolbarItems),'Reopening lost toolbar configuration');
   let controls;await wait(async()=>{controls=await remote('settings',"return {capture:document.querySelector('[data-toolbar-toggle=capture]').checked,order:[...document.querySelectorAll('[data-toolbar-toggle]:checked')].map(input=>input.dataset.toolbarToggle),widths:Object.fromEntries(['pen_width','marker_width','eraser_width'].map(key=>[key,Number(document.querySelector('[data-width-control=\"'+key+'\"] [data-width-number]').value)]))};");return !controls.capture&&JSON.stringify(controls.order)===JSON.stringify(customToolbarItems)&&Object.entries(customWidths).every(([key,width])=>controls.widths[key]===width);},'Reopened settings did not synchronize saved toolbar choices and widths');
   assert(!controls.capture&&JSON.stringify(controls.order)===JSON.stringify(customToolbarItems),'Reopened toolbar settings do not match saved choices');
   assert(Object.entries(customWidths).every(([key,width])=>controls.widths[key]===width),'Reopened custom width controls do not match saved values');return {...info,controls};
  });
  await run('native-webview-korean-text-entry',async()=>{
   document.querySelector('[data-tool=text]').click();await wait(async()=>{const s=await call('snapshot');return s.tool==='text'&&s.drawing;},'Text drawing mode');await sleep(350);
   const text=await remote('overlay',"const root=document.querySelector('#app');root.dispatchEvent(new PointerEvent('pointerdown',{clientX:320,clientY:260,button:0,buttons:1,pointerId:1,bubbles:true}));for(let i=0;i<80&&!document.querySelector('.text-input');i++)await new Promise(r=>setTimeout(r,100));const input=document.querySelector('.text-input');if(!input)throw Error('Text editor did not open');input.value='Pointory 맥 필기 테스트';input.dispatchEvent(new Event('input',{bubbles:true}));const focused=document.activeElement===input;input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return {focused};");
   assert(text.focused,'Text editor was not focused');await wait(async()=>(await call('snapshot')).strokes.some(stroke=>stroke.text?.content==='Pointory 맥 필기 테스트'||stroke.text==='Pointory 맥 필기 테스트'||stroke.content==='Pointory 맥 필기 테스트'),'Text stroke was not committed');return text;
  });
  await run('ink-undo-redo-and-png',async()=>{
   document.querySelector('[data-tool=pen]').click();await wait(async()=>{const s=await call('snapshot');return s.tool==='pen'&&s.drawing;},'Pen drawing mode');let current=await call('snapshot');const before=current.strokes.length;
   await call('add_stroke',{monitor:current.selected,stroke:{tool:'pen',color:'#38bdf8',width:4,points:[{x:.2,y:.2},{x:.4,y:.3}]}});
   await call('action',{name:'undo'});assert((await call('snapshot')).strokes.length===before,'Undo');await call('action',{name:'redo'});assert((await call('snapshot')).strokes.length===before+1,'Redo');
   let path;const unlisten=await api.event.listen('capture-saved',event=>{path=event.payload;});try{await call('request_capture');await wait(()=>Boolean(path),'PNG export did not finish');return {path};}finally{unlisten();}
  });
  await run('gif-export',async()=>{
   const current=await call('snapshot');const {exportGif}=await import('./replay.js');return {path:await exportGif({invoke:call,state:current,onProgress:()=>{},cancelled:()=>false})};
  });
  await run('spotlight-escape-and-toggle',async()=>{
   assert(!(await call('spotlight_status')),'Spotlight unexpectedly enabled before test');await call('spotlight_toggle');
   try{
    await sleep(150);assert(await call('spotlight_status'),'Spotlight did not remain enabled');
    document.body.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));
    await wait(async()=>!(await call('spotlight_status')),'Escape did not stop spotlight');
    await call('spotlight_toggle');assert(await call('spotlight_status'),'Spotlight did not restart after Escape');
    await call('spotlight_toggle');assert(!(await call('spotlight_status')),'Toolbar toggle did not stop spotlight');
    await call('spotlight_stop');assert(!(await call('spotlight_status')),'Repeated stop must not turn spotlight on');
   }finally{await call('spotlight_stop');}
   return {enabled:true,escapeStopped:true,restarted:true,repeatedStopSafe:true,magnification:'disabled for permission-free validation',escape:'WebView key event; physical global key needs a desktop check'};
  });
  await run('classroom-server-start-stop',async()=>{
   let running;try{await call('share_start',{address:'127.0.0.1'});running=await call('share_status');assert(running.running&&running.qr,'Server did not start');assert(new URL(running.url).hostname==='127.0.0.1','Share server did not bind to localhost');}finally{await call('share_stop');}
   const stopped=await call('share_status');assert(!stopped.running,'Server did not stop');return {running:{running:running.running,address:running.address,url:running.url,qrPresent:Boolean(running.qr)},stopped};
  });
  if(window.__POINTORY_VALIDATION_SCREEN)await run('screen-capture-with-system-permission',async()=>{
   await configure({capture_layer_only:false});let saved,error;
   const ok=await api.event.listen('capture-saved',event=>{saved=event.payload;});
   const fail=await api.event.listen('capture-error',event=>{error=event.payload;});
   try{await call('request_capture');await wait(()=>Boolean(saved||error),'Screen capture did not finish');if(error)throw Error(error);return {path:saved};}finally{ok();fail();}
  });
  await call('action',{name:'mouse'});await configure({layout:'horizontal',toolbar_items:customToolbarItems,...customWidths});
  report.final=await call('snapshot');report.status=report.checks.every(check=>check.status==='passed')&&report.errors.length===0?'passed':'failed';
 }catch(e){report.status='failed';report.errors.push(String(e));}
 report.finished=new Date().toISOString();await api.event.emit('pointory-validation-finished',report);
})()
