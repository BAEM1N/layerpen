use crate::model::Stroke;
type P = (f64, f64);
fn path(s: &Stroke, w: f64, h: f64) -> Vec<P> {
    let first = &s.points[0];
    let last = s.points.last().unwrap();
    let a = (first.x * w, first.y * h);
    let b = (last.x * w, last.y * h);
    match s.tool.as_str() {
        "line" => vec![a, b],
        "rectangle" | "text" => vec![a, (b.0, a.1), b, (a.0, b.1), a],
        "ellipse" => {
            let rx = (b.0 - a.0).abs() / 2.;
            let ry = (b.1 - a.1).abs() / 2.;
            let n = ((std::f64::consts::PI * (rx.max(ry) / 0.5).sqrt()).ceil() as usize)
                .clamp(32, 1024);
            (0..=n)
                .map(|i| {
                    let t = i as f64 * std::f64::consts::TAU / n as f64;
                    (
                        (a.0 + b.0) / 2. + rx * t.cos(),
                        (a.1 + b.1) / 2. + ry * t.sin(),
                    )
                })
                .collect()
        }
        _ => s.points.iter().map(|p| (p.x * w, p.y * h)).collect(),
    }
}
fn cross(a: P, b: P, c: P) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}
fn point_distance(p: P, a: P, b: P) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let length = dx * dx + dy * dy;
    let t = if length == 0. {
        0.
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0., 1.)
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}
fn near(a: P, b: P, c: P, d: P, r: f64) -> bool {
    if a.0.max(b.0) + r < c.0.min(d.0)
        || c.0.max(d.0) + r < a.0.min(b.0)
        || a.1.max(b.1) + r < c.1.min(d.1)
        || c.1.max(d.1) + r < a.1.min(b.1)
    {
        return false;
    }
    let crossed = cross(a, b, c) * cross(a, b, d) < 0. && cross(c, d, a) * cross(c, d, b) < 0.;
    crossed
        || point_distance(a, c, d)
            .min(point_distance(b, c, d))
            .min(point_distance(c, a, b))
            .min(point_distance(d, a, b))
            <= r
}
pub fn hits(stroke: &Stroke, eraser: &Stroke, w: f64, h: f64) -> bool {
    if stroke.points.is_empty() || eraser.points.is_empty() {
        return false;
    }
    let a = path(stroke, w, h);
    let b = path(eraser, w, h);
    let r = (stroke.width + eraser.width) / 2.;
    if stroke.tool == "text" && b.iter().any(|p| p.0 >= a[0].0.min(a[2].0)-r && p.0 <= a[0].0.max(a[2].0)+r && p.1 >= a[0].1.min(a[2].1)-r && p.1 <= a[0].1.max(a[2].1)+r) {
        return true;
    }
    for i in 0..a.len().saturating_sub(1).max(1) {
        for j in 0..b.len().saturating_sub(1).max(1) {
            if near(
                a[i],
                a[(i + 1).min(a.len() - 1)],
                b[j],
                b[(j + 1).min(b.len() - 1)],
                r,
            ) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Point;
    fn stroke(tool: &str, coords: &[(f64, f64)]) -> Stroke {
        Stroke {
            text: None,
        times: vec![],
            tool: tool.into(),
            color: "#ff0000".into(),
            width: 4.,
            opacity: 0.3,
            points: coords.iter().map(|&(x, y)| Point { x, y }).collect(),
        }
    }
    #[test]
    fn shape_borders_not_empty_interiors() {
        for tool in ["rectangle", "ellipse"] {
            let s = stroke(tool, &[(0.2, 0.2), (0.8, 0.8)]);
            assert!(!hits(&s, &stroke("eraser", &[(0.5, 0.5)]), 1000., 1000.));
            assert!(hits(&s, &stroke("eraser", &[(0.8, 0.5)]), 1000., 1000.));
        }
    }
    #[test]
    fn fast_crossings_and_dot_radius_use_logical_pixels() {
        let line = stroke("line", &[(0.2, 0.5), (0.8, 0.5)]);
        assert!(hits(
            &line,
            &stroke("eraser", &[(0.5, 0.1), (0.5, 0.9)]),
            1000.,
            1000.
        ));
        let dot = stroke("pen", &[(0.5, 0.5)]);
        let e = stroke("eraser", &[(0.503, 0.5)]);
        assert!(hits(&dot, &e, 1000., 1000.));
        assert!(!hits(&dot, &e, 2000., 1000.));
    }
}
