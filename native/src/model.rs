use serde::{Deserialize, Serialize};

/// Subset of the wallhaven.cc API v1 wallpaper object that Walls uses.
#[derive(Deserialize)]
pub struct Wallpaper {
    pub id: String,
    pub path: String,
    pub resolution: String,
    #[serde(default)]
    pub dimension_x: u32,
    #[serde(default)]
    pub dimension_y: u32,
    #[serde(default)]
    pub file_size: u64,
    #[serde(default)]
    pub views: u64,
    #[serde(default)]
    pub favorites: u64,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub purity: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub thumbs: Thumbs,
}

#[derive(Deserialize, Default)]
pub struct Thumbs {
    #[serde(default)]
    pub large: String,
}

#[derive(Deserialize)]
pub struct SearchMeta {
    #[serde(default)]
    pub current_page: u32,
    #[serde(default)]
    pub seed: Option<String>,
}

#[derive(Deserialize)]
pub struct SearchResponse {
    pub data: Vec<Wallpaper>,
    #[serde(default)]
    pub meta: Option<SearchMeta>,
}

#[derive(Deserialize)]
pub struct ItemResponse {
    pub data: Wallpaper,
}

/// One entry handed to the QML panel.
#[derive(Serialize)]
pub struct SearchResult {
    pub id: String,
    pub thumb: String,
    pub resolution: String,
    pub size: String,   // human-readable file size
    pub views: u64,
    pub favorites: u64,
    pub category: String,
    pub purity: String,
    pub created: String,
}

/// Full stdout payload of `walls search`.
#[derive(Serialize)]
pub struct SearchOut {
    pub results: Vec<SearchResult>,
    pub page: u32,
    pub seed: Option<String>,
}

/// A theme color mapped onto the wallhaven palette.
#[derive(Serialize)]
pub struct ThemeColor {
    pub name: String,
    pub hex: String,
    pub matched: String,
}

/// Full stdout payload of `walls theme-colors`.
#[derive(Serialize)]
pub struct ThemeColors {
    pub colors: Vec<ThemeColor>,
}
