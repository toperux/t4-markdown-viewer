use crate::config;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Clone, Debug)]
pub struct ThemeInfo {
    /// File stem, e.g. `azure-devops`. Stable id used in config.
    pub name: String,
    /// Human label for the picker, e.g. `Azure DevOps`.
    pub label: String,
    pub builtin: bool,
}

/// User-authored themes: `%APPDATA%\t4-markdown-viewer\themes`
pub fn user_dir() -> PathBuf {
    config::dir().join("themes")
}

/// Themes shipped with the app. Resolution differs between a bundled install
/// and `cargo tauri dev`, so try each plausible location.
pub fn builtin_dir(app: &AppHandle) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(res) = app.path().resource_dir() {
        candidates.push(res.join("themes"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            candidates.push(d.join("themes"));
        }
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("themes"));

    candidates.into_iter().find(|p| p.is_dir())
}

fn label_for(stem: &str) -> String {
    stem.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .map(|w| match w.to_ascii_lowercase().as_str() {
            "devops" => "DevOps".to_string(),
            "github" => "GitHub".to_string(),
            "css" => "CSS".to_string(),
            _ => {
                let mut c = w.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn scan(dir: &PathBuf, builtin: bool, out: &mut Vec<ThemeInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("css") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        // `_template.css` and friends are authoring aids, not selectable themes.
        if stem.starts_with('_') {
            continue;
        }
        let info = ThemeInfo {
            name: stem.to_string(),
            label: label_for(stem),
            builtin,
        };
        // A user theme with the same stem shadows the bundled one.
        match out.iter().position(|t| t.name == info.name) {
            Some(i) => out[i] = info,
            None => out.push(info),
        }
    }
}

pub fn list(app: &AppHandle) -> Vec<ThemeInfo> {
    let mut out = Vec::new();
    if let Some(d) = builtin_dir(app) {
        scan(&d, true, &mut out);
    }
    scan(&user_dir(), false, &mut out);
    out.sort_by_key(|t| t.label.to_lowercase());
    out
}

/// Resolve a theme name to a file, user directory taking precedence.
pub fn path_for(app: &AppHandle, name: &str) -> Option<PathBuf> {
    // Guard against `../` and absolute paths arriving from the frontend.
    if name.is_empty() || name.contains(['/', '\\', ':', '.']) {
        return None;
    }
    let file = format!("{name}.css");
    let user = user_dir().join(&file);
    if user.is_file() {
        return Some(user);
    }
    let builtin = builtin_dir(app)?.join(&file);
    builtin.is_file().then_some(builtin)
}

pub fn read(app: &AppHandle, name: &str) -> Result<String, String> {
    let path = path_for(app, name).ok_or_else(|| format!("theme '{name}' not found"))?;
    std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Directories to watch for live theme reloading. Creates the user directory so
/// that dropping a file in later is picked up without a restart.
pub fn watch_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = builtin_dir(app) {
        dirs.push(d);
    }
    let user = user_dir();
    if std::fs::create_dir_all(&user).is_ok() {
        dirs.push(user);
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::label_for;

    #[test]
    fn labels_are_prettified() {
        assert_eq!(label_for("azure-devops"), "Azure DevOps");
        assert_eq!(label_for("github-dark"), "GitHub Dark");
        assert_eq!(label_for("solarized_light"), "Solarized Light");
        assert_eq!(label_for("dracula"), "Dracula");
    }
}
