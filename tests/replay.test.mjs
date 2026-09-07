import test from 'node:test';import assert from 'node:assert/strict';
import {normalizeRecording,sceneAt} from '../ui/replay.js';
import {pickStroke,translateStroke} from '../ui/selection.js';
const s={tool:'pen',width:4,color:'#ff0000',points:[{x:.2,y:.5},{x:.8,y:.5}],times:[0,1000]};
test('selection picks topmost hit and translation preserves shape at screen edge',()=>{
 assert.equal(pickStroke([s,s],{x:.5,y:.5},1000,1000),1);
 assert.equal(pickStroke([s],{x:.5,y:.1},1000,1000),-1);
 const moved=translateStroke(s,1,-1);assert.ok(Math.abs(moved.points[0].x-.4)<1e-8);assert.equal(moved.points[1].x,1);assert.equal(moved.points[0].y,0);assert.equal(s.points[0].x,.2);
});
test('replay grows a stroke then moves and removes it at recorded times',()=>{
 const after=translateStroke(s,0,.2),events=normalizeRecording([{kind:'draw',at:0,stroke:s},{kind:'move',at:1100,duration:1000,index:0,before:s,after},{kind:'replace',at:2200,strokes:[]}]);
 assert.equal(sceneAt(events,500)[0].points.at(-1).x,.5);
 assert.ok(Math.abs(sceneAt(events,1600)[0].points[0].y-.6)<1e-8);
 assert.equal(sceneAt(events,2200).length,0);
});
test('replay retains gaps and orders overlapping clocks monotonically',()=>{
 const events=normalizeRecording([{kind:'draw',at:0,stroke:s},{kind:'draw',at:900,stroke:s},{kind:'replace',at:5000,strokes:[]}]);
 assert.equal(events[1].at,1000);assert.equal(events[2].at,5000);assert.equal(sceneAt(events,4000).length,2);
});
import {resizeStroke} from '../ui/selection.js';
import {defaultBindings,keyChord,matchShortcut} from '../ui/shortcuts.js';
test('resize anchors the opposite corner, changes thickness and respects aspect ratio',()=>{
 const rect={...s,tool:'rectangle',points:[{x:.2,y:.2},{x:.4,y:.4}]};
 const resized=resizeStroke(rect,'se',.6,.8,false,1000,1000);
 assert.deepEqual(resized.points[0],rect.points[0]);assert.equal(resized.points[1].x,.6);assert.ok(Math.abs(resized.points[1].y-.8)<1e-8);assert.ok(resized.width>rect.width);
 const locked=resizeStroke(rect,'se',.6,.8,true,1000,1000);assert.ok(Math.abs(locked.points[1].x-locked.points[1].y)<1e-8);
 const bounded=resizeStroke(rect,'nw',-99,-99,false,1000,1000);assert.ok(bounded.points.every(p=>p.x>=0&&p.y>=0&&p.x<=1&&p.y<=1));assert.deepEqual(bounded.points[1],rect.points[1]);
 const tiny=resizeStroke(rect,'se',.1,.1,false,1000,1000);assert.ok(tiny.points[1].x>tiny.points[0].x);
});
test('resize replay interpolates both points and line thickness',()=>{
 const after={...s,width:8,points:[s.points[0],{x:.9,y:.7}]};
 const events=normalizeRecording([{kind:'draw',at:0,stroke:s},{kind:'move',at:1100,duration:1000,index:0,before:s,after}]);
 assert.equal(sceneAt(events,1600)[0].width,6);assert.ok(Math.abs(sceneAt(events,1600)[0].points[1].x-.85)<1e-8);
});
test('key mappings match custom chords, honor individual toggles and reserve Escape',()=>{
 const bindings=structuredClone(defaultBindings);bindings.pen.key='Mod+Alt+K';
 const e={key:'k',code:'KeyK',ctrlKey:true,altKey:true,shiftKey:false};assert.equal(keyChord(e),'Mod+Alt+K');assert.equal(matchShortcut(e,bindings),'pen');
 bindings.pen.enabled=false;assert.equal(matchShortcut(e,bindings),null);assert.equal(keyChord({key:'Escape',code:'Escape'}),null);
 assert.equal(matchShortcut({key:'P',code:'KeyP',shiftKey:false},defaultBindings),'pen');
});
import {fadeOpacity,visibleFading,boardBackground} from '../ui/lecture.js';
test('temporary ink fades out without delaying subsequent persistent strokes',()=>{
 assert.equal(fadeOpacity(0,1000),1);assert.ok(fadeOpacity(900,1000)<1);assert.equal(fadeOpacity(1000,1000),0);
 assert.equal(visibleFading([{stroke:s,born:100,life:1000}],1100).length,0);
 const events=normalizeRecording([{kind:'fade',at:0,stroke:{...s,tool:'fade',times:[0,100]},life:1000},{kind:'draw',at:200,stroke:{...s,times:[0,100]}}]);
 assert.equal(events[1].at,200);assert.equal(sceneAt(events,500).length,2);assert.equal(sceneAt(events,1200).length,1);
});
test('board backgrounds return to desktop in mouse mode or when hidden',()=>{
 assert.equal(boardBackground({board:'white',drawing:true,visible:true}),'#ffffff');
 assert.equal(boardBackground({board:'black',drawing:true,visible:true}),'#171717');
 assert.equal(boardBackground({board:'white',drawing:false,visible:true}),'transparent');
 assert.equal(boardBackground({board:'black',drawing:true,visible:false}),'transparent');
});
