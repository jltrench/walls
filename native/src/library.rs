//! Local wallpaper library for the active theme.
//!
//! Two sources feed the Saved tab: wallpapers downloaded by Walls (removable)
//! and the theme's own stock backgrounds (viewable/applyable only). Removal
//! is hard-restricted to the user backgrounds directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "webp"];

#[derive(Serialize)]
pub struct WallpaperFile {
    pub path: String,
    pub name: String,
    pub source: String, // "user" | "theme"
    pub is_current: bool,
}

#[derive(Serialize)]
pub struct ListOut {
    pub theme: String,
    pub current: Option<String>,
    pub wallpapers: Vec<WallpaperFile>,
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

pub fn user_backgrounds_dir(theme: &str) -> PathBuf {
    home().join(".config/omarchy/backgrounds").join(theme)
}

pub fn theme_backgrounds_dir() -> PathBuf {
    home().join(".local/state/omarchy/current/theme/backgrounds")
}

pub fn current_background_link() -> Option<PathBuf> {
    fs::read_link(home().join(".local/state/omarchy/current/background")).ok()
}

fn has_image_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn collect_dir(dir: &Path, source: &str, current: Option<&Path>, out: &mut Vec<WallpaperFile>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || !has_image_ext(&path) {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        out.push(WallpaperFile {
            path: path.to_string_lossy().to_string(),
            is_current: current.map(|c| c == &path).unwrap_or(false),
            source: source.to_string(),
            name,
        });
    }
}

/// Build the Saved-tab listing: user wallpapers first (name asc), then the
/// theme's stock backgrounds.
pub fn list_wallpapers(theme: &str) -> ListOut {
    let current = current_background_link();
    let mut wallpapers = Vec::new();

    collect_dir(&user_backgrounds_dir(theme), "user", current.as_deref(), &mut wallpapers);
    wallpapers.sort_by(|a, b| a.name.cmp(&b.name));

    let user_count = wallpapers.len();
    collect_dir(&theme_backgrounds_dir(), "theme", current.as_deref(), &mut wallpapers);

    // Keep the user section sorted; stock section appended after it sorted too.
    wallpapers[user_count..].sort_by(|a, b| a.name.cmp(&b.name));

    ListOut {
        theme: theme.to_string(),
        current: current.map(|c| c.to_string_lossy().to_string()),
        wallpapers,
    }
}

/// Validate that a path may be removed: it must resolve inside the user
/// backgrounds directory of the active theme. Stock theme files and anything
/// outside are refused.
pub fn validate_removable(path: &str, theme: &str) -> Result<PathBuf, String> {
    let target = fs::canonicalize(path)
        .map_err(|_| format!("file not found: {path}"))?;
    let allowed = user_backgrounds_dir(theme);
    let allowed = fs::canonicalize(&allowed)
        .unwrap_or(allowed);
    if !target.starts_with(&allowed) {
        return Err(format!(
            "refusing to remove {}: only files inside {} are managed by Walls",
            target.display(),
            allowed.display()
        ));
    }
    Ok(target)
}

/// Apply a local file as the current background via omarchy.
pub fn set_background(path: &Path) -> Result<(), String> {
    if !path.is_file() || !has_image_ext(path) {
        return Err(format!("not a wallpaper file: {}", path.display()));
    }
    let status = Command::new("omarchy")
        .args(["theme", "bg", "set"])
        .arg(path)
        .status()
        .map_err(|e| format!("cannot run omarchy theme bg set: {e}"))?;
    if !status.success() {
        return Err(format!("omarchy theme bg set exited with {status}"));
    }
    Ok(())
}

/// Remove a user wallpaper. When it was the current background, cycle to the
/// next one afterwards so the symlink never dangles.
pub fn remove_wallpaper(path: &str, theme: &str) -> Result<(PathBuf, bool), String> {
    let target = validate_removable(path, theme)?;
    let was_current = current_background_link()
        .map(|c| c == target)
        .unwrap_or(false);
    fs::remove_file(&target).map_err(|e| format!("cannot remove {}: {e}", target.display()))?;
    if was_current {
        let _ = Command::new("omarchy").args(["theme", "bg", "next"]).status();
    }
    Ok((target, was_current))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_image(dir: &Path, name: &str) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let p = dir.join(name);
        fs::write(&p, b"fake").unwrap();
        p
    }

    #[test]
    fn lists_user_and_theme_sections_sorted() {
        let tmp = std::env::temp_dir().join(format!("walls-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let _b = write_image(&tmp.join("user"), "b.jpg");
        let _a = write_image(&tmp.join("user"), "a.png");
        let _s = write_image(&tmp.join("theme"), "s.jpg");
        write_image(&tmp.join("theme"), "notes.txt"); // non-image: ignored

        let mut out = Vec::new();
        collect_dir(&tmp.join("user"), "user", None, &mut out);
        out.sort_by(|x, y| x.name.cmp(&y.name));
        let user_count = out.len();
        collect_dir(&tmp.join("theme"), "theme", None, &mut out);
        out[user_count..].sort_by(|x, y| x.name.cmp(&y.name));

        assert_eq!(out.len(), 3);
        assert_eq!(out[0].name, "a.png");
        assert_eq!(out[1].name, "b.jpg");
        assert_eq!(out[2].name, "s.jpg");
        assert_eq!(out[0].source, "user");
        assert_eq!(out[2].source, "theme");
        assert!(!out[0].is_current);

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn marks_current_background() {
        let tmp = std::env::temp_dir().join(format!("walls-cur-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let p = write_image(&tmp.join("user"), "cur.jpg");

        let mut out = Vec::new();
        collect_dir(&tmp.join("user"), "user", Some(&p), &mut out);
        assert!(out[0].is_current);

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn remove_validation_blocks_outside_paths() {
        // Anything outside the user backgrounds dir must be refused, including
        // stock theme files and traversal attempts.
        assert!(validate_removable("/etc/passwd", "gruvbox").is_err());
        assert!(validate_removable(
            &theme_backgrounds_dir().join("whatever.jpg").to_string_lossy(),
            "gruvbox"
        )
        .is_err());
        assert!(validate_removable("", "gruvbox").is_err());
    }
}
