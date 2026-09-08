export function activeZoom(state){return state?.zoom&&(state.drawing||state.capturing)?state.zoom:null;}
export function sourcePoint(x,y,w,h,view){const scale=view?.scale||1;return{x:Math.max(0,Math.min(1,(view?.x||0)+x/Math.max(1,w)/scale)),y:Math.max(0,Math.min(1,(view?.y||0)+y/Math.max(1,h)/scale))};}
export function screenPoint(p,w,h,view){return{x:(p.x-(view?.x||0))*w*(view?.scale||1),y:(p.y-(view?.y||0))*h*(view?.scale||1)};}
export function regionView(a,b,w,h){const rw=Math.abs(a.x-b.x),rh=Math.abs(a.y-b.y),scale=rw*w<8||rh*h<8?2:Math.max(1.1,Math.min(6,1/rw,1/rh));return{scale,x:Math.max(0,Math.min(1-1/scale,(a.x+b.x)/2-.5/scale)),y:Math.max(0,Math.min(1-1/scale,(a.y+b.y)/2-.5/scale))};}
export const sourceWidth=(width,view)=>width/(view?.scale||1);
