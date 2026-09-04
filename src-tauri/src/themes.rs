use crate::config;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Which side of a family's light/dark toggle a theme sits on, and the palette
/// the browser draws its own widgets with.
///
/// `Light` is declared first on purpose: `list` orders a family's members by
/// this, so the light one comes out ahead of the dark one.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Light,
    Dark,
}

#[derive(Serialize, Clone, Debug)]
pub struct ThemeInfo {
    /// File stem, e.g. `azure-devops`. Stable id used in config.
    pub name: String,
    /// Human label for the picker, e.g. `Azure DevOps`.
    pub label: String,
    pub builtin: bool,
    /// The family this theme belongs to: `github` for both `github-light` and
    /// `github-dark`. **Not** a theme id — it never goes to `path_for`.
    pub group: String,
    /// Human label for the family, e.g. `GitHub`.
    pub group_label: String,
    pub mode: Mode,
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

/// The family a stem belongs to: the stem with its first `light`/`dark` token
/// dropped. Whole tokens only, so `darkroom` and `highlight` keep theirs, and
/// separators normalise to `-` so `foo_dark` and `foo-dark` are one family.
fn group_key_for(stem: &str) -> String {
    let mut dropped = false;
    let kept: Vec<&str> = stem
        .split(['-', '_'])
        .filter(|s| !s.is_empty())
        .filter(|w| {
            let is_mode = w.eq_ignore_ascii_case("light") || w.eq_ignore_ascii_case("dark");
            // Only the first one goes: `light-dark` is a family named `dark`,
            // not an empty string.
            let drop = is_mode && !dropped;
            dropped |= drop;
            !drop
        })
        .collect();
    // A theme actually called `dark.css` keeps its name rather than vanishing.
    if kept.is_empty() {
        return stem.to_string();
    }
    kept.join("-")
}

/// CSS with `/* … */` comments blanked out, so a scan cannot be fooled by prose.
/// Every bundled theme writes the words `color-scheme` in a comment, and the
/// themes README writes it with a colon.
fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        out.push(' ');
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            // Unterminated comment: the rest of the file is comment.
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// The theme's own `color-scheme` declaration, if it makes a choice. Later
/// declarations win, as the cascade would have it, and `normal` is not a choice.
///
/// Only top-level rules count — `:root { … }`, which is where the README says
/// to put it. A declaration nested one level deeper is inside an at-rule, and
/// the common one is `@media (prefers-color-scheme: dark)`: reading that would
/// report a light theme as dark purely because it also adapts to the OS.
fn mode_from_css(css: &str) -> Option<Mode> {
    const PROP: &str = "color-scheme";
    let cleaned = strip_comments(css).to_ascii_lowercase();
    let mut found = None;
    let mut depth = 0usize;

    for (i, c) in cleaned.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            'c' if depth == 1 && cleaned[i..].starts_with(PROP) => {
                // Reject `--color-scheme` and `--my-color-scheme`: a custom
                // property is not the real thing.
                if cleaned[..i]
                    .chars()
                    .next_back()
                    .is_some_and(|p| p == '-' || p == '_' || p.is_alphanumeric())
                {
                    continue;
                }
                let Some(value) = cleaned[i + PROP.len()..].trim_start().strip_prefix(':') else {
                    continue;
                };
                let value = &value[..value.find([';', '}']).unwrap_or(value.len())];
                // `light dark` means light first; `only dark` means dark.
                if let Some(word) = value
                    .split_whitespace()
                    .find(|w| *w == "light" || *w == "dark")
                {
                    found = Some(if word == "dark" {
                        Mode::Dark
                    } else {
                        Mode::Light
                    });
                }
            }
            _ => {}
        }
    }
    found
}

/// The stylesheet decides; the name only speaks when the stylesheet is silent.
/// A theme that says nothing either way is light, which is what a browser draws
/// when no `color-scheme` is declared.
fn mode_for(stem: &str, css: Option<&str>) -> Mode {
    if let Some(mode) = css.and_then(mode_from_css) {
        return mode;
    }
    let named = stem
        .split(['-', '_'])
        .find(|w| w.eq_ignore_ascii_case("light") || w.eq_ignore_ascii_case("dark"));
    match named {
        Some(w) if w.eq_ignore_ascii_case("dark") => Mode::Dark,
        _ => Mode::Light,
    }
}

/// Fill in `group`/`group_label`. A family that would hold two themes of the
/// same mode cannot answer "the other side of this one", so it is not formed at
/// all: its members are listed individually under their own labels. Nothing is
/// ever hidden — this only ever regroups.
fn assign_groups(themes: &mut [ThemeInfo]) {
    let mut keys: Vec<String> = themes.iter().map(|t| group_key_for(&t.name)).collect();

    // Demoting a family's members can land one of them on a key another family
    // is already using, so settle rather than assume one pass does it. A key
    // only ever moves to the theme's own stem and stems are unique, so this
    // cannot oscillate and stops within one round in every realistic case.
    loop {
        // The whole family goes, not just the members that clash. Dropping only
        // those would leave the family name on whichever theme's stem already
        // spelled it: adding a `solarized.css` would quietly evict the bundled
        // `solarized-light` from Solarized and stand in for it, changing what
        // the picker says for someone who never touched that theme.
        let mut ambiguous: Vec<String> = Vec::new();
        for (i, a) in themes.iter().enumerate() {
            for (j, b) in themes.iter().enumerate().skip(i + 1) {
                if keys[i] == keys[j] && a.mode == b.mode && !ambiguous.contains(&keys[i]) {
                    ambiguous.push(keys[i].clone());
                }
            }
        }
        if ambiguous.is_empty() {
            break;
        }
        for (i, theme) in themes.iter().enumerate() {
            if ambiguous.contains(&keys[i]) {
                keys[i] = theme.name.clone();
            }
        }
    }

    for (theme, key) in themes.iter_mut().zip(keys) {
        theme.group_label = label_for(&key);
        theme.group = key;
    }
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
        // A theme that cannot be read still belongs in the picker — a locked
        // file should cost it the right mode, not its place in the list.
        let css = std::fs::read_to_string(&path).ok();
        let info = ThemeInfo {
            name: stem.to_string(),
            label: label_for(stem),
            builtin,
            // Filled in by `assign_groups` once the whole set is known.
            group: String::new(),
            group_label: String::new(),
            mode: mode_for(stem, css.as_deref()),
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
    assign_groups(&mut out);
    // Sorted by family, and within one, light before dark: the picker shows a
    // family once and relies on its members being adjacent. `group` breaks a
    // tie on the label, because two keys can share one — `Solarized-Light.css`
    // beside `solarized-light.css` on a case-sensitive filesystem — and without
    // it those two families would interleave.
    out.sort_by(|a, b| {
        (a.group_label.to_lowercase(), &a.group, a.mode, &a.name).cmp(&(
            b.group_label.to_lowercase(),
            &b.group,
            b.mode,
            &b.name,
        ))
    });
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
    use super::*;

    fn theme(name: &str, mode: Mode) -> ThemeInfo {
        ThemeInfo {
            name: name.to_string(),
            label: label_for(name),
            builtin: true,
            group: String::new(),
            group_label: String::new(),
            mode,
        }
    }

    /// The 15 bundled themes and the mode each declares.
    fn bundled() -> Vec<ThemeInfo> {
        use Mode::{Dark, Light};
        [
            ("azure-devops", Light),
            ("azure-devops-blue", Light),
            ("azure-devops-dark", Dark),
            ("azure-devops-dark-blue", Dark),
            ("dracula", Dark),
            ("dracula-blue", Dark),
            ("dracula-green", Dark),
            ("github-dark", Dark),
            ("github-dark-blue", Dark),
            ("github-light", Light),
            ("github-light-blue", Light),
            ("sakura", Light),
            ("solarized-dark", Dark),
            ("solarized-light", Light),
            ("tufte", Light),
        ]
        .into_iter()
        .map(|(n, m)| theme(n, m))
        .collect()
    }

    #[test]
    fn labels_are_prettified() {
        assert_eq!(label_for("azure-devops"), "Azure DevOps");
        assert_eq!(label_for("github-dark"), "GitHub Dark");
        assert_eq!(label_for("solarized_light"), "Solarized Light");
        assert_eq!(label_for("dracula"), "Dracula");
    }

    #[test]
    fn group_key_drops_the_mode_token() {
        assert_eq!(group_key_for("github-dark"), "github");
        assert_eq!(group_key_for("azure-devops-dark-blue"), "azure-devops-blue");
        assert_eq!(group_key_for("solarized_light"), "solarized");
        assert_eq!(group_key_for("dracula"), "dracula");
    }

    #[test]
    fn group_key_only_drops_whole_tokens() {
        assert_eq!(group_key_for("darkroom"), "darkroom");
        assert_eq!(group_key_for("lightning"), "lightning");
        assert_eq!(group_key_for("highlight"), "highlight");
    }

    #[test]
    fn group_key_never_becomes_empty() {
        assert_eq!(group_key_for("dark"), "dark");
        assert_eq!(group_key_for("light"), "light");
        // Only the first token goes, so this is a family called `dark`.
        assert_eq!(group_key_for("light-dark"), "dark");
    }

    #[test]
    fn bundled_stems_form_the_expected_groups() {
        let mut themes = bundled();
        assign_groups(&mut themes);

        let group_of = |name: &str| {
            themes
                .iter()
                .find(|t| t.name == name)
                .map(|t| t.group.clone())
                .unwrap()
        };

        assert_eq!(group_of("azure-devops"), "azure-devops");
        assert_eq!(group_of("azure-devops-dark"), "azure-devops");
        assert_eq!(group_of("azure-devops-blue"), "azure-devops-blue");
        assert_eq!(group_of("azure-devops-dark-blue"), "azure-devops-blue");
        assert_eq!(group_of("github-light"), "github");
        assert_eq!(group_of("github-dark"), "github");
        assert_eq!(group_of("github-light-blue"), "github-blue");
        assert_eq!(group_of("github-dark-blue"), "github-blue");
        assert_eq!(group_of("solarized-light"), "solarized");
        assert_eq!(group_of("solarized-dark"), "solarized");
        assert_eq!(group_of("dracula"), "dracula");
        assert_eq!(group_of("sakura"), "sakura");
        assert_eq!(group_of("tufte"), "tufte");

        let mut groups: Vec<&str> = themes.iter().map(|t| t.group.as_str()).collect();
        groups.sort_unstable();
        groups.dedup();
        assert_eq!(groups.len(), 10, "expected ten families, got {groups:?}");
    }

    #[test]
    fn bundled_group_labels_are_prettified() {
        let mut themes = bundled();
        assign_groups(&mut themes);
        let label_of = |name: &str| {
            themes
                .iter()
                .find(|t| t.name == name)
                .map(|t| t.group_label.clone())
                .unwrap()
        };
        assert_eq!(label_of("azure-devops-dark"), "Azure DevOps");
        assert_eq!(label_of("github-dark-blue"), "GitHub Blue");
        assert_eq!(label_of("solarized-light"), "Solarized");
    }

    #[test]
    fn mode_is_read_from_color_scheme() {
        assert_eq!(
            mode_from_css(":root { color-scheme: dark; }"),
            Some(Mode::Dark)
        );
        assert_eq!(
            mode_from_css(":root{color-scheme:light;}"),
            Some(Mode::Light)
        );
        assert_eq!(
            mode_from_css(":root { color-scheme :  DARK ; }"),
            Some(Mode::Dark)
        );
        // A declaration outside any rule is not a declaration.
        assert_eq!(mode_from_css("color-scheme: dark;"), None);
        assert_eq!(mode_from_css(":root { color: #fff; }"), None);
    }

    #[test]
    fn mode_ignores_color_scheme_inside_comments() {
        // The comment every bundled theme carries, plus a hostile one.
        let css = "/* `var()` does not resolve there and `color-scheme` is ignored, so */\n\
                   /* color-scheme: dark */\n\
                   :root { color-scheme: light; }";
        assert_eq!(mode_from_css(css), Some(Mode::Light));
    }

    #[test]
    fn mode_ignores_custom_properties() {
        assert_eq!(mode_from_css(":root { --color-scheme: dark; }"), None);
    }

    #[test]
    fn mode_takes_the_first_keyword_and_the_last_declaration() {
        assert_eq!(
            mode_from_css(":root { color-scheme: light dark; }"),
            Some(Mode::Light)
        );
        assert_eq!(
            mode_from_css(":root { color-scheme: only dark; }"),
            Some(Mode::Dark)
        );
        assert_eq!(mode_from_css(":root { color-scheme: normal; }"), None);
        assert_eq!(
            mode_from_css(":root { color-scheme: light; color-scheme: dark; }"),
            Some(Mode::Dark)
        );
    }

    #[test]
    fn css_mode_overrides_the_name_token() {
        assert_eq!(
            mode_for("foo-dark", Some(":root { color-scheme: light; }")),
            Mode::Light
        );
    }

    #[test]
    fn mode_falls_back_to_the_name_token() {
        assert_eq!(mode_for("foo-dark", None), Mode::Dark);
        assert_eq!(mode_for("foo-light", Some(":root{}")), Mode::Light);
    }

    #[test]
    fn mode_defaults_to_light_when_nothing_says() {
        assert_eq!(mode_for("mytheme", Some(":root{}")), Mode::Light);
        assert_eq!(mode_for("mytheme", None), Mode::Light);
    }

    #[test]
    fn grouping_never_loses_a_theme() {
        let mut themes = bundled();
        themes.push(theme("foo", Mode::Light));
        themes.push(theme("foo-light", Mode::Light));
        themes.push(theme("x-dark", Mode::Dark));
        themes.push(theme("x-dark-dark", Mode::Dark));

        let before: Vec<String> = themes.iter().map(|t| t.name.clone()).collect();
        assign_groups(&mut themes);
        let after: Vec<String> = themes.iter().map(|t| t.name.clone()).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn a_family_that_would_hide_a_theme_is_not_formed() {
        // Both light, so the family cannot answer "the other side of this one".
        let mut themes = vec![theme("foo", Mode::Light), theme("foo-light", Mode::Light)];
        assign_groups(&mut themes);
        assert_eq!(themes[0].group, "foo");
        assert_eq!(themes[1].group, "foo-light");
        assert_eq!(themes[1].group_label, "Foo Light");
    }

    #[test]
    fn every_group_holds_at_most_one_theme_per_mode() {
        let mut themes = bundled();
        themes.push(theme("foo", Mode::Light));
        themes.push(theme("foo-light", Mode::Light));
        themes.push(theme("x-dark", Mode::Dark));
        themes.push(theme("x-dark-dark", Mode::Dark));
        assign_groups(&mut themes);

        for (i, a) in themes.iter().enumerate() {
            for b in themes.iter().skip(i + 1) {
                assert!(
                    a.group != b.group || a.mode != b.mode,
                    "{} and {} share group {} and mode {:?}",
                    a.name,
                    b.name,
                    a.group,
                    a.mode
                );
            }
        }
    }

    #[test]
    fn an_ambiguous_family_dissolves_entirely() {
        // `azure-devops` is the light half but is named without a token, so an
        // `azure-devops-light` wants the same family and the same side. The
        // family cannot say what "the other side" is any more, so all three
        // stand alone rather than two of them keeping the name.
        let mut themes = bundled();
        themes.push(theme("azure-devops-light", Mode::Light));
        assign_groups(&mut themes);

        let group_of = |name: &str| {
            themes
                .iter()
                .find(|t| t.name == name)
                .map(|t| t.group.clone())
                .unwrap()
        };
        assert_eq!(group_of("azure-devops"), "azure-devops");
        assert_eq!(group_of("azure-devops-light"), "azure-devops-light");
        assert_eq!(group_of("azure-devops-dark"), "azure-devops-dark");
    }

    #[test]
    fn a_new_theme_cannot_evict_a_bundled_one_from_its_family() {
        // The reverse orientation: here the newcomer's stem *is* the family key,
        // so a rule that only demoted the clashing members would leave the
        // newcomer standing in for `solarized-light` under the Solarized name.
        let mut themes = bundled();
        themes.push(theme("solarized", Mode::Light));
        assign_groups(&mut themes);

        let group_of = |name: &str| {
            themes
                .iter()
                .find(|t| t.name == name)
                .map(|t| t.group.clone())
                .unwrap()
        };
        assert_eq!(group_of("solarized"), "solarized");
        assert_eq!(group_of("solarized-light"), "solarized-light");
        assert_eq!(group_of("solarized-dark"), "solarized-dark");
        // Untouched families are unaffected by the collision next door.
        assert_eq!(group_of("github-light"), "github");
        assert_eq!(group_of("github-dark"), "github");
    }

    #[test]
    fn mode_ignores_declarations_nested_in_at_rules() {
        // A light theme that also adapts to the OS preference is still light.
        let css = ":root { color-scheme: light; }\n\
                   @media (prefers-color-scheme: dark) {\n\
                     :root { color-scheme: dark; }\n\
                   }";
        assert_eq!(mode_from_css(css), Some(Mode::Light));
    }

    #[test]
    fn stripping_comments_keeps_non_ascii_intact() {
        assert_eq!(strip_comments("a /* x */ é"), "a   é");
        assert_eq!(
            mode_from_css("/* é */ :root { color-scheme: dark; }"),
            Some(Mode::Dark)
        );
    }

    #[test]
    fn group_keys_introduce_no_path_characters() {
        // `path_for`'s guard, applied to every group key a bundled stem yields.
        let mut themes = bundled();
        assign_groups(&mut themes);
        for t in &themes {
            assert!(!t.group.is_empty());
            assert!(
                !t.group.contains(['/', '\\', ':', '.']),
                "group key {} would not survive path_for",
                t.group
            );
        }
    }

    #[test]
    fn default_theme_is_the_dark_half_of_a_two_mode_group() {
        let mut themes = bundled();
        assign_groups(&mut themes);
        let default = themes
            .iter()
            .find(|t| t.name == config::DEFAULT_THEME)
            .expect("the default theme must be one of the bundled ones");
        assert_eq!(default.mode, Mode::Dark);
        let partner = themes
            .iter()
            .find(|t| t.group == default.group && t.mode == Mode::Light);
        assert!(
            partner.is_some(),
            "the default theme must have a light partner for the toggle"
        );
    }
}
