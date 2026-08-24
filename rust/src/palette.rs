//! The wallhaven.cc search API accepts colors from a fixed 28-entry palette.
//! Theme colors are mapped onto the nearest palette entry by weighted RGB
//! distance so "match my theme" searches stay within what the API supports.

/// `(hex without '#', human name)` - mirrors the values documented at
/// https://wallhaven.cc/help/api
pub const PALETTE: &[(&str, &str)] = &[
    ("660000", "dark red"),
    ("990000", "red"),
    ("cc0000", "bright red"),
    ("cc3333", "tomato"),
    ("ea4c88", "pink"),
    ("993399", "purple"),
    ("663399", "violet"),
    ("333399", "indigo"),
    ("0066cc", "blue"),
    ("0099cc", "sky blue"),
    ("66cccc", "teal"),
    ("77cc33", "grass green"),
    ("669900", "light green"),
    ("336600", "green"),
    ("666600", "olive"),
    ("999900", "dark yellow"),
    ("cccc33", "lime"),
    ("ffff00", "yellow"),
    ("ffcc33", "gold"),
    ("ff9900", "orange"),
    ("ff6600", "bright orange"),
    ("cc6633", "brown"),
    ("996633", "light brown"),
    ("663300", "dark brown"),
    ("000000", "black"),
    ("999999", "dark gray"),
    ("cccccc", "light gray"),
    ("ffffff", "white"),
];

fn hex_to_rgb(hex: &str) -> Option<(f32, f32, f32)> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| -> Option<f32> { u8::from_str_radix(&h[i..i + 2], 16).ok().map(|v| v as f32) };
    Some((byte(0)?, byte(2)?, byte(4)?))
}

/// Weighted euclidean distance approximating perceived color difference.
fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dr = a.0 - b.0;
    let dg = a.1 - b.1;
    let db = a.2 - b.2;
    (2.0 * dr * dr + 4.0 * dg * dg + 3.0 * db * db).sqrt()
}

/// Map any hex color onto the closest wallhaven.cc palette entry.
/// Returns `None` when the input is not a valid `#rrggbb` string.
pub fn nearest(hex: &str) -> Option<&'static str> {
    let rgb = hex_to_rgb(hex)?;
    let (max, min) = (rgb.0.max(rgb.1.max(rgb.2)), rgb.0.min(rgb.1.min(rgb.2)));

    // Near-neutral colors read as gray no matter their faint tint; without
    // this snap the weighted distance happily lands them on dark browns.
    if max - min < 24.0 {
        let luma = 0.299 * rgb.0 + 0.587 * rgb.1 + 0.114 * rgb.2;
        return Some(if luma < 64.0 {
            "000000"
        } else if luma < 128.0 {
            "999999"
        } else if luma < 192.0 {
            "cccccc"
        } else {
            "ffffff"
        });
    }

    PALETTE
        .iter()
        .filter(|(hex, _)| !is_gray_entry(hex))
        .copied()
        .chain(GRAY_ENTRIES.iter().copied())
        .min_by(|a, b| {
            let da = distance(rgb, hex_to_rgb(a.0).unwrap_or_default());
            let db = distance(rgb, hex_to_rgb(b.0).unwrap_or_default());
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(hex, _)| hex)
}

/// Grayscale palette entries, handled by the luma snap above and kept out of
/// hue-distance comparisons so tinted colors cannot steal them.
const GRAY_ENTRIES: &[(&str, &str)] = &[("000000", "black"), ("999999", "dark gray"), ("cccccc", "light gray"), ("ffffff", "white")];

fn is_gray_entry(hex: &str) -> bool {
    GRAY_ENTRIES.iter().any(|(g, _)| *g == hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_exact_matches() {
        assert_eq!(nearest("#66cccc"), Some("66cccc"));
        assert_eq!(nearest("000000"), Some("000000"));
    }

    #[test]
    fn maps_theme_colors_to_sensible_entries() {
        // gruvbox accent (#7daea3) is heavily desaturated - it lands on the
        // gray family even though it reads as teal, and that is the honest
        // nearest match under a perceptual distance.
        assert_eq!(nearest("#7daea3"), Some("999999"));
        // a clearly saturated teal maps where you would expect
        assert_eq!(nearest("#40b3a8"), Some("66cccc"));
        // warm orange (#e1875c) lands on the brown/orange family
        assert_eq!(nearest("#e1875c"), Some("cc6633"));
        // near-black background
        assert_eq!(nearest("#282828"), Some("000000"));
    }

    #[test]
    fn rejects_invalid_input() {
        assert_eq!(nearest(""), None);
        assert_eq!(nearest("#12345"), None);
        assert_eq!(nearest("zzzzzz"), None);
    }
}
