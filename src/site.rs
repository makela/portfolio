//! Site identity and configuration used by the page and the SEO files.
//!
//! After changing the name or texts, update design/og-image.svg to match and
//! regenerate the social preview with `just og-image`.

use crate::status;

/// Name shown in the heading, footer, title and structured data.
pub(crate) const NAME: &str = "Timo Mäkelä";

/// Page `<title>`: keep it under ~60 characters so search results don't cut it.
pub(crate) const TITLE: &str = "Timo Mäkelä — Software Engineer & Entrepreneur";

/// Job title in the structured data.
pub(crate) const JOB_TITLE: &str = "Software Engineer";

/// Country in the structured data (ISO 3166-1 alpha-2).
pub(crate) const COUNTRY: &str = "FI";

/// One-line intro under the heading.
pub(crate) const TAGLINE: &str = "Software engineer. I build web with Rust \
                                  — and teach Tai Chi.";

/// Short paragraph under the tagline.
pub(crate) const INTRO: &str = "I've worked on healthcare, laboratory testing, \
                                bioinformatics and telecommunications.";

/// Snippet for search results and link previews (aim for 120–160 characters).
pub(crate) const DESCRIPTION: &str = "Timo Mäkelä, self-employed software engineer: \
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
    /// Resource usage of the sites in `STATUS_SITES`, for the footer and /status.
    pub(crate) status: status::Monitor,
}

impl Config {
    /// Reads `SITE_URL`, falling back to the local address for development,
    /// and `STATUS_SITES` (see [`status::parse_sites`]) with
    /// `STATUS_CGROUP_ROOT`, which may be unset.
    pub(crate) fn from_env(addr: &str) -> Result<Self, String> {
        let site_url = std::env::var("SITE_URL").unwrap_or_else(|_| format!("http://{addr}"));
        let sites = std::env::var("STATUS_SITES").unwrap_or_default();
        let sites = status::parse_sites(&sites).map_err(|e| format!("STATUS_SITES: {e}"))?;
        let cgroups =
            std::env::var("STATUS_CGROUP_ROOT").unwrap_or_else(|_| status::CGROUP_ROOT.to_string());
        Ok(Self::with_status(
            &site_url,
            status::Monitor::new(sites, &cgroups),
        ))
    }

    /// For tests: the given sites, read from the system services' cgroups.
    #[cfg(test)]
    pub(crate) fn new(site_url: &str, sites: Vec<status::Site>) -> Self {
        Self::with_status(site_url, status::Monitor::new(sites, status::CGROUP_ROOT))
    }

    fn with_status(site_url: &str, status: status::Monitor) -> Self {
        Self {
            site_url: site_url.trim_end_matches('/').to_string(),
            status,
        }
    }

    /// Absolute URL for a path on this site, e.g. `absolute("/")`.
    pub(crate) fn absolute(&self, path: &str) -> String {
        format!("{}{path}", self.site_url)
    }
}
