function path(s,w,h){
 const a={x:s.points[0].x*w,y:s.points[0].y*h},p=s.points.at(-1),b={x:p.x*w,y:p.y*h};
 if(s.tool==='line')return[a,b];
 if(s.tool==='rectangle')return[a,{x:b.x,y:a.y},b,{x:a.x,y:b.y},a];
 if(s.tool==='ellipse'){
  const rx=Math.abs(b.x-a.x)/2,ry=Math.abs(b.y-a.y)/2,n=Math.max(32,Math.min(1024,Math.ceil(Math.PI*Math.sqrt(Math.max(rx,ry)/.5))));
  return Array.from({length:n+1},(_,i)=>({x:(a.x+b.x)/2+rx*Math.cos(i*Math.PI*2/n),y:(a.y+b.y)/2+ry*Math.sin(i*Math.PI*2/n)}));
 }
 return s.points.map(p=>({x:p.x*w,y:p.y*h}));
}
const cross=(a,b,c)=>(b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x);
function distance(p,a,b){const dx=b.x-a.x,dy=b.y-a.y,l=dx*dx+dy*dy,t=l?Math.max(0,Math.min(1,((p.x-a.x)*dx+(p.y-a.y)*dy)/l)):0;return Math.hypot(p.x-a.x-t*dx,p.y-a.y-t*dy);}
function near(a,b,c,d,r){
 if(Math.max(a.x,b.x)+r<Math.min(c.x,d.x)||Math.max(c.x,d.x)+r<Math.min(a.x,b.x)||Math.max(a.y,b.y)+r<Math.min(c.y,d.y)||Math.max(c.y,d.y)+r<Math.min(a.y,b.y))return false;
 return cross(a,b,c)*cross(a,b,d)<0&&cross(c,d,a)*cross(c,d,b)<0||Math.min(distance(a,c,d),distance(b,c,d),distance(c,a,b),distance(d,a,b))<=r;
}
export function hits(stroke,eraser,w,h){
 if(!stroke.points.length||!eraser.points.length)return false;
 const a=path(stroke,w,h),b=path(eraser,w,h),r=(stroke.width+eraser.width)/2;
 for(let i=0;i<Math.max(1,a.length-1);i++)for(let j=0;j<Math.max(1,b.length-1);j++)if(near(a[i],a[Math.min(i+1,a.length-1)],b[j],b[Math.min(j+1,b.length-1)],r))return true;
 return false;
}
