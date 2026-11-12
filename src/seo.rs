//! robots.txt, sitemap.xml and the JSON-LD structured data.

use std::fmt::Write;

use axum::{
    extract::State,
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::IntoResponse,
};

use crate::{
    assets, json, resume,
    site::{self, Config},
};

/// Pages listed in the sitemap.
const PAGES: &[&str] = &["/", "/resume"];

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
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for path in PAGES {
        let _ = writeln!(
            xml,
            "  <url><loc>{}</loc></url>",
            xml_escape(&config.absolute(path))
        );
    }
    xml.push_str("</urlset>\n");
    xml
}

/// schema.org `Person` for the page owner, as JSON-LD.
pub(crate) fn person_json_ld(config: &Config) -> String {
    let mut json = String::from("{\"@context\":\"https://schema.org\",\"@type\":\"Person\"");
    let _ = write!(
        json,
        ",\"name\":{},\"jobTitle\":{},\"url\":{},\"image\":{}",
        json::string(site::NAME),
        json::string(site::JOB_TITLE),
        json::string(&config.absolute("/")),
        json::string(&config.absolute(&assets::url(site::OG_IMAGE))),
    );
    let _ = write!(
        json,
        ",\"address\":{{\"@type\":\"PostalAddress\",\"addressCountry\":{}}}",
        json::string(site::COUNTRY)
    );
    let _ = write!(
        json,
        ",\"knowsLanguage\":{},\"knowsAbout\":{}",
        json::array(resume::LANGUAGES.iter().map(|(_, code)| *code)),
        json::array(resume::SKILLS.iter().copied()),
    );
    let schools: Vec<String> = resume::EDUCATION
        .iter()
        .map(|school| {
            format!(
                "{{\"@type\":\"CollegeOrUniversity\",\"name\":{}}}",
                json::string(school.place)
            )
        })
        .collect();
    let _ = write!(json, ",\"alumniOf\":[{}]", schools.join(","));
    if !site::SAME_AS.is_empty() {
        let _ = write!(
            json,
            ",\"sameAs\":{}",
            json::array(site::SAME_AS.iter().copied())
        );
    }
    json.push('}');
    json
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
        Config::new("https://portfolio.example.com", vec![])
    }

    #[test]
    fn robots_points_to_sitemap() {
        assert!(
            robots_txt(&config()).contains("Sitemap: https://portfolio.example.com/sitemap.xml")
        );
    }

    #[test]
    fn sitemap_lists_all_pages() {
        let xml = sitemap_xml(&config());
        assert!(xml.contains("<loc>https://portfolio.example.com/</loc>"));
        assert!(xml.contains("<loc>https://portfolio.example.com/resume</loc>"));
    }

    #[test]
    fn json_ld_describes_person() {
        let json = person_json_ld(&config());
        assert!(json.starts_with(r#"{"@context":"https://schema.org","@type":"Person""#));
        assert!(json.contains(r#""url":"https://portfolio.example.com/""#));
        assert!(json.contains(r#""sameAs":["https://github.com/makela"]"#));
    }
}
