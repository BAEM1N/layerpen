import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizedPoint, paintStroke, redraw } from '../ui/drawing.js';
import { hits } from '../ui/hit-test.js';

function context() {
  const calls=[];
  const ctx=new Proxy({calls},{set(target,key,value){calls.push([key,value]);target[key]=value;return true;},get(target,key){return key in target?target[key]:(...args)=>calls.push([key,...args]);}});
  return ctx;
}
const stroke={tool:'pen',color:'#123456',width:4,points:[{x:.25,y:.5},{x:.75,y:.5}]};
test('pointer coordinates remain inside the selected display',()=>{
  assert.deepEqual(normalizedPoint(-5,120,100,100),{x:0,y:1});
  assert.deepEqual(normalizedPoint(960,540,1920,1080),{x:.5,y:.5});
  assert.deepEqual(normalizedPoint(0,0,0,0),{x:0,y:0});
});
test('eraser is a deletion gesture and never paints pixels',()=>{
  const ctx=context();paintStroke(ctx,{...stroke,tool:'eraser'},1920,1080);
  assert.deepEqual(ctx.calls,[]);
});
test('eraser preview removes the entire hit stroke and keeps others',()=>{
 const ctx=context(),canvas={clientWidth:1000,clientHeight:1000,width:1000,height:1000,getContext:()=>ctx};
 const other={...stroke,points:[{x:.1,y:.1}]};
 const eraser={...stroke,tool:'eraser',points:[{x:.5,y:.3},{x:.5,y:.7}]};
 redraw(canvas,[stroke,other],eraser,1);
 assert.equal(ctx.calls.filter(c=>c[0]==='stroke').length,0);
 assert.equal(ctx.calls.filter(c=>c[0]==='fill').length,1);
});
test('shape interiors are empty, borders and fast crossings are hit',()=>{
 const eraser={...stroke,tool:'eraser',points:[{x:.5,y:.5}]};
 for(const tool of ['rectangle','ellipse']){
  const shape={...stroke,tool,points:[{x:.2,y:.2},{x:.8,y:.8}]};
  assert.equal(hits(shape,eraser,1000,1000),false);
  assert.equal(hits(shape,{...eraser,points:[{x:.8,y:.5}]},1000,1000),true);
 }
 assert.equal(hits(stroke,{...eraser,points:[{x:.5,y:0},{x:.5,y:1}]},1000,1000),true);
});
test('marker is stroked once for consistent opacity across pointer samples',()=>{
  const ctx=context();paintStroke(ctx,{...stroke,tool:'marker'},100,100);
  assert.equal(ctx.calls.filter(c=>c[0]==='stroke').length,1);
  assert.ok(ctx.calls.some(c=>c[0]==='globalAlpha'&&c[1]===.3));
});
test('single click draws a dot',()=>{
  const ctx=context();paintStroke(ctx,{...stroke,points:[{x:.5,y:.5}]},100,100);
  assert.deepEqual(ctx.calls.find(c=>c[0]==='arc'),['arc',50,50,2,0,Math.PI*2]);
  assert.equal(ctx.calls.filter(c=>c[0]==='fill').length,1);
});
test('HiDPI raster size is scaled while stroke coordinates stay logical',()=>{
  const ctx=context();const canvas={clientWidth:800,clientHeight:600,width:0,height:0,getContext:()=>ctx};
  redraw(canvas,[stroke],null,1.5);
  assert.equal(canvas.width,1200);assert.equal(canvas.height,900);
  assert.deepEqual(ctx.calls.find(c=>c[0]==='setTransform'),['setTransform',1.5,0,0,1.5,0,0]);
  assert.deepEqual(ctx.calls.find(c=>c[0]==='moveTo'),['moveTo',200,300]);
});
test('rectangle supports dragging toward the upper left',()=>{
 const ctx=context();paintStroke(ctx,{...stroke,tool:'rectangle',points:[{x:.8,y:.9},{x:.2,y:.1}]},100,100);
 const rect=ctx.calls.find(c=>c[0]==='rect');assert.equal(rect[1],20);assert.equal(rect[2],10);assert.ok(Math.abs(rect[3]-60)<1e-8);assert.equal(rect[4],80);
});
test('straight line ignores intermediate points',()=>{
 const ctx=context();paintStroke(ctx,{...stroke,tool:'line',points:[{x:0,y:0},{x:.2,y:.8},{x:1,y:1}]},100,100);
 assert.deepEqual(ctx.calls.filter(c=>c[0]==='lineTo'),[['lineTo',100,100]]);
});
test('marker keeps opacity captured at stroke creation',()=>{
 const ctx=context();paintStroke(ctx,{...stroke,tool:'marker',opacity:.6},100,100);
 assert.ok(ctx.calls.some(c=>c[0]==='globalAlpha'&&c[1]===.6));
});
