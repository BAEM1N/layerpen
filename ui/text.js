// Text remains editable geometry for move/resize/undo and renders into PNG/GIF.
export const fontSizeForPen = width => 8 + 4 * width;
export const canvasFont = (font,size) => `${size}px ${JSON.stringify(font)}, sans-serif`;
export function textStroke(ctx,{content,font,width,color,point,canvasWidth,canvasHeight,scale=1}){
  if(!content.trim())return null;
  const size=fontSizeForPen(width)/scale;
  ctx.save();ctx.font=canvasFont(font,size);
  const maxWidth=Math.max(1,(1-point.x)*canvasWidth-2/scale),lines=[];
  for(const paragraph of content.slice(0,4096).replaceAll('\r','').split('\n')){
    let line='';
    for(const char of paragraph){
      if(line&&ctx.measureText(line+char).width>maxWidth){lines.push(line);line=char;}else line+=char;
    }
    lines.push(line);
  }
  const boxWidth=Math.max(1,...lines.map(line=>ctx.measureText(line).width))+2/scale;
  const boxHeight=lines.length*size*1.25;
  ctx.restore();
  return {tool:'text',color,width:width/scale,opacity:1,times:[],
    points:[point,{x:Math.min(1,point.x+boxWidth/canvasWidth),y:Math.min(1,point.y+boxHeight/canvasHeight)}],
    text:{content:lines.join('\n'),font,size,box_width:boxWidth,box_height:boxHeight}};
}
export function paintText(ctx,stroke,width,height){
  const text=stroke.text;if(!text)return;
  const [a,b]=stroke.points;
  ctx.translate(a.x*width,a.y*height);
  ctx.scale((b.x-a.x)*width/text.box_width,(b.y-a.y)*height/text.box_height);
  ctx.font=canvasFont(text.font,text.size);ctx.textBaseline='top';
  text.content.split('\n').forEach((line,i)=>ctx.fillText(line,0,i*text.size*1.25));
}
