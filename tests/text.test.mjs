import test from 'node:test';
import assert from 'node:assert/strict';
import {textStroke,fontSizeForPen} from '../ui/text.js';
import {paintStroke} from '../ui/drawing.js';
import {hits} from '../ui/hit-test.js';
import {translateStroke,resizeStroke} from '../ui/selection.js';
import {sceneAt,normalizeRecording} from '../ui/replay.js';
const ctx={save(){},restore(){},measureText(s){return {width:[...s].length*12};}};
const make=(extra={})=>textStroke(ctx,{content:'한글 Text\n두 번째 줄',font:'Malgun Gothic',width:4,color:'#123456',point:{x:.2,y:.3},canvasWidth:1000,canvasHeight:800,...extra});
test('text sizes follow all five pen sizes; zoom converts once',()=>{
 assert.deepEqual([2,4,8,16,24].map(fontSizeForPen),[16,24,40,72,104]);
 assert.equal(make({scale:2}).text.size,12);assert.equal(make({content:'  '}),null);
});
test('text is picked/erased inside its box and survives move, resize, undo replay',()=>{
 const s=make(),a=s.points[0],b=s.points[1],center={x:(a.x+b.x)/2,y:(a.y+b.y)/2};
 assert(hits(s,{width:2,points:[center]},1000,800));
 assert(!hits(s,{width:2,points:[{x:.9,y:.9}]},1000,800));
 const moved=translateStroke(s,.1,.1),resized=resizeStroke(s,'se',b.x+.1,b.y+.1,true,1000,800);
 assert.deepEqual(moved.text,s.text);assert.deepEqual(resized.text,s.text);
 const events=normalizeRecording([{kind:'draw',at:0,stroke:s},{kind:'move',at:10,duration:20,index:0,before:s,after:moved}]);
 assert.equal(sceneAt(events,40)[0].text.content,s.text.content);
 assert.equal(sceneAt(events,40)[0].points[0].x,moved.points[0].x);
});
test('PNG/GIF shared renderer draws Unicode text with saved font and geometric scale',()=>{
 const calls=[];const c={save(){},restore(){},beginPath(){},translate(...x){calls.push(['origin',...x]);},scale(...x){calls.push(['scale',...x]);},fillText(...x){calls.push(['text',...x]);}};
 const s=make();paintStroke(c,s,1000,800);
 assert(c.font.includes('Malgun Gothic'));assert.equal(calls.filter(x=>x[0]==='text').length,2);
 assert.deepEqual(calls.find(x=>x[0]==='origin'),['origin',200,240]);
 assert.equal(calls.find(x=>x[0]==='text')[1],'한글 Text');
});
test('long input wraps to the remaining display width and clamps geometry',()=>{
 const s=make({content:'가나다라마바사아자차카타파하',point:{x:.98,y:.9}});
 assert(s.text.content.includes('\n'));assert(s.points[1].x<=1&&s.points[1].y<=1);
});
