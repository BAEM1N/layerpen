use crate::model::Preferences;
use std::{ffi::OsString, fs, io, path::{Path, PathBuf}};

pub fn override_directory(mut read: impl FnMut(&str) -> Option<OsString>) -> Option<PathBuf> {
    ["POINTORY_DATA_DIR", "ONPEN_DATA_DIR", "MONITOR_INK_DATA_DIR"]
        .iter().find_map(|key| read(key)).map(PathBuf::from)
}

pub fn directories(config: &Path, app_config: &Path, custom: Option<PathBuf>) -> (PathBuf, Vec<PathBuf>) {
    if let Some(custom) = custom { return (custom, vec![]); }
    let mut legacy: Vec<_> = ["OnPen", "LayerPen", "InkLatch", "Inklach", "MonitorInk", "Monitor Ink"]
        .iter().map(|name| config.join(name)).collect();
    if !legacy.iter().any(|path| path == app_config) { legacy.push(app_config.to_owned()); }
    (config.join("Pointory"), legacy)
}

fn copy_missing(source: &Path, destination: &Path) -> io::Result<()> {
    let mut source = fs::File::open(source)?;
    match fs::OpenOptions::new().write(true).create_new(true).open(destination) {
        Ok(mut target) => { io::copy(&mut source, &mut target)?; Ok(()) }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

fn migrate_fonts(source: &Path, destination: &Path) -> io::Result<()> {
    let entries = match fs::read_dir(source.join("fonts")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let id = path.file_stem().and_then(|name| name.to_str()).unwrap_or("");
        if entry.file_type()?.is_file() && path.extension().is_some_and(|ext| ext == "ttf")
            && id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            let target = destination.join("fonts");
            fs::create_dir_all(&target)?;
            copy_missing(&path, &target.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Copy only missing data, leaving every legacy file intact for rollback.
/// The unchanged bundle identifier preserves WebView storage and caption preferences.
pub fn load(directory: &Path, legacy: &[PathBuf]) -> io::Result<Preferences> {
    let destination = directory.join("settings.json");
    let mut preferences = match fs::read(&destination) {
        Ok(bytes) => Some(serde_json::from_slice::<Preferences>(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    for source in legacy {
        migrate_fonts(source, directory)?;
        if preferences.is_some() { continue; }
        let source_settings = source.join("settings.json");
        let bytes = match fs::read(&source_settings) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        // A damaged legacy file must not prevent recovery from an older profile.
        if let Ok(prefs) = serde_json::from_slice::<Preferences>(&bytes) {
            fs::create_dir_all(directory)?;
            copy_missing(&source_settings, &destination)?;
            preferences = Some(prefs);
        }
    }
    let mut preferences = preferences.unwrap_or_default();
    if let Some(id) = preferences.text_font.strip_prefix("OnPenFont-") {
        if id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            preferences.text_font = format!("PointoryFont-{id}");
        }
    }
    Ok(preferences)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("pointory-profile-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap(); Self(path)
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    #[test]
    fn upgrade_preserves_preferences_imported_fonts_and_original_files() {
        for name in ["OnPen", "LayerPen", "MonitorInk", "dev.personal.monitorink"] {
            let fixture = Fixture::new(); let old = fixture.0.join(name);
            let id = "a".repeat(64); let font_name = format!("{id}.ttf");
            fs::create_dir_all(old.join("fonts")).unwrap();
            fs::write(old.join("fonts").join(&font_name), b"legacy font bytes").unwrap();
            let bytes = serde_json::to_vec(&serde_json::json!({"theme":"teal", "layout":"vertical",
                "capture_dir":"D:/Teaching/screens", "pen_width":7,
                "text_font":format!("OnPenFont-{id}")})).unwrap();
            fs::write(old.join("settings.json"), &bytes).unwrap();
            let (target, legacy) = directories(&fixture.0, &fixture.0.join("dev.personal.monitorink"), None);
            let prefs = load(&target, &legacy).unwrap();
            assert_eq!(prefs.theme, "teal"); assert_eq!(prefs.layout, "vertical");
            assert_eq!(prefs.pen_width, 7.); assert_eq!(prefs.capture_dir, "D:/Teaching/screens");
            assert_eq!(prefs.text_font, format!("PointoryFont-{id}"));
            assert_eq!(fs::read(target.join("fonts").join(&font_name)).unwrap(), b"legacy font bytes");
            assert_eq!(fs::read(old.join("settings.json")).unwrap(), bytes);
            assert_eq!(fs::read(target.join("settings.json")).unwrap(), bytes);
            assert!(old.join("fonts").join(&font_name).is_file());
        }
    }

    #[test]
    fn current_preferences_and_fonts_win_over_legacy_data() {
        let fixture = Fixture::new(); let old = fixture.0.join("OnPen");
        let target = fixture.0.join("Pointory"); let font_name = format!("{}.ttf", "b".repeat(64));
        for (folder, theme) in [(&target, "orange"), (&old, "teal")] {
            fs::create_dir_all(folder.join("fonts")).unwrap();
            fs::write(folder.join("settings.json"), format!("{{\"theme\":\"{theme}\"}}")).unwrap();
            fs::write(folder.join("fonts").join(&font_name), theme).unwrap();
        }
        assert_eq!(load(&target, &[old]).unwrap().theme, "orange");
        assert_eq!(fs::read(target.join("fonts").join(font_name)).unwrap(), b"orange");
    }

    #[test]
    fn profile_override_priority_and_isolation_remain_compatible() {
        for (index, expected) in ["POINTORY_DATA_DIR", "ONPEN_DATA_DIR", "MONITOR_INK_DATA_DIR"].iter().enumerate() {
            let keys = ["POINTORY_DATA_DIR", "ONPEN_DATA_DIR", "MONITOR_INK_DATA_DIR"];
            let custom = override_directory(|key| keys[index..].contains(&key).then(|| OsString::from(key)));
            assert_eq!(custom.as_deref(), Some(Path::new(expected)));
            let (target, legacy) = directories(Path::new("config"), Path::new("app-config"), custom);
            assert_eq!(target, Path::new(expected)); assert!(legacy.is_empty());
        }
    }

    #[test]
    fn corrupt_current_preferences_are_never_overwritten_with_defaults() {
        let fixture = Fixture::new(); fs::write(fixture.0.join("settings.json"), b"broken json").unwrap();
        assert!(load(&fixture.0, &[]).is_err());
        assert_eq!(fs::read(fixture.0.join("settings.json")).unwrap(), b"broken json");
    }
}
