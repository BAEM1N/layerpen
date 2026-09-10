import test from 'node:test';
import assert from 'node:assert/strict';
import {toolbarItems,defaultToolbarItems,normalizeToolbarItems,moveToolbarItem} from '../ui/toolbar-config.js';

test('toolbar preferences migrate missing values while allowing all tools in More',()=>{
  assert.deepEqual(normalizeToolbarItems(undefined),defaultToolbarItems);
  assert.deepEqual(normalizeToolbarItems([]),[]);
  assert.deepEqual(normalizeToolbarItems(['capture','unknown','capture','pen']),['capture','pen']);
  assert.equal(defaultToolbarItems[0],'visibility');
  assert.equal(defaultToolbarItems[1],'mouse');
  assert.ok(defaultToolbarItems.includes('capture') && defaultToolbarItems.includes('board'));
  assert.equal(new Set(toolbarItems.map(([id])=>id)).size,18);
});
test('toolbar ordering only moves enabled items and keeps boundaries intact',()=>{
  const source=['visibility','mouse','capture'];
  assert.deepEqual(moveToolbarItem(source,'capture',-1),['visibility','capture','mouse']);
  assert.deepEqual(moveToolbarItem(source,'visibility',-1),source);
  assert.deepEqual(moveToolbarItem(source,'capture',1),source);
  assert.deepEqual(moveToolbarItem(source,'board',1),source);
  assert.deepEqual(source,['visibility','mouse','capture']);
});
