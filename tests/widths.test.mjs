import test from 'node:test';
import assert from 'node:assert/strict';
import {nextWidth} from '../ui/width-controls.js';
import {fontSizeForPen} from '../ui/text.js';
import {sourceWidth} from '../ui/zoom.js';
import {resizeStroke} from '../ui/selection.js';

test('width shortcuts move in the requested direction from custom values',()=>{
  for (const [value,smaller,larger] of [[1,1,2],[3,2,4],[7.5,4,8],[16,8,24],[24,16,25],[24.5,24,25.5],[63,62,64],[64,63,64]]) {
    assert.equal(nextWidth(value,-1),smaller);
    assert.equal(nextWidth(value,1),larger);
  }
});
test('custom width works for text and survives maximum zoom plus selection resize',()=>{
  assert.equal(fontSizeForPen(7.5),38);
  assert.equal(fontSizeForPen(64),264);
  const width=sourceWidth(1,{scale:6});
  assert.equal(width,1/6);
  const stroke={width,points:[{x:.1,y:.1},{x:.3,y:.3}]};
  assert.equal(resizeStroke(stroke,'se',.3,.3,false,1000,1000).width,width);
});
