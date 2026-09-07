import { hits } from './hit-test.js';
export function normalizedPoint(x, y, width, height) {
  return { x: Math.max(0, Math.min(1, x / Math.max(1, width))), y: Math.max(0, Math.min(1, y / Math.max(1, height))) };
}

// Render each stroke in one path so a marker does not darken at every pointer sample.
export function paintStroke(ctx, stroke, width, height) {
  if (!stroke.points.length || stroke.tool === 'eraser') return;
  ctx.save();
  ctx.globalCompositeOperation = 'source-over';
  ctx.globalAlpha = stroke.tool === 'marker' ? (stroke.opacity ?? 0.3) : stroke.tool==='fade'?(stroke.opacity??1):1;
  ctx.strokeStyle = stroke.color;
  ctx.fillStyle = stroke.color;
  ctx.lineWidth = stroke.width;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  const first = stroke.points[0];
  ctx.beginPath();
  const last = stroke.points.at(-1);
  if (stroke.tool === 'rectangle') {
    ctx.rect(Math.min(first.x,last.x)*width,Math.min(first.y,last.y)*height,Math.abs(last.x-first.x)*width,Math.abs(last.y-first.y)*height);ctx.stroke();
  } else if (stroke.tool === 'ellipse') {
    ctx.ellipse((first.x+last.x)*width/2,(first.y+last.y)*height/2,Math.abs(last.x-first.x)*width/2,Math.abs(last.y-first.y)*height/2,0,0,Math.PI*2);ctx.stroke();
  } else if (stroke.tool === 'line') {
    ctx.moveTo(first.x*width,first.y*height);ctx.lineTo(last.x*width,last.y*height);ctx.stroke();
  } else if (stroke.points.length === 1) {
    ctx.arc(first.x * width, first.y * height, stroke.width / 2, 0, Math.PI * 2);
    ctx.fill();
  } else {
    ctx.moveTo(first.x * width, first.y * height);
    for (const p of stroke.points.slice(1)) ctx.lineTo(p.x * width, p.y * height);
    ctx.stroke();
  }
  ctx.restore();
}

export function redraw(canvas, strokes, draft = null, ratio = globalThis.devicePixelRatio || 1, view = null) {
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  const w = Math.max(1, Math.round(width * ratio));
  const h = Math.max(1, Math.round(height * ratio));
  if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
  const ctx = canvas.getContext('2d');
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.clearRect(0, 0, width, height);
  if(view)ctx.setTransform(ratio*view.scale,0,0,ratio*view.scale,-view.x*width*ratio*view.scale,-view.y*height*ratio*view.scale);
  for (const stroke of strokes || []) {
    if(draft?.tool==='eraser' && hits(stroke,draft,width,height))continue;
    paintStroke(ctx, stroke, width, height);
  }
  if (draft) paintStroke(ctx, draft, width, height);
}
export function exportLayer(source,strokes,view=null){
 const canvas=document.createElement('canvas');canvas.width=source.width;canvas.height=source.height;
 const ctx=canvas.getContext('2d'),ratio=canvas.width/Math.max(1,source.clientWidth);
 ctx.setTransform(ratio*(view?.scale||1),0,0,ratio*(view?.scale||1),-(view?.x||0)*source.clientWidth*ratio*(view?.scale||1),-(view?.y||0)*source.clientHeight*ratio*(view?.scale||1));
 for(const stroke of strokes||[])paintStroke(ctx,stroke,source.clientWidth,source.clientHeight);
 return canvas.toDataURL('image/png');
}
