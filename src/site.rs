//! Site identity and configuration used by the page and the SEO files.
//!
//! After changing the name or texts, update design/og-image.svg to match and
//! regenerate the social preview with `just og-image`.

/// Name shown in the heading, footer, title and structured data.
pub(crate) const NAME: &str = "Timo Mäkelä";

/// Page `<title>`: keep it under ~60 characters so search results don't cut it.
pub(crate) const TITLE: &str = "Timo Mäkelä — Software Engineer & Entrepreneur";

/// Job title in the structured data.
pub(crate) const JOB_TITLE: &str = "Software Engineer";

/// Country in the structured data (ISO 3166-1 alpha-2).
pub(crate) const COUNTRY: &str = "FI";

/// One-line intro under the heading.
pub(crate) const TAGLINE: &str = "Self-employed software engineer in Finland. I build websites \
     and web apps with Rust, Haskell and Nix — and teach Tai Chi.";

/// Short paragraph under the tagline.
pub(crate) const INTRO: &str = "For nearly twenty years I've built software for healthcare, \
     laboratory testing, bioinformatics and telecommunications — from command-line tooling to \
     interactive visualizations and reports. Today I work remotely on my own schedule, which \
     leaves room for my other passion: Tai Chi.";

/// Snippet for search results and link previews (aim for 120–160 characters).
pub(crate) const DESCRIPTION: &str = "Timo Mäkelä, self-employed software engineer in Finland: \
     websites and web apps built with Rust, Haskell (IHP) and Nix, including Tai Chi club sites.";

/// Profiles that belong to the same person (e.g. GitHub), listed as
/// `sameAs` in the structured data so search engines can connect them.
pub(crate) const SAME_AS: &[&str] = &[GITHUB];

/// GitHub profile, also linked in the footer.
pub(crate) const GITHUB: &str = "https://github.com/makela";

/// Social preview image (1200×630 PNG, generated from design/og-image.svg).
pub(crate) const OG_IMAGE: &str = "/static/og-image.png";
pub(crate) const OG_IMAGE_ALT: &str = "Timo Mäkelä — Software Engineer & Entrepreneur";

/// Runtime configuration.
#[derive(Debug, Clone)]
pub(crate) struct Config {
    /// Public URL without a trailing slash, e.g. `https://portfolio.example.com`.
    /// Used for the canonical URL, link previews, robots.txt and the sitemap.
    pub(crate) site_url: String,
}

impl Config {
    /// Reads `SITE_URL`, falling back to the local address for development.
    pub(crate) fn from_env(addr: &str) -> Self {
        let site_url = std::env::var("SITE_URL").unwrap_or_else(|_| format!("http://{addr}"));
        Self {
            site_url: site_url.trim_end_matches('/').to_string(),
        }
    }

    /// Absolute URL for a path on this site, e.g. `absolute("/")`.
    pub(crate) fn absolute(&self, path: &str) -> String {
        format!("{}{path}", self.site_url)
    }
}
