export const themes={blue:{label:'블루',hue:212},teal:{label:'청록',hue:172},green:{label:'그린',hue:142},orange:{label:'오렌지',hue:28},purple:{label:'퍼플',hue:267}};
export function resolveTheme(name){return Object.hasOwn(themes,name)?name:'blue';}
export function applyTheme(name,root=document.documentElement){const key=resolveTheme(name);root.dataset.theme=key;root.style.setProperty('--theme-hue',themes[key].hue);}
