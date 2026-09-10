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

// 36 px controls + 2 px gaps; grip, padding and four fixed controls use 210 px.
pub fn toolbar_size(vertical: bool, item_count: usize) -> (f64, f64) {
    let length = item_count.min(crate::model::TOOLBAR_ITEMS.len()) as f64 * 38. + 210.;
    if vertical { (64., length) } else { (length, 64.) }
}
pub fn toolbar_panel_size(vertical: bool, open: bool, item_count: usize) -> (f64, f64) {
    let (width, height) = toolbar_size(vertical, item_count);
    let extra = if open { 280. } else { 0. };
    if vertical { (width + extra, height) } else { (if open { width.max(280.) } else { width }, height + extra) }
}
#[cfg(test)]
mod layout_tests {
    use super::*;
    #[test]
    fn two_orientations_keep_a_single_strip_and_expand_only_across_it() {
        assert_eq!(toolbar_size(false, 14), (742., 64.));
        assert_eq!(toolbar_size(true, 14), (64., 742.));
        assert_eq!(toolbar_panel_size(false, true, 14), (742., 344.));
        assert_eq!(toolbar_panel_size(true, true, 14), (344., 742.));
        for vertical in [false, true] {
            assert_eq!(toolbar_panel_size(vertical, false, 14), toolbar_size(vertical, 14));
        }
    }
    #[test]
    fn minimal_toolbar_retains_fixed_controls_and_panel_space() {
        assert_eq!(toolbar_size(false, 0), (210., 64.));
        assert_eq!(toolbar_size(true, 0), (64., 210.));
        assert_eq!(toolbar_panel_size(false, true, 0), (280., 344.));
        assert_eq!(toolbar_panel_size(true, true, 0), (344., 210.));
        assert_eq!(toolbar_size(false, 18), (894., 64.));
        assert_eq!(toolbar_size(false, usize::MAX), (894., 64.));
    }
}
