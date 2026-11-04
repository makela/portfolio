//! Site identity and configuration used by the page and the SEO files.
//!
//! The texts here are placeholders until the real name and intro are written;
//! after changing them, regenerate the social preview with `just og-image`.

/// Name shown in the heading, footer, title and structured data.
pub(crate) const NAME: &str = "Your Name";

/// Page `<title>`: keep it under ~60 characters so search results don't cut it.
pub(crate) const TITLE: &str = "Your Name — Portfolio";

/// One-line intro under the heading.
pub(crate) const TAGLINE: &str = "Software developer building things with Rust.";

/// Snippet for search results and link previews (aim for 120–160 characters).
pub(crate) const DESCRIPTION: &str = "Portfolio of Your Name: websites and web apps built \
     with Rust and Haskell (IHP), including tai chi club sites, a blog and an online order form.";

/// Profiles that belong to the same person (e.g. GitHub), listed as
/// `sameAs` in the structured data so search engines can connect them.
pub(crate) const SAME_AS: &[&str] = &[];

/// Social preview image (1200×630 PNG, generated from og-image.svg).
pub(crate) const OG_IMAGE: &str = "/static/og-image.png";
pub(crate) const OG_IMAGE_ALT: &str = "Your Name — Portfolio";

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
