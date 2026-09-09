use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Display {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct TextData {
    pub content: String,
    pub font: String,
    pub size: f64,
    pub box_width: f64,
    pub box_height: f64,
}
impl TextData {
    pub fn valid(&self) -> bool {
        !self.content.trim().is_empty() && self.content.chars().count() <= 4096
            && valid_font(&self.font)
            && self.size.is_finite() && (0.25..=1024.).contains(&self.size)
            && [self.box_width, self.box_height].iter().all(|v| v.is_finite() && (0.01..=32768.).contains(v))
    }
}
#[cfg(test)]
mod text_tests {
    use super::*;
    #[test]
    fn text_validation_legacy_defaults_and_font_preferences() {
        let legacy = serde_json::json!({"tool":"pen","color":"#123456","width":4,"points":[{"x":0.1,"y":0.1}]});
        let old: Stroke = serde_json::from_value(legacy).unwrap();
        assert!(old.valid());assert!(old.text.is_none());
        let mut text = old.clone();text.tool="text".into();text.points.push(Point{x:0.4,y:0.2});
        text.text=Some(TextData{content:"한글 Text".into(),font:"Malgun Gothic".into(),size:24.,box_width:300.,box_height:80.});
        assert!(text.valid());
        let eraser=Stroke{points:vec![Point{x:0.25,y:0.15}],..old};
        assert!(crate::geometry::hits(&text,&eraser,1000.,800.));
        text.text.as_mut().unwrap().content=" ".into();assert!(!text.valid());
        let mut prefs: Preferences=serde_json::from_str("{}").unwrap();assert_eq!(prefs.text_font,"Malgun Gothic");
        prefs.text_font="Arial".into();assert!(prefs.valid());
        prefs.text_font="bad\nfont".into();assert!(!prefs.valid());
    }
}
pub fn valid_font(font: &str) -> bool {
    !font.trim().is_empty() && font.chars().count() <= 160 && !font.chars().any(char::is_control)
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Stroke {
    pub tool: String,
    pub color: String,
    pub width: f64,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    pub points: Vec<Point>,
    #[serde(default)]
    pub times: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<TextData>,
}
impl Stroke {
    pub fn valid(&self) -> bool {
        matches!(
            self.tool.as_str(),
            "pen" | "marker" | "fade" | "eraser" | "line" | "rectangle" | "ellipse" | "text"
        ) && self.color.len() == 7
            && self.color.starts_with('#')
            && self.color[1..].bytes().all(|c| c.is_ascii_hexdigit())
            && self.width.is_finite()
            && (0.25..=80.0).contains(&self.width)
            && self.opacity.is_finite()
            && (0.1..=1.0).contains(&self.opacity)
            && (self.times.is_empty()
                || (self.times.len() == self.points.len()
                    && self.times.windows(2).all(|t| t[0] <= t[1])
                    && self.times.last().copied().unwrap_or(0) <= 3_600_000))
            && !self.points.is_empty()
            && self.points.len() <= 100_000
            && (if self.tool == "text" {
                self.points.len() == 2 && self.text.as_ref().is_some_and(TextData::valid)
                    && self.points[1].x > self.points[0].x && self.points[1].y > self.points[0].y
            } else { self.text.is_none() })
            && self.points.iter().all(|p| {
                p.x.is_finite()
                    && p.y.is_finite()
                    && (0.0..=1.0).contains(&p.x)
                    && (0.0..=1.0).contains(&p.y)
            })
    }
}
#[derive(Default)]
pub struct History {
    pub strokes: Vec<Stroke>,
    pub undo: Vec<Edit>,
    pub redo: Vec<Edit>,
    pub recording: Vec<Recorded>,
    started: Option<std::time::Instant>,
}
#[derive(Clone)]
pub enum Edit {
    Added(Stroke),
    Moved {
        index: usize,
        before: Stroke,
        after: Stroke,
    },
    Removed(Vec<(usize, Stroke)>),
}
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Recorded {
    Fade {
        at: u64,
        stroke: Stroke,
        life: u64,
    },
    Draw {
        at: u64,
        stroke: Stroke,
    },
    Replace {
        at: u64,
        strokes: Vec<Stroke>,
    },
    Move {
        at: u64,
        duration: u64,
        index: usize,
        before: Stroke,
        after: Stroke,
    },
}
impl History {
    fn clock(&mut self, duration: u64) -> u64 {
        let now = std::time::Instant::now();
        let start = self.started.get_or_insert_with(|| {
            now.checked_sub(std::time::Duration::from_millis(duration))
                .unwrap_or(now)
        });
        now.duration_since(*start).as_millis() as u64
    }
    fn record_state(&mut self) {
        let at = self.clock(0);
        self.recording.push(Recorded::Replace {
            at,
            strokes: self.strokes.clone(),
        });
    }
    pub fn move_stroke(&mut self, index: usize, dx: f64, dy: f64, duration: u64) -> bool {
        let Some(before) = self.strokes.get(index).cloned() else {
            return false;
        };
        if !dx.is_finite() || !dy.is_finite() || duration > 3_600_000 {
            return false;
        }
        let min_x = before.points.iter().map(|p| p.x).fold(1., f64::min);
        let max_x = before.points.iter().map(|p| p.x).fold(0., f64::max);
        let min_y = before.points.iter().map(|p| p.y).fold(1., f64::min);
        let max_y = before.points.iter().map(|p| p.y).fold(0., f64::max);
        let dx = dx.clamp(-min_x, 1. - max_x);
        let dy = dy.clamp(-min_y, 1. - max_y);
        if dx.abs() + dy.abs() < 1e-9 {
            return false;
        }
        let mut after = before.clone();
        for p in &mut after.points {
            p.x += dx;
            p.y += dy;
        }
        self.strokes[index] = after.clone();
        self.undo.push(Edit::Moved {
            index,
            before: before.clone(),
            after: after.clone(),
        });
        self.redo.clear();
        let at = self.clock(duration).saturating_sub(duration);
        self.recording.push(Recorded::Move {
            at,
            duration,
            index,
            before,
            after,
        });
        true
    }

    pub fn resize_stroke(
        &mut self,
        index: usize,
        handle: &str,
        x: f64,
        y: f64,
        uniform: bool,
        duration: u64,
        w: f64,
        h: f64,
    ) -> bool {
        if !matches!(handle, "nw" | "ne" | "sw" | "se")
            || !x.is_finite()
            || !y.is_finite()
            || duration > 3_600_000
        {
            return false;
        }
        let Some(before) = self.strokes.get(index).cloned() else {
            return false;
        };
        let min_x = before.points.iter().map(|p| p.x).fold(1., f64::min);
        let max_x = before.points.iter().map(|p| p.x).fold(0., f64::max);
        let min_y = before.points.iter().map(|p| p.y).fold(1., f64::min);
        let max_y = before.points.iter().map(|p| p.y).fold(0., f64::max);
        let east = handle.ends_with('e');
        let south = handle.starts_with('s');
        let ax = if east { min_x } else { max_x };
        let ay = if south { min_y } else { max_y };
        let bw = (max_x - min_x).max(1. / w);
        let bh = (max_y - min_y).max(1. / h);
        let limit_x = (if east { 1. - ax } else { ax }) / bw;
        let limit_y = (if south { 1. - ay } else { ay }) / bh;
        let mut sx = ((x - ax) * if east { 1. } else { -1. } / bw)
            .max(2. / w / bw)
            .min(limit_x.max(0.));
        let mut sy = ((y - ay) * if south { 1. } else { -1. } / bh)
            .max(2. / h / bh)
            .min(limit_y.max(0.));
        if uniform {
            let scale = sx.max(sy).min(limit_x).min(limit_y).max(0.);
            sx = scale;
            sy = scale;
        }
        let mut after = before.clone();
        for p in &mut after.points {
            p.x = (ax + (p.x - ax) * sx).clamp(0., 1.);
            p.y = (ay + (p.y - ay) * sy).clamp(0., 1.);
        }
        after.width = (before.width * (sx * sy).sqrt()).clamp(0.25, 80.);
        if !after.valid() {
            return false;
        }
        if (after.width - before.width).abs() < 1e-9
            && after
                .points
                .iter()
                .zip(&before.points)
                .all(|(a, b)| (a.x - b.x).abs() + (a.y - b.y).abs() < 1e-9)
        {
            return false;
        }
        self.strokes[index] = after.clone();
        self.undo.push(Edit::Moved {
            index,
            before: before.clone(),
            after: after.clone(),
        });
        self.redo.clear();
        let at = self.clock(duration).saturating_sub(duration);
        self.recording.push(Recorded::Move {
            at,
            duration,
            index,
            before,
            after,
        });
        true
    }

    pub fn record_fade(&mut self, stroke: Stroke, life: u64) {
        let duration = stroke.times.last().copied().unwrap_or(0);
        let at = self.clock(duration).saturating_sub(duration);
        self.recording.push(Recorded::Fade { at, stroke, life });
    }
    pub fn push(&mut self, stroke: Stroke) {
        let duration = stroke.times.last().copied().unwrap_or(0);
        let at = self.clock(duration).saturating_sub(duration);
        self.recording.push(Recorded::Draw {
            at,
            stroke: stroke.clone(),
        });
        self.strokes.push(stroke.clone());
        self.undo.push(Edit::Added(stroke));
        self.redo.clear();
    }
    pub fn undo(&mut self) {
        if let Some(edit) = self.undo.pop() {
            match &edit {
                Edit::Moved { index, before, .. } => self.strokes[*index] = before.clone(),
                Edit::Added(_) => {
                    self.strokes.pop();
                }
                Edit::Removed(items) => {
                    for (index, stroke) in items {
                        self.strokes.insert(*index, stroke.clone());
                    }
                }
            }
            self.redo.push(edit);
            self.record_state();
        }
    }
    pub fn redo(&mut self) {
        if let Some(edit) = self.redo.pop() {
            match &edit {
                Edit::Moved { index, after, .. } => self.strokes[*index] = after.clone(),
                Edit::Added(stroke) => self.strokes.push(stroke.clone()),
                Edit::Removed(items) => {
                    for (index, _) in items.iter().rev() {
                        self.strokes.remove(*index);
                    }
                }
            }
            self.undo.push(edit);
            self.record_state();
        }
    }
    pub fn erase(&mut self, eraser: &Stroke, width: f64, height: f64) {
        let removed: Vec<_> = self
            .strokes
            .iter()
            .enumerate()
            .filter(|(_, s)| crate::geometry::hits(s, eraser, width, height))
            .map(|(i, s)| (i, s.clone()))
            .collect();
        if removed.is_empty() {
            return;
        }
        for (index, _) in removed.iter().rev() {
            self.strokes.remove(*index);
        }
        self.undo.push(Edit::Removed(removed));
        self.record_state();
        self.redo.clear();
    }
    pub fn clear(&mut self) {
        self.strokes.clear();
        self.recording.clear();
        self.started = None;
        self.undo.clear();
        self.redo.clear();
    }
}
fn default_opacity() -> f64 {
    0.3
}
#[derive(Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: String,
    pub enabled: bool,
}
pub fn default_bindings() -> std::collections::BTreeMap<String, KeyBinding> {
    [
        ("pen", "P"),
        ("fade", "F"),
        ("zoom", "Z"),
        ("zoom_reset", "0"),
        ("board", "B"),
        ("marker", "H"),
        ("eraser", "E"),
        ("select", "V"),
        ("line", "L"),
        ("rectangle", "R"),
        ("ellipse", "O"),
        ("mouse", "M"),
        ("visibility", "Tab"),
        ("undo", "Mod+Z"),
        ("redo", "Mod+Shift+Z"),
        ("capture", "Mod+Shift+S"),
        ("gif", "Mod+Shift+G"),
        ("clear", "Mod+Shift+Delete"),
        ("color1", "1"),
        ("color2", "2"),
        ("color3", "3"),
        ("color4", "4"),
        ("color5", "5"),
        ("thinner", "["),
        ("thicker", "]"),
    ]
    .into_iter()
    .map(|(id, key)| {
        (
            id.into(),
            KeyBinding {
                key: key.into(),
                enabled: true,
            },
        )
    })
    .collect()
}
fn valid_chord(chord: &str) -> bool {
    let mut rest = chord;
    for modifier in ["Mod+", "Alt+", "Shift+"] {
        if let Some(next) = rest.strip_prefix(modifier) {
            rest = next;
        }
    }
    (rest.len() == 1
        && rest
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b"[]`=,./\\;'-".contains(&b)))
        || matches!(
            rest,
            "Tab"
                | "Enter"
                | "Delete"
                | "Backspace"
                | "Space"
                | "ArrowUp"
                | "ArrowDown"
                | "ArrowLeft"
                | "ArrowRight"
                | "Home"
                | "End"
                | "PageUp"
                | "PageDown"
        )
        || rest
            .strip_prefix('F')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=12).contains(&n))
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub language: String,
    pub text_font: String,
    pub theme: String,
    pub monitor: Option<String>,
    pub layout: String,
    pub pen_width: f64,
    pub marker_width: f64,
    pub eraser_width: f64,
    pub marker_opacity: f64,
    pub shortcut: String,
    pub capture_dir: String,
    pub capture_layer_only: bool,
    pub palette: [String; 5],
    pub gif_speed: f64,
    pub gif_background: String,
    pub gif_repeat: bool,
    pub tool_shortcuts: bool,
    pub keybindings: std::collections::BTreeMap<String, KeyBinding>,
    pub global_shortcut_enabled: bool,
    pub fade_seconds: u64,
    pub spotlight_radius:f64,
    pub spotlight_dim:f64,
    pub spotlight_scale:f64,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            text_font: "Malgun Gothic".into(),
            theme: "blue".into(),
            monitor: None,
            layout: "horizontal".into(),
            pen_width: 4.,
            marker_width: 16.,
            eraser_width: 24.,
            marker_opacity: 0.3,
            shortcut: "CommandOrControl+Shift+D".into(),
            capture_dir: String::new(),
            capture_layer_only: false,
            gif_speed: 1.,
            gif_background: "white".into(),
            gif_repeat: true,
            tool_shortcuts: true,
            keybindings: default_bindings(),
            global_shortcut_enabled: true,
            fade_seconds: 3,
            spotlight_radius:120.,spotlight_dim:0.65,spotlight_scale:2.,
            palette: ["#8b5cf6", "#f43f5e", "#fbbf24", "#38bdf8", "#f8fafc"].map(String::from),
        }
    }
}
impl Preferences {
    pub fn migrate_capture_dir(&mut self, pictures: &std::path::Path, profile: Option<&std::path::Path>) {
        if self.capture_dir.is_empty() {
            let root = profile.map(|p| p.join("captures")).unwrap_or_else(|| pictures.join("Pointory"));
            self.capture_dir = root.to_string_lossy().into_owned();
            return;
        }
        let same = ["OnPen","LayerPen","InkLatch","Inklach","MonitorInk","Monitor Ink"].iter().any(|name| {
            let old=pictures.join(name);
            let same=std::path::Path::new(&self.capture_dir)==old;
            #[cfg(target_os="windows")]
            let same=same || self.capture_dir.replace('/', "\\").trim_end_matches('\\').eq_ignore_ascii_case(&old.to_string_lossy());
            same
        });
        if same {
            self.capture_dir = pictures.join("Pointory").to_string_lossy().into_owned();
        }
    }
    pub fn valid(&self) -> bool {
        matches!(self.language.as_str(), "auto" | "ko" | "en" | "ja" | "zh-CN")
            && matches!(self.theme.as_str(), "blue" | "teal" | "green" | "orange" | "purple")
            && valid_font(&self.text_font)
            && (1..=10).contains(&self.fade_seconds)
            && self.spotlight_radius.is_finite() && (40.0..=300.0).contains(&self.spotlight_radius)
            && self.spotlight_dim.is_finite() && (0.1..=0.9).contains(&self.spotlight_dim)
            && [1.,1.5,2.,3.].contains(&self.spotlight_scale)
            && [0.5, 1., 2., 4.].contains(&self.gif_speed)
            && matches!(
                self.gif_background.as_str(),
                "screen" | "white" | "dark" | "transparent"
            )
            && matches!(self.layout.as_str(), "horizontal" | "vertical")
            && self.palette.iter().all(|c| {
                c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
            })
            && [self.pen_width, self.marker_width, self.eraser_width]
                .iter()
                .all(|v| v.is_finite() && (1.0..=40.0).contains(v))
            && self.marker_opacity.is_finite()
            && (0.1..=0.8).contains(&self.marker_opacity)
            && self.valid_shortcuts()
    }
    pub fn valid_shortcuts(&self) -> bool {
        let global = self.shortcut.replace("CommandOrControl", "Mod");
        if !valid_chord(&global)
            || !(global.starts_with("Mod+")
                || global.starts_with("Alt+")
                || global.starts_with('F'))
            || self
                .shortcut
                .parse::<tauri_plugin_global_shortcut::Shortcut>()
                .is_err()
        {
            return false;
        }
        let defaults = default_bindings();
        if self.keybindings.len() != defaults.len()
            || self.keybindings.keys().any(|k| !defaults.contains_key(k))
        {
            return false;
        }
        let mut used = std::collections::HashSet::new();
        if self.global_shortcut_enabled {
            used.insert(global);
        }
        for binding in self.keybindings.values() {
            if !valid_chord(&binding.key) {
                return false;
            }
            if self.tool_shortcuts && binding.enabled && !used.insert(binding.key.clone()) {
                return false;
            }
        }
        true
    }
    pub fn width_for(&self, tool: &str) -> f64 {
        match tool {
            "marker" => self.marker_width,
            "eraser" => self.eraser_width,
            _ => self.pen_width,
        }
    }
}
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
#[derive(Clone, Serialize)]
pub struct FadingStroke {
    pub stroke: Stroke,
    pub born: u64,
    pub life: u64,
}
#[derive(Clone, Serialize)]
pub struct ZoomView {
    pub id: u64,
    pub scale: f64,
    pub x: f64,
    pub y: f64,
    #[serde(skip)]
    pub image: String,
}
pub struct Session {
    pub prefs: Preferences,
    pub displays: Vec<Display>,
    pub drawing: bool,
    pub visible: bool,
    pub tool: String,
    pub color: String,
    pub width: f64,
    pub documents: HashMap<String, History>,
    pub warning: Option<String>,
    pub capturing: bool,
    pub boards: HashMap<String, String>,
    pub fading: HashMap<String, Vec<FadingStroke>>,
    pub capture_board: String,
    pub zoom: Option<ZoomView>,
    pub zoom_return_tool: String,
    pub zoom_return_width: f64,
}
impl Session {
    pub fn new(mut prefs: Preferences) -> Self {
        // Migrate newly added mappings without discarding existing custom keys.
        for (id, mut binding) in default_bindings() {
            if !prefs.keybindings.contains_key(&id) {
                if prefs
                    .keybindings
                    .values()
                    .any(|b| b.enabled && b.key == binding.key)
                    || prefs.shortcut.replace("CommandOrControl", "Mod") == binding.key
                {
                    binding.enabled = false;
                }
                prefs.keybindings.insert(id, binding);
            }
        }

        let prefs = if prefs.valid() {
            prefs
        } else {
            Preferences {
                monitor: prefs.monitor,
                ..Preferences::default()
            }
        };
        let width = prefs.pen_width;
        Self {
            prefs,
            displays: vec![],
            drawing: false,
            visible: true,
            tool: "pen".into(),
            color: "#8b5cf6".into(),
            width,
            documents: HashMap::new(),
            warning: None,
            capturing: false,
            boards: HashMap::new(),
            fading: HashMap::new(),
            capture_board: "screen".into(),
            zoom: None,
            zoom_return_tool: "pen".into(),
            zoom_return_width: width,
        }
    }
    pub fn selected(&self) -> Option<&Display> {
        self.displays
            .iter()
            .find(|d| Some(&d.id) == self.prefs.monitor.as_ref())
    }
    pub fn board(&self) -> &str {
        self.prefs
            .monitor
            .as_ref()
            .and_then(|id| self.boards.get(id))
            .map(String::as_str)
            .unwrap_or("screen")
    }
    pub fn active_board(&self) -> &str {
        if self.drawing && self.visible {
            self.board()
        } else {
            "screen"
        }
    }
    pub fn snapshot(&self) -> serde_json::Value {
        let history = self
            .prefs
            .monitor
            .as_ref()
            .and_then(|id| self.documents.get(id));
        serde_json::json!({ "monitors": self.displays, "selected": self.prefs.monitor,
            "zoom":self.zoom,"board":self.board(), "fading":self.prefs.monitor.as_ref().and_then(|id|self.fading.get(id)).map(|items|items.iter().filter(|f|now_ms()<f.born+f.life).collect::<Vec<_>>()), "connected": self.selected().is_some(), "drawing": self.drawing, "visible": self.visible,
            "tool": self.tool, "color": self.color, "width": self.width, "preferences": self.prefs,
            "hasRecording":history.is_some_and(|h| !h.recording.is_empty()), "strokes": history.map(|h| &h.strokes), "canUndo": history.is_some_and(|h| !h.undo.is_empty()),
            "canRedo": history.is_some_and(|h| !h.redo.is_empty()), "warning": self.warning, "capturing": self.capturing })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn theme_preferences_migrate_and_roundtrip_without_changing_ink() {
        let mut prefs: super::Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(prefs.theme, "blue");
        let palette=prefs.palette.clone();
        for theme in ["blue", "teal", "green", "orange", "purple"] {
            prefs.theme=theme.into();assert!(prefs.valid());
            let loaded: super::Preferences=serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
            assert_eq!(loaded.theme,theme);assert_eq!(loaded.palette,palette);
        }
        prefs.theme="unknown".into();assert!(!prefs.valid());
    }
    #[test]
    fn capture_folder_migration_preserves_custom_paths() {
        let pictures = std::path::Path::new("/users/example/Pictures");
        let mut p = super::Preferences::default();
        p.migrate_capture_dir(pictures, None);
        assert_eq!(std::path::Path::new(&p.capture_dir), pictures.join("Pointory"));
        p.capture_dir = pictures.join("MonitorInk").to_string_lossy().into_owned();
        p.migrate_capture_dir(pictures, None);
        assert_eq!(std::path::Path::new(&p.capture_dir), pictures.join("Pointory"));
        for old in ["OnPen","LayerPen","InkLatch","Inklach","Monitor Ink"] {
            p.capture_dir=pictures.join(old).to_string_lossy().into_owned();p.migrate_capture_dir(pictures,None);
            assert_eq!(std::path::Path::new(&p.capture_dir),pictures.join("Pointory"));
        }
        p.capture_dir = pictures.join("Meetings").to_string_lossy().into_owned();
        let custom = p.capture_dir.clone();
        p.migrate_capture_dir(pictures, None);
        assert_eq!(p.capture_dir, custom);
        p.capture_dir.clear();
        p.migrate_capture_dir(pictures, Some(std::path::Path::new("/test/profile")));
        assert_eq!(std::path::Path::new(&p.capture_dir), std::path::Path::new("/test/profile/captures"));
    }
    use super::*;
    fn stroke() -> Stroke {
        Stroke {
            tool: "pen".into(),
            color: "#ff0000".into(),
            width: 4.,
            opacity: 0.3,
            points: vec![Point { x: 0.5, y: 0.5 }],
            text: None,
        times: vec![],
        }
    }
    #[test]
    fn zoom_supports_fractional_width_and_omits_image_from_snapshots() {
        let mut st = stroke();
        st.width = 2. / 6.;
        assert!(st.valid());
        let mut session = Session::new(Preferences::default());
        session.zoom = Some(ZoomView {
            id: 1,
            scale: 2.,
            x: 0.25,
            y: 0.25,
            image: "private pixels".into(),
        });
        let value = session.snapshot();
        assert_eq!(value["zoom"]["scale"], 2.);
        assert!(value["zoom"].get("image").is_none());
    }
    #[test]
    fn fade_records_without_changing_persistent_history() {
        let mut h = History::default();
        let mut st = stroke();
        st.tool = "fade".into();
        assert!(st.valid());
        h.record_fade(st, 1000);
        assert!(h.strokes.is_empty());
        assert!(h.undo.is_empty());
        assert_eq!(h.recording.len(), 1);
        h.clear();
        assert!(h.recording.is_empty());
    }
    #[test]
    fn new_shortcut_migration_preserves_custom_keys_and_disables_conflict() {
        let mut p = Preferences::default();
        p.keybindings.remove("fade");
        p.keybindings.remove("board");
        p.keybindings.get_mut("pen").unwrap().key = "F".into();
        p.palette[0] = "#123456".into();
        let s = Session::new(p);
        assert_eq!(s.prefs.palette[0], "#123456");
        assert_eq!(s.prefs.keybindings["pen"].key, "F");
        assert!(!s.prefs.keybindings["fade"].enabled);
        assert!(s.prefs.keybindings["board"].enabled);
        assert!(s.prefs.valid());
    }
    #[test]
    fn resize_records_undo_and_preserves_opposite_corner() {
        let mut h = History::default();
        let mut st = stroke();
        st.tool = "rectangle".into();
        st.points = vec![Point { x: 0.2, y: 0.2 }, Point { x: 0.4, y: 0.4 }];
        h.push(st);
        assert!(h.resize_stroke(0, "se", 0.6, 0.8, false, 100, 1000., 1000.));
        assert_eq!(h.strokes[0].points[0].x, 0.2);
        assert!((h.strokes[0].points[1].y - 0.8).abs() < 1e-9);
        assert!(h.strokes[0].width > 4.);
        h.undo();
        assert_eq!(h.strokes[0].points[1].x, 0.4);
        assert_eq!(h.strokes[0].width, 4.);
        h.redo();
        assert!(h.strokes[0].width > 4.);
        assert!(!h.resize_stroke(0, "bad", 0.5, 0.5, false, 0, 1000., 1000.));
        assert!(!h.resize_stroke(0, "se", f64::NAN, 0.5, false, 0, 1000., 1000.));
        h.undo();
        assert!(h.resize_stroke(0, "se", 0.6, 0.8, true, 100, 1000., 1000.));
        assert!((h.strokes[0].points[1].x - h.strokes[0].points[1].y).abs() < 1e-9);
    }
    #[test]
    fn shortcut_mapping_roundtrips_and_rejects_enabled_duplicates() {
        let mut p = Preferences::default();
        assert!(p.valid());
        p.keybindings.get_mut("pen").unwrap().key = "Mod+Alt+K".into();
        assert!(p.valid());
        let saved: Preferences = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(saved.keybindings["pen"].key, "Mod+Alt+K");
        p.keybindings.get_mut("marker").unwrap().key = "Mod+Alt+K".into();
        assert!(!p.valid());
        p.keybindings.get_mut("marker").unwrap().enabled = false;
        assert!(p.valid());
        p.keybindings.get_mut("pen").unwrap().key = "Mod+Shift+D".into();
        assert!(!p.valid());
        p.global_shortcut_enabled = false;
        assert!(p.valid());
        p.shortcut = "F9".into();
        assert!(p.valid());
        p.gif_background = "screen".into();
        assert!(p.valid());
        p.keybindings.get_mut("pen").unwrap().key = "Escape".into();
        assert!(!p.valid());
    }
    #[test]
    fn recording_resets_and_move_undo_redo_restore_positions() {
        let mut h = History::default();
        let mut st = stroke();
        st.times = vec![1000];
        h.push(st);
        assert_eq!(h.recording.len(), 1);
        assert!(h.move_stroke(0, 0.2, -0.2, 500));
        assert!((h.strokes[0].points[0].x - 0.7).abs() < 1e-8);
        h.undo();
        assert_eq!(h.strokes[0].points[0].x, 0.5);
        h.redo();
        assert!((h.strokes[0].points[0].x - 0.7).abs() < 1e-8);
        assert_eq!(h.recording.len(), 4);
        h.clear();
        assert!(h.recording.is_empty());
        assert!(h.started.is_none());
        assert!(h.undo.is_empty());
        assert!(h.redo.is_empty());
        h.push(stroke());
        match &h.recording[0] {
            Recorded::Draw { at, .. } => assert!(*at < 100),
            _ => panic!("expected new drawing"),
        };
    }
    #[test]
    fn invalid_timing_and_moves_are_rejected() {
        let mut st = stroke();
        st.times = vec![0, 100];
        assert!(!st.valid());
        st.times = vec![3_600_001];
        assert!(!st.valid());
        let mut h = History::default();
        h.push(stroke());
        assert!(!h.move_stroke(0, f64::NAN, 0., 0));
        assert!(!h.move_stroke(4, 0.1, 0., 0));
        assert!(h.move_stroke(0, 99., -99., 0));
        assert_eq!(h.strokes[0].points[0].x, 1.);
        assert_eq!(h.strokes[0].points[0].y, 0.);
    }
    #[test]
    fn undo_redo_branch_discards_future() {
        let mut h = History::default();
        h.push(stroke());
        h.push(stroke());
        h.undo();
        assert_eq!(h.strokes.len(), 1);
        h.redo();
        assert_eq!(h.strokes.len(), 2);
        h.undo();
        h.push(stroke());
        assert!(h.redo.is_empty());
    }
    #[test]
    fn erase_gesture_restores_original_order_and_redoes() {
        let mut h = History::default();
        for (x, color) in [(0.3, "#ff0000"), (0.1, "#00ff00"), (0.7, "#0000ff")] {
            let mut s = stroke();
            s.points[0].x = x;
            s.color = color.into();
            h.push(s);
        }
        let mut e = stroke();
        e.tool = "eraser".into();
        e.points = vec![Point { x: 0.3, y: 0.5 }, Point { x: 0.7, y: 0.5 }];
        h.erase(&e, 1000., 1000.);
        assert_eq!(h.strokes.len(), 1);
        assert_eq!(h.strokes[0].color, "#00ff00");
        h.undo();
        assert_eq!(
            h.strokes
                .iter()
                .map(|s| s.color.as_str())
                .collect::<Vec<_>>(),
            vec!["#ff0000", "#00ff00", "#0000ff"]
        );
        h.redo();
        assert_eq!(h.strokes.len(), 1);
        h.undo();
        h.undo();
        assert_eq!(h.strokes.len(), 2);
    }
    #[test]
    fn erasing_last_stroke_is_undoable_and_misses_preserve_redo() {
        let mut s = Session::new(Preferences {
            monitor: Some("a".into()),
            ..Preferences::default()
        });
        let h = s.documents.entry("a".into()).or_default();
        h.push(stroke());
        let mut e = stroke();
        e.tool = "eraser".into();
        h.erase(&e, 1000., 1000.);
        assert!(h.strokes.is_empty());
        assert_eq!(s.snapshot()["canUndo"], true);
        let h = s.documents.get_mut("a").unwrap();
        h.undo();
        e.points[0].y = 0.1;
        h.erase(&e, 1000., 1000.);
        assert_eq!(h.redo.len(), 1);
        h.redo();
        assert!(h.strokes.is_empty());
    }
    #[test]
    fn missing_monitor_does_not_fall_back() {
        let mut s = Session::new(Preferences {
            monitor: Some("unplugged".into()),
            ..Preferences::default()
        });
        s.displays.push(Display {
            id: "other".into(),
            name: "other".into(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale: 1.,
        });
        assert!(s.selected().is_none());
    }
    #[test]
    fn reject_invalid_strokes() {
        let mut s = stroke();
        assert!(s.valid());
        s.points[0].x = f64::NAN;
        assert!(!s.valid());
        s.points[0].x = 1.1;
        assert!(!s.valid());
        s.points[0].x = 0.5;
        s.width = -1.;
        assert!(!s.valid());
    }
    #[test]
    fn monitor_histories_are_isolated() {
        let mut s = Session::new(Preferences::default());
        s.documents.entry("a".into()).or_default().push(stroke());
        s.documents.entry("b".into()).or_default().push(stroke());
        s.documents.get_mut("a").unwrap().undo();
        assert_eq!(s.documents["b"].strokes.len(), 1);
    }
    #[test]
    fn old_settings_migrate_with_defaults() {
        let p: Preferences = serde_json::from_str(r#"{"monitor":"display-a"}"#).unwrap();
        assert!(p.valid());
        assert_eq!(p.monitor.as_deref(), Some("display-a"));
        assert_eq!(p.layout, "horizontal");
        assert_eq!(p.language, "auto");
        assert_eq!(p.palette.len(), 5);
        let mut custom = p.clone();
        custom.palette[0] = "#123456".into();
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&custom).unwrap()).unwrap();
        assert_eq!(restored.palette[0], "#123456");
        assert!(restored.valid());
        custom.palette[0] = "invalid".into();
        assert!(!custom.valid());
    }
    #[test]
    fn legacy_toolbar_lines_are_ignored_without_losing_preferences() {
        for layout in ["horizontal", "vertical"] {
            for lines in 1..=3 {
                let legacy = serde_json::json!({
                    "layout": layout,
                    "toolbar_lines": lines,
                    "monitor": "display-b",
                    "pen_width": 7.,
                    "text_font": "Arial",
                    "theme": "blue"
                });
                let p: Preferences = serde_json::from_value(legacy).unwrap();
                assert!(p.valid());
                assert_eq!(p.layout, layout);
                assert_eq!(p.monitor.as_deref(), Some("display-b"));
                assert_eq!(p.pen_width, 7.);
                assert_eq!(p.text_font, "Arial");
                let saved = serde_json::to_value(&p).unwrap();
                assert!(saved.get("toolbar_lines").is_none());
                let restored: Preferences = serde_json::from_value(saved).unwrap();
                assert!(restored.valid());
                assert_eq!(restored.layout, layout);
                assert_eq!(restored.pen_width, 7.);
            }
        }
    }
    #[test]
    fn language_preferences_persist_and_reject_unknown_values() {
        for language in ["auto", "ko", "en", "ja", "zh-CN"] {
            let mut p = Preferences::default();
            p.language = language.into();
            let restored: Preferences = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
            assert_eq!(restored.language, language);
            assert!(restored.valid());
        }
        let mut p = Preferences::default();
        p.language = "unknown".into();
        assert!(!p.valid());
    }
    #[test]
    fn settings_reject_invalid_ranges_and_shortcuts() {
        let mut p = Preferences::default();
        p.marker_opacity = 2.;
        assert!(!p.valid());
        p.marker_opacity = 0.4;
        p.shortcut = "D".into();
        assert!(!p.valid());
    }
}
