const native=window.__TAURI__;
const loaded=new Map();let pending;
export async function ensureFonts(){
 if(!native)return [];
 if(pending)return pending;
 pending=(async()=>{const assets=await native.core.invoke('font_assets');
 for(const item of assets){if(!loaded.has(item.id)){const data=await native.core.invoke('font_data',{id:item.id});const face=new FontFace(item.family,Uint8Array.from(atob(data),c=>c.charCodeAt(0)));await face.load();document.fonts.add(face);loaded.set(item.id,face);}}
 return assets;})();
 try{return await pending;}finally{pending=null;}
}
export async function fontChoices(){const [system,assets]=await Promise.all([native?native.core.invoke('system_fonts'):Promise.resolve(['sans-serif']),ensureFonts()]);return {system,assets};}
