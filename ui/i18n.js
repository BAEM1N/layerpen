// Korean source messages are stable IDs for the existing UI and native errors.
// Translation only touches visible text and accessibility attributes, never markup.
export const languages = {auto:'System',ko:'한국어',en:'English',ja:'日本語','zh-CN':'简体中文'};
export function resolveLanguage(choice, preferred = globalThis.navigator?.languages || ['en']) {
  if (choice !== 'auto' && Object.hasOwn(languages, choice)) return choice;
  for (const item of preferred) {
    const base = item.toLowerCase().split('-')[0];
    if (base === 'zh') return 'zh-CN';
    if (['ko','en','ja'].includes(base)) return base;
  }
  return 'en';
}
const escapeRegExp = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export function translator(catalog) {
  const patterns = Object.keys(catalog).filter(key => key.includes('{')).map(key => {
    const names = [...key.matchAll(/\{(\w+)\}/g)].map(m=>m[1]);
    const parts=key.split(/\{\w+\}/).map(escapeRegExp);
    return {key,names,re:new RegExp('^'+parts.join('([\\s\\S]+?)')+'$')};
  });
  return function translate(value) {
    const text=String(value), key=text.trim();
    let result=catalog[key];
    if (result === undefined) {
      for (const pattern of patterns) {
        const match=key.match(pattern.re);
        if (match) { result=catalog[pattern.key].replace(/\{(\w+)\}/g,(_,name)=>match[pattern.names.indexOf(name)+1]); break; }
      }
    }
    // Toolbar captions contain a status prefix and an unmodified device name.
    if(result===undefined && key.includes(' · ')) {
      const index=key.indexOf(' · '), prefix=key.slice(0,index);
      if(Object.hasOwn(catalog,prefix)) result=catalog[prefix]+key.slice(index);
    }
    // Tool titles append user-configured key chords to their source labels.
    if(result===undefined) {
      const match=key.match(/^(.*) (\([^()]+\))$/);
      if(match && Object.hasOwn(catalog,match[1])) result=catalog[match[1]]+' '+match[2];
    }
    return result===undefined ? text : text.slice(0,text.indexOf(key))+result+text.slice(text.indexOf(key)+key.length);
  };
}
let current='en', catalogs={}, translate=value=>String(value);
export const t=value=>translate(value);
export const currentLanguage=()=>current;
export async function loadLanguages() {
  for (const language of Object.keys(languages).filter(x=>x!=='auto')) {
    const response=await fetch(new URL(`./locales/${language}.json`,import.meta.url));
    if(!response.ok) throw new Error(`Cannot load language: ${language}`);
    catalogs[language]=await response.json();
  }
  setLanguage('auto');
}
export function setLanguage(choice) {
  current=resolveLanguage(choice);
  translate=translator({...catalogs.en,...catalogs[current]});
  if(globalThis.document) document.documentElement.lang=current;
}

// Retain original source text per node so switching languages is reversible.
// Dynamic renderers may overwrite translated text; their new source is recorded.
export function localizeDocument(doc=document) {
  const originals=new WeakMap();
  const apply=(node,attribute,value,write)=>{
    const fields=originals.get(node)||{};
    let record=fields[attribute];
    if(!record || value!==record.output) record={source:value};
    const output=t(record.source);
    fields[attribute]={source:record.source,output};originals.set(node,fields);
    if(output!==value) write(output);
  };
  const refresh=()=>{
    const walker=doc.createTreeWalker(doc.body,NodeFilter.SHOW_TEXT);
    for(let node=walker.nextNode();node;node=walker.nextNode()) {
      if(node.parentElement?.closest('script,style,[data-i18n-skip],.monitor-copy strong,#lastCapture')) continue;
      apply(node,'text',node.textContent,value=>node.textContent=value);
    }
    for(const node of doc.body.querySelectorAll('[title],[aria-label],[placeholder]')) {
      for(const name of ['title','aria-label','placeholder']) if(node.hasAttribute(name)) apply(node,name,node.getAttribute(name),value=>node.setAttribute(name,value));
    }
  };
  const observer=new MutationObserver(()=>refresh());
  observer.observe(doc.body,{subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:['title','aria-label','placeholder']});
  refresh();
  return {refresh,disconnect:()=>observer.disconnect()};
}
