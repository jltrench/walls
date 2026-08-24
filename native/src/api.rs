//! wallhaven.cc API v1 client. SFW-only defaults, 45 req/min server limit.

use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::time::Duration;

use crate::model::{ItemResponse, SearchResponse};

const API_BASE: &str = "https://wallhaven.cc/api/v1";

pub const SORTS: &[&str] = &["relevance", "date_added", "toplist", "random", "views", "favorites", "hot"];
pub const TOP_RANGES: &[&str] = &["1d", "3d", "1w", "1M", "3M", "6M", "1y"];

pub fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(60))
        .user_agent(concat!("walls/", env!("CARGO_PKG_VERSION"), " (omarchy plugin)"))
        .build()
}

/// Everything a search request needs; rendered into the query string by
/// [`SearchQuery::url`].
pub struct SearchQuery<'a> {
    pub q: &'a str,
    pub sort: &'a str,
    pub top_range: Option<&'a str>,
    pub color: Option<&'a str>,
    pub seed: Option<&'a str>,
    pub page: u32,
}

impl Default for SearchQuery<'_> {
    fn default() -> Self {
        Self { q: "", sort: "relevance", top_range: None, color: None, seed: None, page: 1 }
    }
}

impl SearchQuery<'_> {
    pub fn url(&self) -> String {
        let mut url = format!("{API_BASE}/search?categories=100&purity=100&page={}", self.page);
        if !self.q.is_empty() {
            url.push_str("&q=");
            url.push_str(&urlencode(self.q));
        }
        url.push_str("&sorting=");
        url.push_str(&urlencode(self.sort));
        if let Some(r) = self.top_range {
            url.push_str("&topRange=");
            url.push_str(&urlencode(r));
        }
        if let Some(c) = self.color {
            url.push_str("&colors=");
            url.push_str(&urlencode(c));
        }
        if let Some(s) = self.seed {
            url.push_str("&seed=");
            url.push_str(&urlencode(s));
        }
        url
    }

    pub fn validate(&self) -> Result<(), String> {
        if !SORTS.contains(&self.sort) {
            return Err(format!("invalid --sort {}; expected one of: {}", self.sort, SORTS.join(", ")));
        }
        if let Some(r) = self.top_range {
            if !TOP_RANGES.contains(&r) {
                return Err(format!(
                    "invalid --range {r}; expected one of: {}",
                    TOP_RANGES.join(", ")
                ));
            }
        }
        if let Some(c) = self.color {
            if crate::palette::nearest(c).is_none() && !crate::palette::PALETTE.iter().any(|(hex, _)| *hex == c.to_lowercase()) {
                return Err(format!(
                    "unsupported --color {c}; see the fixed palette at https://wallhaven.cc/help/api (walls theme-colors prints valid entries)"
                ));
            }
        }
        Ok(())
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

fn describe(err: &ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, _) => match code {
            429 => "rate limited by wallhaven.cc (45 req/min) - try again shortly".into(),
            401 => "unauthorized - NSFW content requires an API key".into(),
            status => format!("HTTP {status} from wallhaven.cc"),
        },
        ureq::Error::Transport(t) => format!("network error: {t}"),
    }
}

pub fn search(query: &SearchQuery) -> Result<SearchResponse, String> {
    query.validate()?;
    let response = agent().get(&query.url()).call().map_err(|e| describe(&e))?;
    response.into_json().map_err(|e| format!("failed to parse API response: {e}"))
}

pub fn get_item(id: &str) -> Result<ItemResponse, String> {
    let url = format!("{API_BASE}/w/{id}");
    let response = agent().get(&url).call().map_err(|e| describe(&e))?;
    response.into_json().map_err(|e| format!("failed to parse item response: {e}"))
}

/// Hard cap on bytes accepted from a remote wallpaper body before the file is
/// applied or retained. Guards against a compromised API/CDN streaming an
/// unbounded response and exhausting disk.
const MAX_DOWNLOAD_BYTES: u64 = 100 * 1024 * 1024;

/// Allowed image extensions for a downloaded wallpaper, derived from the
/// API's file path. Anything else (e.g. an injected `.sh`) is rejected so a
/// compromised API/CDN cannot write an arbitrary file into the theme folder.
pub const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp"];

/// Ensure a download URL is a wallhaven asset before it is fetched. The API
/// may be compromised, so only trusted hosts are accepted.
pub fn validate_download_url(url: &str) -> Result<(), String> {
    let lower = url.trim().to_ascii_lowercase();
    let starts_https = lower.starts_with("https://");
    let starts_http = lower.starts_with("http://");
    if !starts_https && !starts_http {
        return Err("refusing to download: expected an http(s) URL".into());
    }
    // Skip the scheme; the host is the text up to the next '/', '?', or '#'.
    let after_scheme = &lower[lower.find("://").unwrap() + 3..];
    let host = after_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .split(['?', '#', '\\'])
        .next()
        .unwrap_or("")
        .to_string();
    let allowed = ["wallhaven.cc", "whvn.cc", "w.wallhaven.cc"];
    if !allowed.contains(&host.as_str()) {
        return Err(format!("refusing to download from untrusted host: {host}"));
    }
    Ok(())
}

/// Stream a wallpaper to disk. The partial file is removed on failure, and
/// downloads larger than [`MAX_DOWNLOAD_BYTES`] are rejected.
pub fn download_to(url: &str, dest: &Path) -> io::Result<()> {
    let reader = agent()
        .get(url)
        .call()
        .map_err(|e| io::Error::other(describe(&e)))?
        .into_reader();
    let mut file = fs::File::create(dest)?;
    let copied = io::copy(&mut reader.take(MAX_DOWNLOAD_BYTES + 1), &mut file)?;
    if copied > MAX_DOWNLOAD_BYTES {
        drop(file);
        let _ = fs::remove_file(dest);
        return Err(io::Error::other("download exceeds the 100 MiB size limit"));
    }
    Ok(())
}

/// Extract a wallpaper id from either a bare id or any wallhaven URL form.
pub fn extract_id(arg: &str) -> String {
    arg.trim().trim_end_matches('/').rsplit('/').next().unwrap_or(arg).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_sorted_urls() {
        let q = SearchQuery { q: "mountains", sort: "toplist", top_range: Some("1w"), page: 2, ..Default::default() };
        assert_eq!(
            q.url(),
            format!("{API_BASE}/search?categories=100&purity=100&page=2&q=mountains&sorting=toplist&topRange=1w")
        );
    }

    #[test]
    fn builds_random_url_with_seed_and_color() {
        let q = SearchQuery { sort: "random", color: Some("66cccc"), seed: Some("4ZSecs"), ..Default::default() };
        let url = q.url();
        assert!(url.contains("sorting=random"));
        assert!(url.contains("colors=66cccc"));
        assert!(url.contains("seed=4ZSecs"));
        assert!(!url.contains("q="));
    }

    #[test]
    fn rejects_bad_sort_and_range() {
        assert!(SearchQuery { sort: "nope", ..Default::default() }.validate().is_err());
        assert!(SearchQuery { sort: "toplist", top_range: Some("2d"), ..Default::default() }.validate().is_err());
        assert!(SearchQuery { sort: "toplist", top_range: Some("1y"), ..Default::default() }.validate().is_ok());
    }

    #[test]
    fn extracts_ids_from_urls() {
        assert_eq!(extract_id("abc123"), "abc123");
        assert_eq!(extract_id("https://wallhaven.cc/w/94x38z/"), "94x38z");
        assert_eq!(extract_id("http://whvn.cc/qroevq"), "qroevq");
    }

    #[test]
    fn percent_encodes_queries() {
        let q = SearchQuery { q: "dark forest", sort: "relevance", ..Default::default() };
        assert!(q.url().contains("q=dark%20forest"));
    }

    #[test]
    fn rejects_untrusted_download_hosts() {
        assert!(validate_download_url("https://wallhaven.cc/wallpapers/full/5/gp/94x38z.jpg").is_ok());
        assert!(validate_download_url("https://w.wallhaven.cc/full/94/wallhaven-94x38z.jpg").is_ok());
        assert!(validate_download_url("http://whvn.cc/x.jpg").is_ok());
        assert!(validate_download_url("https://evil.example.com/x.jpg").is_err());
        assert!(validate_download_url("file:///etc/passwd").is_err());
        assert!(validate_download_url("wallhaven.cc/x.jpg").is_err());
        assert!(validate_download_url("").is_err());
    }
}
