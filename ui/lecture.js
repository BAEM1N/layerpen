export function fadeOpacity(elapsed,life){if(elapsed>=life)return 0;return Math.min(1,Math.max(0,(life-elapsed)/Math.min(600,life*.4)));}
export function visibleFading(items,now=Date.now()){return (items||[]).map(f=>({...f.stroke,opacity:fadeOpacity(now-f.born,f.life)})).filter(s=>s.opacity>0);}
export function boardBackground(state){return state?.drawing&&state?.visible!==false?(state.board==='white'?'#ffffff':state.board==='black'?'#171717':'transparent'):'transparent';}
