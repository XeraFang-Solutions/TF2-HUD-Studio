//! Best-effort first-image resolution for screenshot albums. No page scripts are executed.
use regex::Regex;
use url::Url;

pub fn raw_github(value: &str) -> String {
    let Ok(mut url) = Url::parse(value) else { return value.into() };
    if url.host_str() == Some("github.com") && url.path().contains("/blob/") {
        let path = url.path().replacen("/blob/", "/", 1);
        let _ = url.set_host(Some("raw.githubusercontent.com"));
        url.set_path(&path);
        url.set_query(None);
    }
    url.to_string()
}

fn resolve(base: &str, value: &str) -> Option<String> {
    let value = value.replace("&amp;", "&").replace("&#38;", "&");
    let url = Url::parse(base).ok()?.join(&value).ok()?;
    if url.scheme() != "https" {
        return None;
    }
    Some(raw_github(url.as_str()))
}

pub fn direct_image(url: &str) -> bool {
    Url::parse(url).ok().is_some_and(|u| {
        let path = u.path().to_ascii_lowercase();
        [".png", ".jpg", ".jpeg", ".webp", ".gif"].iter().any(|ext| path.ends_with(ext))
    })
}

/// Prefer the actual first album item in embedded Imgur state, then album social metadata.
pub fn first_image(base: &str, text: &str) -> Option<String> {
    let host = Url::parse(base).ok()?.host_str()?.to_string();
    if host == "imgur.com" || host == "www.imgur.com" {
        // Albums may expose images/media JSON in a script, or as escaped JSON in a hydration string.
        let unescaped = text.replace("\\\"", "\"").replace("\\/", "/");
        if let Some(start) = Regex::new(r#""(?:images|media)"\s*:\s*\["#).unwrap().find(&unescaped) {
            let mut json = serde_json::Deserializer::from_str(&unescaped[start.end() - 1..]).into_iter::<serde_json::Value>();
            if let Some(Ok(items)) = json.next() {
                if let Some(first) = items.as_array().and_then(|items| items.first()) {
                    let first = first.get("image").unwrap_or(first);
                    if let Some(value) = first.get("url").or_else(|| first.get("link")).and_then(|v| v.as_str()) {
                        if value.starts_with("https://i.imgur.com/") && direct_image(value) {
                            return resolve(base, value);
                        }
                    }
                    if let (Some(hash), Some(ext)) = (first.get("hash").and_then(|v| v.as_str()), first.get("ext").and_then(|v| v.as_str())) {
                        if hash.chars().all(|c| c.is_ascii_alphanumeric()) && matches!(ext, ".png" | ".jpg" | ".jpeg" | ".gif" | ".webp") {
                            return Some(format!("https://i.imgur.com/{hash}{ext}"));
                        }
                    }
                }
            }
        }
    }
    if host == "raw.githubusercontent.com" {
        // Combined expression preserves document order between Markdown and HTML images.
        let image = Regex::new(r#"(?is)!\[[^\]]*\]\(\s*<?([^\s)>]+)>?(?:\s+[^)]*)?\)|<img\b[^>]*?\bsrc\s*=\s*["']([^"']+)["']"#).unwrap();
        for c in image.captures_iter(text) {
            if let Some(value) = c.get(1).or_else(|| c.get(2)) {
                if let Some(url) = resolve(base, value.as_str()) {
                    return Some(url);
                }
            }
        }
        // Markdown reference-style image: ![caption][first] followed by [first]: image.png
        let reference = Regex::new(r"!\[([^\]]*)\]\[([^\]]*)\]").unwrap();
        if let Some(c) = reference.captures(text) {
            let label = if c[2].is_empty() { &c[1] } else { &c[2] };
            for line in text.lines() {
                if let Some(value) = line.trim().strip_prefix(&format!("[{label}]:")) {
                    return resolve(base, value.split_whitespace().next()?.trim_matches(['<', '>']));
                }
            }
        }
    }
    // Most hosted albums expose the first image as og:image (including Imgur's server-rendered pages).
    let meta = Regex::new(r"(?is)<meta\b[^>]*>").unwrap();
    let attr = Regex::new(r#"(?i)\b(property|name|content)\s*=\s*["']([^"']+)["']"#).unwrap();
    for tag in meta.find_iter(text) {
        let mut property = String::new();
        let mut content = String::new();
        for c in attr.captures_iter(tag.as_str()) {
            if c[1].eq_ignore_ascii_case("content") {
                content = c[2].into();
            } else {
                property = c[2].to_ascii_lowercase();
            }
        }
        if matches!(property.as_str(), "og:image" | "twitter:image" | "twitter:image:src") {
            if let Some(url) = resolve(base, &content) {
                if host.contains("imgur.com") && !url.starts_with("https://i.imgur.com/") {
                    continue;
                }
                return Some(url);
            }
        }
    }
    None
}

pub fn image_type(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("image/png", "png"))
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some(("image/jpeg", "jpg"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(("image/gif", "gif"))
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some(("image/webp", "webp"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn github_showcase_uses_first_image_and_resolves_relative_paths() {
        let base = raw_github("https://github.com/TF2HUDsArchive/hud/blob/screenshots/showcase.md");
        assert_eq!(base, "https://raw.githubusercontent.com/TF2HUDsArchive/hud/screenshots/showcase.md");
        assert_eq!(
            first_image(&base, "# Screenshots\n![first](./images/one.png)\n![second](two.jpg)"),
            Some("https://raw.githubusercontent.com/TF2HUDsArchive/hud/screenshots/images/one.png".into())
        );
        assert!(first_image(&base, "<img src='data:image/svg+xml,bad'>").is_none());
        assert_eq!(
            first_image(&base, "<img src='first.jpg'>\n![later](later.png)"),
            Some("https://raw.githubusercontent.com/TF2HUDsArchive/hud/screenshots/first.jpg".into())
        );
    }
    #[test]
    fn imgur_prefers_first_item_over_cover_and_reads_metadata() {
        let page = r#"<meta property="og:image" content="https://i.imgur.com/cover.jpg"><script>{"media":[{"url":"https://i.imgur.com/first.png"},{"url":"https://i.imgur.com/second.png"}]}</script>"#;
        assert_eq!(first_image("https://imgur.com/a/test", page), Some("https://i.imgur.com/first.png".into()));
        assert_eq!(
            first_image("https://imgur.com/a/test", r#"<script>{"images":[{"hash":"first","ext":".png"},{"link":"https://i.imgur.com/second.jpg"}]}</script>"#),
            Some("https://i.imgur.com/first.png".into())
        );
        assert_eq!(
            first_image("https://imgur.com/a/test", r#"<meta content='https://i.imgur.com/one.jpg?x=1&amp;y=2' property='og:image'>"#),
            Some("https://i.imgur.com/one.jpg?x=1&y=2".into())
        );
        assert!(first_image("https://imgur.com/a/test", "<html>Javascript required</html>").is_none());
        assert!(image_type(b"<html>not an image</html>").is_none());
    }
}
