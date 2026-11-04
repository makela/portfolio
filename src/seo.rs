//! robots.txt, sitemap.xml and the JSON-LD structured data.

use std::fmt::Write;

use axum::{
    extract::State,
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::IntoResponse,
};

use crate::{
    assets,
    site::{self, Config},
};

pub(crate) async fn robots(State(config): State<Config>) -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "text/plain; charset=utf-8"),
            (CACHE_CONTROL, "no-cache"),
        ],
        robots_txt(&config),
    )
}

pub(crate) async fn sitemap(State(config): State<Config>) -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "application/xml; charset=utf-8"),
            (CACHE_CONTROL, "no-cache"),
        ],
        sitemap_xml(&config),
    )
}

fn robots_txt(config: &Config) -> String {
    format!(
        "User-agent: *\nAllow: /\n\nSitemap: {}\n",
        config.absolute("/sitemap.xml")
    )
}

fn sitemap_xml(config: &Config) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         \x20 <url><loc>{}</loc></url>\n\
         </urlset>\n",
        xml_escape(&config.absolute("/"))
    )
}

/// schema.org `Person` for the page owner, as JSON-LD.
pub(crate) fn person_json_ld(config: &Config) -> String {
    let mut json = String::from("{\"@context\":\"https://schema.org\",\"@type\":\"Person\"");
    let _ = write!(
        json,
        ",\"name\":{},\"url\":{},\"image\":{}",
        json_string(site::NAME),
        json_string(&config.absolute("/")),
        json_string(&config.absolute(&assets::url(site::OG_IMAGE))),
    );
    if !site::SAME_AS.is_empty() {
        let profiles: Vec<String> = site::SAME_AS.iter().map(|url| json_string(url)).collect();
        let _ = write!(json, ",\"sameAs\":[{}]", profiles.join(","));
    }
    json.push('}');
    json
}

/// JSON string literal. `<`, `>` and `&` are escaped too, so the result is
/// safe inside a `<script>` element (no `</script>` can appear).
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '<' | '>' | '&' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            site_url: "https://portfolio.example.com".to_string(),
        }
    }

    #[test]
    fn robots_points_to_sitemap() {
        assert!(
            robots_txt(&config()).contains("Sitemap: https://portfolio.example.com/sitemap.xml")
        );
    }

    #[test]
    fn sitemap_lists_home_page() {
        assert!(sitemap_xml(&config()).contains("<loc>https://portfolio.example.com/</loc>"));
    }

    #[test]
    fn json_ld_is_safe_inside_script() {
        assert_eq!(
            json_string("</script>\"&"),
            r#""\u003c/script\u003e\"\u0026""#
        );
    }

    #[test]
    fn json_ld_describes_person() {
        let json = person_json_ld(&config());
        assert!(json.starts_with(r#"{"@context":"https://schema.org","@type":"Person""#));
        assert!(json.contains(r#""url":"https://portfolio.example.com/""#));
    }
}
