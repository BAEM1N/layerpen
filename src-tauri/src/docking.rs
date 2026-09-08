// Physical coordinates: supports negative monitor origins and mixed DPI.
#[derive(Clone,Copy,Debug)]
pub struct Rect {pub x:i32,pub y:i32,pub w:i32,pub h:i32}
pub fn position(toolbar:Rect, area:Rect, width:i32,height:i32,gap:i32)->(i32,i32){
 let right=(toolbar.x+toolbar.w+gap,toolbar.y);
 let left=(toolbar.x-width-gap,toolbar.y);
 let below=(toolbar.x,toolbar.y+toolbar.h+gap);
 let above=(toolbar.x,toolbar.y-height-gap);
 let choices=if toolbar.h>toolbar.w {[right,left,below,above]}else{[below,above,right,left]};
 let (x,y)=choices.into_iter().find(|&(x,y)|x>=area.x&&y>=area.y&&x+width<=area.x+area.w&&y+height<=area.y+area.h).unwrap_or(choices[0]);
 (x.clamp(area.x,(area.x+area.w-width).max(area.x)),y.clamp(area.y,(area.y+area.h-height).max(area.y)))
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn dock_avoids_edges_and_negative_origins(){
 let a=Rect{x:-1920,y:0,w:1920,h:1040};
 assert_eq!(position(Rect{x:-130,y:50,w:112,h:620},a,520,660,8),(-658,50));
 assert_eq!(position(Rect{x:-1800,y:880,w:640,h:122},a,520,660,8),(-1800,212));
 let (x,y)=position(Rect{x:500,y:500,w:112,h:620},Rect{x:0,y:0,w:640,h:480},620,460,8);assert_eq!((x,y),(20,20));
 }
}

// 31 cells, 36 px buttons, 5 px gaps, 20 px padding/border, margins + caption.
pub fn toolbar_size(vertical:bool,lines:u8)->(f64,f64){let cross=lines.clamp(1,3) as u32;let long=31_u32.div_ceil(cross);let extent=|n:u32|(n*41+15) as f64;let (cols,rows)=if vertical{(cross,long)}else{(long,cross)};(extent(cols)+12.,extent(rows)+28.)}
#[cfg(test)]mod layout_tests{use super::*;#[test]fn all_six_layouts(){for lines in 1..=3{let (w,h)=toolbar_size(false,lines);let (vw,vh)=toolbar_size(true,lines);assert_eq!(w-12.,vh-28.);assert_eq!(h-28.,vw-12.);assert!(w>0.&&h>0.);}}}
