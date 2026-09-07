import {bundle} from '@remotion/bundler';
import {selectComposition,renderMedia,renderStill} from '@remotion/renderer';
import {mkdir} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.dirname(fileURLToPath(import.meta.url));
process.chdir(root);
await mkdir('out',{recursive:true});
const serveUrl=await bundle({entryPoint:path.join(root,'src/index.jsx'),publicDir:path.join(root,'public')});
const browserExecutable=process.env.REMOTION_BROWSER_EXECUTABLE||undefined;
const composition=await selectComposition({serveUrl,id:'LayerPenIntro',browserExecutable});
if(process.argv.includes('--stills')){
 for(const frame of [100,280,460,640,820,1000])await renderStill({serveUrl,composition,frame,browserExecutable,output:path.join(root,`out/frame-${frame}.png`)});
}else{
 let last=-1;
 await renderMedia({serveUrl,composition,browserExecutable,codec:'h264',crf:18,concurrency:2,outputLocation:path.join(root,'out/LayerPen-intro-v0.1.0.mp4'),onProgress:p=>{const n=Math.floor(p.progress*10);if(n!==last){last=n;console.log(`Rendering ${n*10}%`);}}});
}
console.log('Render complete');
