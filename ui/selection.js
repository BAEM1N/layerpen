import {hits} from './hit-test.js';
export function pickStroke(strokes,point,w,h,scale=1){const probe={tool:'eraser',width:12/scale,points:[point]};for(let i=strokes.length-1;i>=0;i--)if(hits(strokes[i],probe,w,h))return i;return -1;}
export function translateStroke(stroke,dx,dy){
 const xs=stroke.points.map(p=>p.x),ys=stroke.points.map(p=>p.y);
 const minX=xs.reduce((a,b)=>Math.min(a,b),1),maxX=xs.reduce((a,b)=>Math.max(a,b),0),minY=ys.reduce((a,b)=>Math.min(a,b),1),maxY=ys.reduce((a,b)=>Math.max(a,b),0);
 dx=Math.max(-minX,Math.min(1-maxX,dx));dy=Math.max(-minY,Math.min(1-maxY,dy));
 return {...stroke,points:stroke.points.map(p=>({x:p.x+dx,y:p.y+dy}))};
}
export function strokeBounds(stroke){return stroke.points.reduce((b,p)=>({minX:Math.min(b.minX,p.x),minY:Math.min(b.minY,p.y),maxX:Math.max(b.maxX,p.x),maxY:Math.max(b.maxY,p.y)}),{minX:1,minY:1,maxX:0,maxY:0});}
export function resizeStroke(stroke,handle,x,y,uniform,w,h){
 const b=strokeBounds(stroke),east=handle.endsWith('e'),south=handle.startsWith('s'),ax=east?b.minX:b.maxX,ay=south?b.minY:b.maxY,bw=Math.max(b.maxX-b.minX,1/w),bh=Math.max(b.maxY-b.minY,1/h);
 const limitX=(east?1-ax:ax)/bw,limitY=(south?1-ay:ay)/bh;
 let sx=Math.min(Math.max((x-ax)*(east?1:-1)/bw,2/w/bw),Math.max(0,limitX)),sy=Math.min(Math.max((y-ay)*(south?1:-1)/bh,2/h/bh),Math.max(0,limitY));
 if(uniform)sx=sy=Math.max(0,Math.min(Math.max(sx,sy),limitX,limitY));
 return {...stroke,width:Math.max(.25,Math.min(80,stroke.width*Math.sqrt(sx*sy))),points:stroke.points.map(p=>({x:Math.max(0,Math.min(1,ax+(p.x-ax)*sx)),y:Math.max(0,Math.min(1,ay+(p.y-ay)*sy))}))};
}
