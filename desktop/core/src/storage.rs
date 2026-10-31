//! The data file of the desktop app: `homestats.json` in the app data folder (the same JSON as the
//! backup the web page downloads). Every save first keeps the previous file as `homestats.json.bak`
//! and, once a day, a copy in `Sicherungen/` (the newest 30 stay); the new content is written to a
//! temporary file and renamed, so a crash never leaves half a file behind.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const FILE_NAME: &str = "homestats.json";
const BACKUP_DIR: &str = "Sicherungen";
const KEEP_DAILY: usize = 30;

#[derive(Debug, PartialEq, Serialize)]
pub struct Loaded {
    pub text: String,
    /// The data file was missing or damaged and the `.bak` copy was loaded instead.
    pub recovered: bool,
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

fn read_valid(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<serde_json::Value>(&text).ok().filter(|v| v.is_object())?;
    Some(text)
}

/// The saved data; `None` on the first start. A damaged file without a usable backup is an error, so
/// the app does not start empty and overwrite it.
pub fn load(path: &Path) -> Result<Option<Loaded>, String> {
    if let Some(text) = read_valid(path) {
        return Ok(Some(Loaded { text, recovered: false }));
    }
    if let Some(text) = read_valid(&backup_path(path)) {
        return Ok(Some(Loaded { text, recovered: true }));
    }
    if path.exists() {
        return Err(format!("{} ist beschädigt und es gibt keine lesbare Sicherung.", path.display()));
    }
    Ok(None)
}

/// Saves `json` (must be a JSON object). `today` (YYYY-MM-DD) names the daily copy.
pub fn save(path: &Path, json: &str, today: &str) -> Result<(), String> {
    if !serde_json::from_str::<serde_json::Value>(json).is_ok_and(|v| v.is_object()) {
        return Err("Ungültige Daten – nicht gespeichert.".into());
    }
    let dir = path.parent().ok_or("Kein Ordner für die Datendatei.")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    // Only an intact previous file becomes the backup, so a damaged one never replaces a good copy.
    if read_valid(path).is_some() {
        fs::copy(path, backup_path(path)).map_err(|e| e.to_string())?;
        daily_copy(path, dir, today);
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// First save of the day: copies the previous file to `Sicherungen/homestats-<today>.json`, keeps the newest 30.
/// A failing daily copy does not stop the save.
fn daily_copy(path: &Path, dir: &Path, today: &str) {
    let backups = dir.join(BACKUP_DIR);
    let target = backups.join(format!("homestats-{today}.json"));
    if target.exists() || fs::create_dir_all(&backups).is_err() || fs::copy(path, &target).is_err() {
        return;
    }
    let mut files: Vec<PathBuf> = fs::read_dir(&backups)
        .map(|entries| entries.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.retain(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("homestats-") && n.ends_with(".json")));
    files.sort();
    let extra = files.len().saturating_sub(KEEP_DAILY);
    for old in &files[..extra] {
        let _ = fs::remove_file(old);
    }
}

/// Today's date in UTC as YYYY-MM-DD (days-to-civil algorithm by Howard Hinnant).
pub fn today() -> String {
    let days = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64 / 86_400).unwrap_or(0);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("homestats-test-{name}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn first_start_has_no_data() {
        let path = temp_dir("first").join(FILE_NAME);
        assert_eq!(load(&path), Ok(None));
    }

    #[test]
    fn saves_keeps_backup_and_daily_copy() {
        let dir = temp_dir("save");
        let path = dir.join(FILE_NAME);
        save(&path, r#"{"version":3,"n":1}"#, "2026-10-30").unwrap();
        save(&path, r#"{"version":3,"n":2}"#, "2026-10-31").unwrap();
        save(&path, r#"{"version":3,"n":3}"#, "2026-10-31").unwrap();
        assert_eq!(load(&path).unwrap().unwrap().text, r#"{"version":3,"n":3}"#);
        assert_eq!(fs::read_to_string(backup_path(&path)).unwrap(), r#"{"version":3,"n":2}"#);
        // The daily copy holds the state before the first save of that day.
        assert_eq!(fs::read_to_string(dir.join("Sicherungen/homestats-2026-10-31.json")).unwrap(), r#"{"version":3,"n":1}"#);
        assert!(!dir.join(FILE_NAME).with_extension("json.tmp").exists());
    }

    #[test]
    fn refuses_invalid_json_and_recovers_from_backup() {
        let dir = temp_dir("recover");
        let path = dir.join(FILE_NAME);
        assert!(save(&path, "kaputt", "2026-10-31").is_err());
        assert!(save(&path, "[1,2]", "2026-10-31").is_err());
        save(&path, r#"{"n":1}"#, "2026-10-31").unwrap();
        save(&path, r#"{"n":2}"#, "2026-10-31").unwrap();
        fs::write(&path, "{halb").unwrap();
        assert_eq!(load(&path), Ok(Some(Loaded { text: r#"{"n":1}"#.into(), recovered: true })));
        // A damaged file does not replace the good backup on the next save.
        save(&path, r#"{"n":3}"#, "2026-10-31").unwrap();
        assert_eq!(fs::read_to_string(backup_path(&path)).unwrap(), r#"{"n":1}"#);
    }

    #[test]
    fn damaged_file_without_backup_is_an_error() {
        let path = temp_dir("damaged").join(FILE_NAME);
        fs::write(&path, "{halb").unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn keeps_only_the_newest_daily_copies() {
        let dir = temp_dir("prune");
        let path = dir.join(FILE_NAME);
        save(&path, r#"{"n":0}"#, "2026-01-01").unwrap();
        for day in 1..=35 {
            save(&path, &format!(r#"{{"n":{day}}}"#), &format!("2026-02-{day:02}")).unwrap();
        }
        let count = fs::read_dir(dir.join("Sicherungen")).unwrap().count();
        assert_eq!(count, KEEP_DAILY);
        assert!(!dir.join("Sicherungen/homestats-2026-02-01.json").exists());
    }

    #[test]
    fn today_is_a_date() {
        let t = today();
        assert_eq!(t.len(), 10);
        assert!(t.starts_with("20"));
    }
}
