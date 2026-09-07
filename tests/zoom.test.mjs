import test from 'node:test';import assert from 'node:assert/strict';
import {sourcePoint,screenPoint,regionView,sourceWidth,activeZoom} from '../ui/zoom.js';
import {redraw} from '../ui/drawing.js';
test('zoom coordinates round-trip across landscape, portrait and fractional scales',()=>{
 for(const [w,h] of [[1920,1080],[1080,1920]])for(const scale of [1.1,2,6]){const view={scale,x:.01,y:.02},p=sourcePoint(w*.3,h*.7,w,h,view),back=screenPoint(p,w,h,view);assert.ok(Math.abs(back.x-w*.3)<1e-8);assert.ok(Math.abs(back.y-h*.7)<1e-8);}
});
test('chosen pen size is constant on screen and scales back to the original',()=>{
 assert.equal(sourceWidth(8,{scale:2}),4);assert.equal(sourceWidth(2,{scale:6}),1/3);assert.equal(sourceWidth(8,null),8);
});
test('region zoom is centered and clamped inside the original screen',()=>{
 const v=regionView({x:.25,y:.25},{x:.75,y:.75},1000,1000);assert.deepEqual(v,{scale:2,x:.25,y:.25});
 const click=regionView({x:.99,y:.99},{x:.99,y:.99},1000,1000);assert.equal(click.scale,2);assert.equal(click.x,.5);assert.equal(click.y,.5);
 assert.equal(regionView({x:.2,y:.2},{x:.21,y:.21},1000,1000).scale,6);
});
test('renderer applies DPI and zoom once and keeps source geometry unchanged',()=>{
 const calls=[],ctx=new Proxy({},{get:(_,key)=>(...args)=>calls.push([key,...args]),set:()=>true}),canvas={clientWidth:1000,clientHeight:600,width:0,height:0,getContext:()=>ctx};
 const stroke={tool:'line',width:4,color:'#ff0000',points:[{x:.3,y:.3},{x:.4,y:.4}]};
 redraw(canvas,[stroke],null,1.5,{scale:2,x:.25,y:.25});
 assert.ok(calls.some(c=>JSON.stringify(c)===JSON.stringify(['setTransform',3,0,0,3,-750,-450])));
 assert.ok(calls.some(c=>JSON.stringify(c)===JSON.stringify(['moveTo',300,180])));assert.equal(stroke.width,4);
 assert.equal(activeZoom({drawing:false,capturing:false,zoom:{scale:2}}),null);
});
