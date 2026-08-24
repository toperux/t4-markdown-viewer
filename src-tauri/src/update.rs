//! Checking GitHub for a newer release, and installing one on request.
//!
//! The whole flow lives here rather than in the webview because there is no
//! JavaScript build step in this project: `withGlobalTauri` exposes the core
//! API only, and the updater plugin's JS bindings ship as an npm package this
//! app cannot reach. Wrapping the Rust API in commands is also why no new
//! capability permissions are needed — app-defined commands are not
//! permission-gated the way plugin commands are.

use crate::AppState;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

/// What the frontend needs to describe an available release.
#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    version: String,
    /// The `notes` field of the manifest. Deliberately short — see
    /// `release_url` for why there is no changelog here.
    notes: String,
    /// False when this build cannot replace itself: a deb or rpm install is
    /// the package manager's business, and asking the plugin to update one
    /// only produces a failure further along.
    installable: bool,
    /// Where to send someone whose install cannot update itself, and where the
    /// real release notes live. `latest.json` is generated before the GitHub
    /// release exists, so the manifest can carry a link but never the body.
    release_url: String,
}

/// Only an AppImage can rewrite itself in place. The plugin sets `APPIMAGE`
/// nowhere — the AppImage runtime does — so its absence on Linux means this is
/// a deb or rpm install.
fn installable() -> bool {
    if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE").is_some()
    } else {
        true
    }
}

fn release_url() -> String {
    format!("{}/releases/latest", env!("CARGO_PKG_REPOSITORY"))
}

/// Ask whether a newer version exists.
///
/// Answered from cache after the first call, so opening three windows costs one
/// request rather than three. `force` is the Settings button, which asks even
/// when the automatic check is switched off.
#[tauri::command]
pub async fn check_for_update(app: AppHandle, force: bool) -> Result<Option<UpdateInfo>, String> {
    // Scoped so the guard is gone before the first await: holding a std Mutex
    // across one is how an async deadlock gets written by accident.
    let cached = app.state::<AppState>().update.lock().unwrap().clone();
    if cached.is_some() {
        return Ok(cached);
    }

    if !force && !crate::config::load().auto_update_check {
        return Ok(None);
    }

    let found = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;

    let Some(update) = found else {
        return Ok(None);
    };

    let info = UpdateInfo {
        version: update.version.clone(),
        notes: update.body.clone().unwrap_or_default(),
        installable: installable(),
        release_url: release_url(),
    };

    *app.state::<AppState>().update.lock().unwrap() = Some(info.clone());
    Ok(Some(info))
}

/// Download the update, install it, and restart into it. Does not return: the
/// process is replaced either by `restart` below or, on Windows, by the NSIS
/// step terminating the app as part of installing.
///
/// The `Update` handle is fetched again rather than parked in `AppState`: it is
/// one small request, and it keeps a type from a plugin's internals out of this
/// app's shared state.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "There is no update to install.".to_string())?;

    // Broadcast rather than window-scoped: one install is happening to the
    // whole app, and every open window's dialog should show the same progress.
    let downloaded = Arc::new(AtomicU64::new(0));
    let progress_app = app.clone();
    let done_app = app.clone();
    let counter = Arc::clone(&downloaded);

    update
        .download_and_install(
            move |chunk, total| {
                let done = counter.fetch_add(chunk as u64, Ordering::Relaxed) + chunk as u64;
                // A manifest without a content length gives no percentage;
                // null tells the frontend to show an indeterminate state.
                let percent = total.map(|t| (done * 100 / t.max(1)).min(100));
                let _ = progress_app.emit("update-progress", percent);
            },
            move || {
                let _ = done_app.emit("update-progress", Some(100u64));
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    app.restart()
}

#[tauri::command]
pub fn set_auto_update_check(enabled: bool) {
    let mut cfg = crate::config::load();
    cfg.auto_update_check = enabled;
    crate::config::save(&cfg);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The link the frontend opens for deb/rpm installs is built from the
    /// manifest, so a typo here would ship a dead button.
    #[test]
    fn release_url_points_at_the_releases_page() {
        assert_eq!(
            release_url(),
            "https://github.com/toperux/t4-markdown-viewer/releases/latest"
        );
    }

    /// Every platform but Linux can replace itself; on Linux it depends on how
    /// the app was installed.
    #[test]
    fn installable_everywhere_except_a_packaged_linux_install() {
        if cfg!(target_os = "linux") {
            assert_eq!(installable(), std::env::var_os("APPIMAGE").is_some());
        } else {
            assert!(installable());
        }
    }
}
