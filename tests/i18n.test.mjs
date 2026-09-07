import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {resolveLanguage,translator} from '../ui/i18n.js';
const catalogs=Object.fromEntries(['ko','en','ja','zh-CN'].map(l=>[l,JSON.parse(fs.readFileSync(new URL(`../ui/locales/${l}.json`,import.meta.url)))]));
test('all languages cover the same messages and preserve placeholders',()=>{
 for(const [language,catalog] of Object.entries(catalogs)) {
  assert.deepEqual(Object.keys(catalog),Object.keys(catalogs.ko));
  for(const [key,value] of Object.entries(catalog)) {
   assert.ok(value.trim(),`${language}: ${key}`);
   assert.deepEqual((value.match(/\{\w+\}/g)||[]).sort(),(key.match(/\{\w+\}/g)||[]).sort());
  }
 }
});
test('language detection and fallback respect explicit preferences',()=>{
 assert.equal(resolveLanguage('ja',['ko-KR']),'ja');
 assert.equal(resolveLanguage('auto',['zh-TW']),'zh-CN');
 assert.equal(resolveLanguage('auto',['fr-FR','ja-JP']),'ja');
 assert.equal(resolveLanguage('auto',['de-DE']),'en');
 assert.equal(resolveLanguage('unknown',['en-US']),'en');
});
test('native errors translate while paths and external error details remain intact',()=>{
 const t=translator(catalogs.en);
 assert.equal(t('설정을 저장하지 못했습니다: C:\\자료\\회의.txt'),'Could not save settings: C:\\자료\\회의.txt');
 assert.equal(t('저장됨: C:\\회의\\펜.png'),'Saved: C:\\회의\\펜.png');
 assert.equal(t('● 필기 중 · x2.0 · 내 모니터'),'● Drawing · x2.0 · 내 모니터');
 assert.equal(t('펜 (Ctrl + P)'),'Pen (Ctrl + P)');
 assert.equal(t('색상 #ffffff'),'Color #ffffff');
 assert.equal(t('3초'),'3 seconds');
 assert.equal(t('unrecognized diagnostic'),'unrecognized diagnostic');
});
