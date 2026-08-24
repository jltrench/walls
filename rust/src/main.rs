//! walls - search wallhaven.cc wallpapers that match your Omarchy theme.
//!
//! Commands:
//!   walls search [query] [flags]   Search / browse listings
//!   walls apply <id-or-url>        Download a wallpaper and set it on the active theme
//!   walls theme-colors             Print the active theme's colors mapped to the API palette
//!   walls list                     List saved + theme wallpapers for the active theme
//!   walls set <path>               Apply a local image as background
//!   walls remove <path>            Delete a user wallpaper (backgrounds dir only)
//!   walls --version

mod api;
mod library;
mod model;
mod palette;
mod theme;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, exit};

use model::{SearchOut, SearchResult};

fn fail(msg: &str) -> ! {
    println!("{}", serde_json::json!({ "error": msg }));
    eprintln!("walls: {msg}");
    exit(1);
}

fn home() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/".into()))
}

fn current_theme() -> String {
    fs::read_to_string(home().join(".local/state/omarchy/current/theme.name"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "default".into())
}

/// User backgrounds for the active theme; created on demand. Files here join
/// the theme rotation (`omarchy theme bg next`) and survive theme re-apply.
fn backgrounds_dir() -> Result<PathBuf, String> {
    let dir = home()
        .join(".config/omarchy/backgrounds")
        .join(current_theme());
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    Ok(dir)
}

struct SearchArgs {
    query: String,
    sort: String,
    range: Option<String>,
    color: Option<String>,
    seed: Option<String>,
    page: u32,
}

fn take_value(raw: &[String], i: &mut usize, flag: &str) -> String {
    *i += 1;
    match raw.get(*i) {
        Some(v) => v.clone(),
        None => fail(&format!("{flag} needs a value")),
    }
}

impl SearchArgs {
    fn parse(raw: &[String]) -> Self {
        let mut args = SearchArgs {
            query: String::new(),
            sort: "relevance".into(),
            range: None,
            color: None,
            seed: None,
            page: 1,
        };
        let mut i = 0;
        while i < raw.len() {
            match raw[i].as_str() {
                "--sort" => args.sort = take_value(raw, &mut i, "--sort"),
                "--range" => args.range = Some(take_value(raw, &mut i, "--range")),
                "--color" => args.color = Some(take_value(raw, &mut i, "--color")),
                "--seed" => args.seed = Some(take_value(raw, &mut i, "--seed")),
                "--page" => match take_value(raw, &mut i, "--page").parse::<u32>() {
                    Ok(p) if p >= 1 => args.page = p,
                    _ => fail("--page needs a number >= 1"),
                },
                other => {
                    if !args.query.is_empty() {
                        args.query.push(' ');
                    }
                    args.query.push_str(other);
                }
            }
            i += 1;
        }
        if args.sort == "toplist" && args.range.is_none() {
            args.range = Some("1M".into());
        }
        args
    }

    fn into_query(&self) -> api::SearchQuery<'_> {
        api::SearchQuery {
            q: &self.query,
            sort: &self.sort,
            top_range: self.range.as_deref(),
            color: self.color.as_deref(),
            seed: self.seed.as_deref(),
            page: self.page,
        }
    }
}

fn cmd_search(raw: &[String]) {
    // `search` alone (no sort flag) means "latest": matches the site landing
    // behavior where an empty query shows the newest SFW wallpapers.
    let mut args = SearchArgs::parse(raw);
    if args.query.trim().is_empty() && args.sort == "relevance" && args.color.is_none() {
        args.sort = "date_added".into();
    }
    if args.query.trim().is_empty() && args.color.is_none() && args.sort == "relevance" {
        fail("usage: walls search <query> [--sort ...] [--color HEX] [--range ...] [--page N]");
    }

    let query = args.into_query();
    eprintln!(
        "walls: {} (page {})",
        query.url(),
        query.page
    );

    let response = match api::search(&query) {
        Ok(r) => r,
        Err(e) => fail(&e),
    };

    let out = SearchOut {
        results: response
            .data
            .iter()
            .map(|w| SearchResult {
                id: w.id.clone(),
                thumb: w.thumbs.large.clone(),
                resolution: w.resolution.clone(),
            })
            .collect(),
        page: response.meta.as_ref().map(|m| m.current_page).unwrap_or(args.page),
        seed: response.meta.and_then(|m| m.seed),
    };
    println!("{}", serde_json::to_string(&out).unwrap_or_else(|_| "{\"results\":[],\"page\":1,\"seed\":null}".into()));
}

fn cmd_apply(raw: &[String]) {
    let positional: Vec<&String> = raw.iter().filter(|a| !a.starts_with("--")).collect();
    let id = api::extract_id(positional.first().map(|s| s.as_str()).unwrap_or(""));
    if id.is_empty() {
        fail("usage: walls apply <id-or-url>");
    }

    let item = match api::get_item(&id) {
        Ok(i) => i,
        Err(e) => fail(&e),
    };

    let ext = item.data.path.rsplit('.').next().unwrap_or("jpg").to_lowercase();
    let dir = match backgrounds_dir() {
        Ok(d) => d,
        Err(e) => fail(&e),
    };
    let dest = dir.join(format!("wallhaven-{}.{}", item.data.id, ext));

    eprintln!(
        "walls: downloading {} ({}) for theme \"{}\"",
        item.data.path,
        item.data.resolution,
        current_theme()
    );
    if let Err(e) = api::download_to(&item.data.path, &dest) {
        let _ = fs::remove_file(&dest);
        fail(&format!("download failed: {e}"));
    }
    eprintln!("walls: saved {}", dest.display());

    let set = Command::new("omarchy")
        .args(["theme", "bg", "set"])
        .arg(&dest)
        .status();
    match set {
        Ok(s) if s.success() => {}
        Ok(s) => {
            let _ = fs::remove_file(&dest);
            fail(&format!("omarchy theme bg set exited with {s}"));
        }
        Err(e) => fail(&format!("cannot run omarchy theme bg set: {e}")),
    }

    let _ = Command::new("omarchy-notification-send")
        .args([&format!("Walls: wallpaper {} aplicado", item.data.id), "-t", "2000"])
        .status();

    println!(
        "{}",
        serde_json::json!({ "applied": dest.display().to_string(), "theme": current_theme() })
    );
}

fn cmd_theme_colors() {
    println!(
        "{}",
        serde_json::to_string(&theme::theme_colors())
            .unwrap_or_else(|_| "{\"colors\":[]}".into())
    );
}

fn cmd_list() {
    let t = current_theme();
    println!(
        "{}",
        serde_json::to_string(&library::list_wallpapers(&t)).unwrap_or_else(|_| "{\"wallpapers\":[]}".into())
    );
}

fn cmd_set(raw: &[String]) {
    let raw_path = raw.first().map(String::as_str).unwrap_or("");
    if raw_path.is_empty() {
        fail("usage: walls set <path-to-image>");
    }
    let path = PathBuf::from(raw_path);
    match library::set_background(&path) {
        Ok(()) => {
            let _ = Command::new("omarchy-notification-send")
                .args([&format!("Walls: {} aplicado", path.display()), "-t", "2000"])
                .status();
            println!("{}", serde_json::json!({ "applied": raw_path }));
        }
        Err(e) => fail(&e),
    }
}

fn cmd_remove(raw: &[String]) {
    let raw_path = raw.first().map(String::as_str).unwrap_or("");
    if raw_path.is_empty() {
        fail("usage: walls remove <path>");
    }
    let t = current_theme();
    match library::remove_wallpaper(raw_path, &t) {
        Ok((removed, was_current)) => {
            let _ = Command::new("omarchy-notification-send")
                .args([&format!("Walls: {} removido", removed.display()), "-t", "2000"])
                .status();
            println!(
                "{}",
                serde_json::json!({ "removed": removed.display().to_string(), "was_current": was_current })
            );
        }
        Err(e) => fail(&e),
    }
}

fn main() {
    let mut argv = env::args().skip(1);
    match argv.next().as_deref() {
        Some("search") => cmd_search(&argv.collect::<Vec<_>>()),
        Some("apply") => cmd_apply(&argv.collect::<Vec<_>>()),
        Some("theme-colors") => cmd_theme_colors(),
        Some("list") => cmd_list(),
        Some("set") => cmd_set(&argv.collect::<Vec<_>>()),
        Some("remove") => cmd_remove(&argv.collect::<Vec<_>>()),
        Some("--version") | Some("version") => println!("walls {}", env!("CARGO_PKG_VERSION")),
        Some(other) => fail(&format!(
            "unknown command \"{other}\"; usage: walls <search|apply|theme-colors|list|set|remove>"
        )),
        None => fail("usage: walls <search|apply|theme-colors|list|set|remove>"),
    }
}
