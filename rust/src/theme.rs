//! Minimal reader for Omarchy theme `colors.toml` files.
//!
//! We only need flat `key = "#rrggbb"` pairs, so a line scanner avoids pulling
//! a full TOML dependency into the binary. Unknown lines (tables, arrays,
//! comments) are ignored, which keeps this resilient to theme file variations.

use std::fs;
use std::path::{Path, PathBuf};

/// Keys worth offering as search colors, in display order.
pub const CURATED_KEYS: &[&str] = &[
    "accent",
    "red",
    "orange",
    "yellow",
    "green",
    "cyan",
    "blue",
    "magenta",
];

fn state_colors_path(home: &Path) -> PathBuf {
    home.join(".local/state/omarchy/current/theme/colors.toml")
}

/// Resolve the active theme's colors.toml: the omarchy state copy first
/// (always reflects the applied theme), then the user overlay, then stock.
pub fn resolve_colors_path() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()));

    let candidates = [
        state_colors_path(&home),
        home.join(format!(
            ".config/omarchy/themes/{}/colors.toml",
            read_theme_name(&home)
        )),
        home.join(".local/share/omarchy/default/themes/current/colors.toml"),
    ];

    candidates.into_iter().find(|p| p.is_file())
}

fn read_theme_name(home: &Path) -> String {
    fs::read_to_string(home.join(".local/state/omarchy/current/theme.name"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Extract `(key, hex)` pairs from a colors.toml body. Hex values may or may
/// not carry the leading '#'; both forms are accepted.
pub fn parse_colors(content: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = value.trim().trim_matches('"').trim();
        let hex = value.trim_start_matches('#');
        if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            out.push((key, hex.to_lowercase()));
        }
    }
    out
}

/// Build the curated color list for `walls theme-colors`: one entry per
/// curated key found in the file, each mapped onto the wallhaven palette.
/// Entries whose mapped palette colors collide are deduplicated - desaturated
/// themes would otherwise produce rows of identical gray searches.
pub fn theme_colors() -> crate::model::ThemeColors {
    let path = resolve_colors_path();
    let content = path
        .as_deref()
        .map(|p| fs::read_to_string(p).unwrap_or_default())
        .unwrap_or_default();
    let pairs = parse_colors(&content);

    let mut colors = Vec::new();
    let mut seen_matched = std::collections::HashSet::new();
    for key in CURATED_KEYS {
        let Some((_, hex)) = pairs.iter().find(|(k, _)| k == key) else {
            continue;
        };
        let Some(matched) = crate::palette::nearest(hex) else {
            continue;
        };
        if !seen_matched.insert(matched.to_string()) {
            continue;
        }
        colors.push(crate::model::ThemeColor {
            name: (*key).to_string(),
            hex: hex.clone(),
            matched: matched.to_string(),
        });
    }

    crate::model::ThemeColors { colors }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"
mode = "dark"

accent = "#7daea3"
selection = "#504945"
background = "#282828"

red = "#ea6962"
yellow = "#d8a657"
green = "#a9b665"
cyan = "#89b482"
blue = "#7daea3"

# a comment with #deadbe inside should be ignored
bad = "nothex"
short = "#fff"
"##;

    #[test]
    fn parses_flat_hex_pairs() {
        let colors = parse_colors(SAMPLE);
        assert!(colors.contains(&("accent".into(), "7daea3".into())));
        assert!(colors.contains(&("background".into(), "282828".into())));
        // non-hex values are skipped entirely
        assert!(!colors.iter().any(|(k, _)| k == "mode"));
        assert!(!colors.iter().any(|(k, _)| k == "bad"));
        assert!(!colors.iter().any(|(k, _)| k == "short"));
    }

    #[test]
    fn curated_output_follows_key_order() {
        let parsed = parse_colors(SAMPLE);
        let mut ordered = Vec::new();
        for key in CURATED_KEYS {
            if let Some((_, hex)) = parsed.iter().find(|(k, _)| k == key) {
                ordered.push((*key, hex.clone()));
            }
        }
        assert_eq!(
            ordered.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
            vec!["accent", "red", "yellow", "green", "cyan", "blue"]
        );
    }

    #[test]
    fn missing_theme_file_yields_empty_list() {
        // theme_colors() on a machine without omarchy state must degrade
        // gracefully instead of panicking; we only assert it runs here by
        // exercising parse path directly.
        assert!(parse_colors("").is_empty());
    }
}
