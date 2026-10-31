// No console window next to the app on Windows (release builds).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! HomeStats as a desktop app: the web page `homestats.html` (copied to `desktop/ui/index.html` at build time)
//! in a native window. The page detects the app (`window.__TAURI__`) and uses these commands instead of
//! browser storage and downloads: the data file in the app data folder, save dialogs and the update check.

use homestats_core::{storage, update};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// The release found by the last update check; `install_update` installs it.
struct PendingUpdate(Mutex<Option<update::ReleaseInfo>>);

fn data_file(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map(|dir| dir.join(storage::FILE_NAME)).map_err(|e| e.to_string())
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    data_path: String,
    os: &'static str,
}

#[tauri::command]
fn app_info(app: AppHandle) -> Result<AppInfo, String> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_path: data_file(&app)?.display().to_string(),
        os: std::env::consts::OS,
    })
}

#[tauri::command]
async fn load_state(app: AppHandle) -> Result<Option<storage::Loaded>, String> {
    let path = data_file(&app)?;
    blocking(move || storage::load(&path)).await?
}

#[tauri::command]
async fn save_state(app: AppHandle, json: String) -> Result<(), String> {
    let path = data_file(&app)?;
    blocking(move || storage::save(&path, &json, &storage::today())).await?
}

/// Backup and CSV exports: asks where to save (suggesting `name`), then writes `contents`. None = cancelled.
#[tauri::command]
async fn save_text_file(app: AppHandle, name: String, contents: String) -> Result<Option<String>, String> {
    let extension = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    let mut dialog = app.dialog().file().set_file_name(&name);
    if !extension.is_empty() {
        dialog = dialog.add_filter(extension.to_uppercase(), &[extension.as_str()]);
    }
    let Some(file) = blocking(move || dialog.blocking_save_file()).await? else { return Ok(None) };
    let path = file.into_path().map_err(|e| e.to_string())?;
    let shown = path.display().to_string();
    blocking(move || std::fs::write(&path, contents)).await?.map_err(|e| e.to_string())?;
    Ok(Some(shown))
}

/// Shows the data file (or its folder before the first save) in the Finder / Explorer.
#[tauri::command]
fn reveal_data(app: AppHandle) -> Result<(), String> {
    let path = data_file(&app)?;
    let target = if path.exists() { path } else { path.parent().map(PathBuf::from).ok_or("Kein Datenordner.")? };
    if !target.exists() {
        std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    }
    app.opener().reveal_item_in_dir(&target).map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfo {
    version: String,
    prerelease: bool,
    page_url: String,
}

/// Newest release on GitHub if it is newer than this app and has a package for this system.
/// Betas are offered when asked for, or while a beta is installed.
#[tauri::command]
async fn check_update(app: AppHandle, pending: State<'_, PendingUpdate>, beta: bool) -> Result<Option<UpdateInfo>, String> {
    let installed = app.package_info().version.to_string();
    let include_betas = beta || update::is_prerelease(&installed);
    let release = blocking(move || update::latest_release(include_betas)).await?.map_err(|e| e.to_string())?;
    let newer = update::is_newer(&release.version, &installed) && release.package_url.is_some();
    let info = newer.then(|| UpdateInfo { version: release.version.clone(), prerelease: release.prerelease, page_url: release.page_url.clone() });
    *pending.0.lock().map_err(|e| e.to_string())? = newer.then_some(release);
    Ok(info)
}

/// Downloads and verifies the pending release, installs it and restarts (macOS) or exits for the
/// Windows Installer, which starts the new version afterwards.
#[tauri::command]
async fn install_update(app: AppHandle, pending: State<'_, PendingUpdate>) -> Result<String, String> {
    let release = pending.0.lock().map_err(|e| e.to_string())?.clone().ok_or("Es liegt kein Update bereit.")?;
    let version = release.version.clone();
    blocking(move || update::download_package(&release).and_then(|path| update::install(&path, &release.version)))
        .await?
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    app.restart();
    #[cfg(windows)]
    app.exit(0);
    #[allow(unreachable_code)]
    Ok(format!("HomeStats {version} wurde installiert. Bitte die App neu starten."))
}

#[tauri::command]
fn open_release_page(app: AppHandle, pending: State<'_, PendingUpdate>) -> Result<(), String> {
    let url = pending.0.lock().map_err(|e| e.to_string())?.as_ref().map(|r| r.page_url.clone());
    let url = url.unwrap_or_else(|| format!("https://github.com/{}/releases", update::REPOSITORY));
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

fn main() {
    let builder = tauri::Builder::default();
    // Must be the first plugin: a second start only brings the running window to the front.
    #[cfg(windows)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }));
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(PendingUpdate(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            app_info,
            load_state,
            save_state,
            save_text_file,
            reveal_data,
            check_update,
            install_update,
            open_release_page
        ])
        .run(tauri::generate_context!())
        .expect("HomeStats konnte nicht gestartet werden");
}
