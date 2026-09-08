export const shortcutDefinitions=[
 ['zoom','부분 확대','Z'],['zoom_reset','확대 종료','0'],['fade','사라지는 잉크','F'],['board','화면 / 화이트 / 블랙보드','B'],['pen','펜','P'],['marker','형광펜','H'],['eraser','지우개','E'],['select','선택 · 이동 · 크기','V'],['line','직선','L'],['rectangle','사각형','R'],['ellipse','타원','O'],['mouse','마우스 모드','M'],['visibility','판서 숨김 / 표시','Tab'],['undo','실행 취소','Mod+Z'],['redo','다시 실행','Mod+Shift+Z'],['capture','PNG 캡처','Mod+Shift+S'],['gif','GIF 내보내기','Mod+Shift+G'],['clear','모두 지우기 (두 번)','Mod+Shift+Delete'],['color1','색상 1','1'],['color2','색상 2','2'],['color3','색상 3','3'],['color4','색상 4','4'],['color5','색상 5','5'],['thinner','굵기 줄이기','['],['thicker','굵기 늘리기',']']
];
export const defaultBindings=Object.fromEntries(shortcutDefinitions.map(([id,,key])=>[id,{key,enabled:true}]));
export function keyChord(event){
 if(event.repeat||['Shift','Control','Alt','Meta','Escape'].includes(event.key))return null;
 let key=event.code?.replace(/^Key|^Digit/,'');
 const special={BracketLeft:'[',BracketRight:']',Space:'Space',Backquote:'`',Minus:'-',Equal:'=',Comma:',',Period:'.',Slash:'/',Backslash:'\\',Semicolon:';',Quote:"'"};
 key=special[event.code]||key||event.key;
 if(!/^(?:[A-Z0-9]|F(?:[1-9]|1[0-2])|Tab|Enter|Delete|Backspace|Space|Arrow(?:Up|Down|Left|Right)|Home|End|PageUp|PageDown|[\[\]`=,./\\;'\-])$/.test(key))return null;
 return [...(event.ctrlKey||event.metaKey?['Mod']:[]),...(event.altKey?['Alt']:[]),...(event.shiftKey?['Shift']:[]),key].join('+');
}
export const displayChord=key=>key.replace('CommandOrControl','Mod').replace('Mod',/Mac/.test(globalThis.navigator?.platform||'')?'⌘':'Ctrl').replaceAll('+',' + ');
export function matchShortcut(event,bindings){const chord=keyChord(event);if(!chord)return null;return Object.entries(bindings).find(([,b])=>b.enabled&&b.key===chord)?.[0]||null;}
