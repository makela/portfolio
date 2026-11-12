use std::fmt::Write as _;

use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{
    assets,
    projects::{Project, PROJECTS},
    seo,
    site::{self, Config},
    status,
};

/// Size of every project preview (all SVGs use a 640×360 viewBox; a test checks
/// it). Explicit sizes let the browser reserve space, so the layout doesn't
/// shift while images load.
const PREVIEW_WIDTH: u32 = 640;
const PREVIEW_HEIGHT: u32 = 360;

/// Cards visible without scrolling on a desktop (one row of the grid). They
/// load immediately; the rest are lazy-loaded.
const EAGER_CARDS: usize = 4;

/// Per-page metadata for the shared layout.
pub(crate) struct Page {
    /// Path on this site, e.g. `/resume`; also the canonical URL.
    pub(crate) path: &'static str,
    pub(crate) title: &'static str,
    /// Snippet for search results and link previews (120–160 characters).
    pub(crate) description: &'static str,
    /// Whether search engines should index it. A page that isn't has no
    /// canonical URL.
    pub(crate) indexed: bool,
    /// Whether the name in the header is the page's heading (`h1`): on the
    /// pages about the owner. The others have a heading of their own.
    pub(crate) name_is_heading: bool,
}

pub(crate) const HOME: Page = Page {
    path: "/",
    title: site::TITLE,
    description: site::DESCRIPTION,
    indexed: true,
    name_is_heading: true,
};

/// Any address that isn't a page, see [`not_found`]. It has no path of its
/// own, so it's never marked current in the navigation.
pub(crate) const NOT_FOUND: Page = Page {
    path: "",
    title: concat!("Page not found — ", site::name!()),
    description: "There is no page at this address.",
    indexed: false,
    name_is_heading: false,
};

/// IBM Plex Sans, self-hosted: IBM's own pre-split WOFF2 files from the
/// `@ibm/plex-sans@1.1.0` release (github.com/IBM/plex), unmodified, with the
/// OFL licence next to them. Latin1 covers English and Finnish text; Pi has
/// the arrows. Browsers download a file only when the page uses a character in
/// its `unicode-range`; other characters fall back to the system font.
const LATIN1: &str = "U+0000, U+000D, U+0020-007E, U+00A0-00FF, U+0131, U+0152-0153, U+02C6, U+02DA, U+02DC, U+2013-2014, U+2018-201A, U+201C-201E, U+2020-2022, U+2026, U+2030, U+2039-203A, U+2044, U+20AC, U+2122, U+2212, U+FB01-FB02";
const PI: &str = "U+0E3F, U+2000-200D, U+2015, U+2028-2029, U+202F, U+2032-2033, U+2070, U+2074-2079, U+2080-2089, U+2113, U+2116, U+2126, U+212E, U+2150-2151, U+2153-215E, U+2190-2199, U+21A9-21AA, U+21B0-21B3, U+21B6-21B7, U+21BA-21BB, U+21C4, U+21C6, U+2202, U+2206, U+220F, U+2211, U+2215, U+221A, U+221E, U+222B, U+2248, U+2260, U+2264-2265, U+25CA, U+2713, U+274C, U+2B0E-2B11, U+ECE0, U+EFCC, U+FEFF, U+FFFD";

/// (weight, file, unicode-range) for every `@font-face`.
const FONT_FACES: &[(u16, &str, &str)] = &[
    (
        400,
        "/static/fonts/IBMPlexSans-Regular-Latin1.woff2",
        LATIN1,
    ),
    (400, "/static/fonts/IBMPlexSans-Regular-Pi.woff2", PI),
    (
        600,
        "/static/fonts/IBMPlexSans-SemiBold-Latin1.woff2",
        LATIN1,
    ),
    (600, "/static/fonts/IBMPlexSans-SemiBold-Pi.woff2", PI),
];

/// Fonts every page needs for its first screen: preloaded, so text doesn't
/// render in the fallback font first and then jump.
const PRELOAD_FONTS: &[&str] = &[
    "/static/fonts/IBMPlexSans-Regular-Latin1.woff2",
    "/static/fonts/IBMPlexSans-SemiBold-Latin1.woff2",
];

/// The `@font-face` rules live in the page rather than in style.css, so the
/// font URLs carry content hashes like every other asset (cached for a year)
/// and exactly match the preload links.
fn font_faces() -> String {
    let mut css = String::new();
    for (weight, file, range) in FONT_FACES {
        // Writing to a String can't fail.
        let _ = write!(
            css,
            "@font-face{{font-family:\"IBM Plex Sans\";font-style:normal;font-weight:{weight};\
             font-display:swap;src:url(\"{}\") format(\"woff2\");unicode-range:{range}}}",
            assets::url(file)
        );
    }
    css
}

/// Navigation between the pages, in the header; the current page is marked,
/// not linked. /status is reached through the footer charts.
const NAV: &[(&str, &str)] = &[("/", "Projects"), ("/resume", "Resume")];

/// HTML shell shared by every page: `<head>` with the SEO tags, and the footer.
pub(crate) fn layout(config: &Config, page: &Page, content: &Markup) -> Markup {
    let canonical = config.absolute(page.path);
    // While a site is running, every page's footer shows the server's CPU
    // and memory, drawn by status.js, see footer_charts.
    let running = status::any_running(&config.status.snapshot());
    let footer_charts = footer_charts(page, running);
    let live_status = page.path == status::PAGE.path || running;
    let og_image = config.absolute(&assets::url(site::OG_IMAGE));
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (page.title) }
                meta name="description" content=(page.description);
                @if page.indexed {
                    link rel="canonical" href=(canonical);
                } @else {
                    meta name="robots" content="noindex";
                }

                // Link previews (Open Graph, also read by LinkedIn, Slack, …)
                meta property="og:type" content="website";
                @if page.indexed {
                    meta property="og:url" content=(canonical);
                }
                meta property="og:title" content=(page.title);
                meta property="og:description" content=(page.description);
                meta property="og:image" content=(og_image);
                meta property="og:image:width" content="1200";
                meta property="og:image:height" content="630";
                meta property="og:image:alt" content=(site::OG_IMAGE_ALT);
                meta name="twitter:card" content="summary_large_image";

                link rel="icon" type="image/svg+xml" href=(assets::url("/static/favicon.svg"));
                meta name="theme-color" content="#fafafa" media="(prefers-color-scheme: light)";
                meta name="theme-color" content="#121212" media="(prefers-color-scheme: dark)";
                @for font in PRELOAD_FONTS {
                    link rel="preload" href=(assets::url(font)) as="font" type="font/woff2" crossorigin;
                }
                style { (PreEscaped(font_faces())) }
                link rel="stylesheet" href=(assets::url("/static/style.css"));
                // Blocking on purpose: applies a saved theme before first paint.
                script src=(assets::url("/static/theme.js")) {}
                @if live_status {
                    script src=(assets::url("/static/status.js")) defer {}
                }

                script type="application/ld+json" { (PreEscaped(seo::person_json_ld(config))) }
            }
            body {
                (site_header(page))
                (content)
                footer {
                    p {
                        "© " (site::NAME)
                        " · "
                        // rel="me": this profile belongs to the page owner.
                        a href=(site::GITHUB) rel="me" { "GitHub" (external_mark()) }
                    }
                    p.colophon { "Built with Rust, Axum and Maud — pair-programmed with Claude Code." }
                    @if footer_charts {
                        // CPU and memory of every site,
                        // as on /status but without the legend, linking to
                        // it. Drawn by status.js, which shows them.
                        a.footer-charts href=(status::PAGE.path) title="Server status"
                            aria-label="Server status: CPU and memory of the sites on this server" hidden {
                            div.spark.cpu-chart {}
                            div.membar.memory-chart {}
                        }
                    }
                }
            }
        }
    }
}

/// The top of every page: the name, linking to the front page, then the
/// navigation and the theme toggle. In print only the name shows.
fn site_header(page: &Page) -> Markup {
    let name = html! { a href="/" { (site::NAME) } };
    html! {
        header.site-header {
            @if page.name_is_heading {
                h1.site-name { (name) }
            } @else {
                p.site-name { (name) }
            }
            nav.site-nav aria-label="Pages" {
                ul {
                    @for (path, label) in NAV {
                        // data-label: the link's width in bold, so the links
                        // don't move when the current one is, see style.css
                        li {
                            @if *path == page.path {
                                span aria-current="page" data-label=(label) { (label) }
                            } @else {
                                a href=(path) data-label=(label) { (label) }
                            }
                        }
                    }
                }
            }
            (theme_toggle())
        }
    }
}

/// Whether the page's footer shows the sites' CPU and memory, as the link to
/// /status: on every page while a site is running, except /status itself,
/// which has the big charts.
fn footer_charts(page: &Page, running: bool) -> bool {
    running && page.path != status::PAGE.path
}

/// Marks a link that leaves this site: an arrow for sighted readers, and
/// words for screen readers (which would otherwise say "north east arrow").
fn external_mark() -> Markup {
    html! {
        " "
        span aria-hidden="true" { "↗" }
        span.visually-hidden { " (opens external site)" }
    }
}

/// Light/dark toggle, wired up by theme.js. Hidden until the script runs, so
/// it never shows where it can't work.
fn theme_toggle() -> Markup {
    html! {
        button #theme-toggle .theme-toggle type="button" aria-label="Switch theme" hidden {
            svg.moon viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {
                path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" {}
            }
            svg.sun viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {
                circle cx="12" cy="12" r="4" {}
                path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" {}
            }
        }
    }
}

pub(crate) fn index(config: &Config) -> Markup {
    layout(
        config,
        &HOME,
        &html! {
            // The page's heading is the name in the header, see site_header.
            div.hero {
                p.tagline { (site::TAGLINE) }
                p.intro { (site::INTRO) }
                p.more { a href="/resume" { "Read my resume →" } }
            }
            main {
                (projects_section())
            }
        },
    )
}

/// The page for an address that isn't one, in the site's layout, with the
/// way back to the pages that are.
pub(crate) fn not_found(config: &Config) -> Markup {
    layout(
        config,
        &NOT_FOUND,
        &html! {
            main.not-found {
                h1 { "Page not found" }
                p { "There is no page at this address. It may have moved, or the link may be mistyped." }
                p.more {
                    a href="/" { "See my projects" }
                    " or "
                    a href="/resume" { "read my resume" }
                    "."
                }
            }
        },
    )
}

fn projects_section() -> Markup {
    html! {
        section #projects {
            h2 { "Projects" }
            div.grid {
                @for (i, p) in PROJECTS.iter().enumerate() {
                    (project_card(p, i < EAGER_CARDS))
                }
            }
        }
    }
}

fn project_card(p: &Project, eager: bool) -> Markup {
    html! {
        article.card {
            img src=(assets::url(p.image))
                alt={ "Preview of " (p.title) }
                width=(PREVIEW_WIDTH)
                height=(PREVIEW_HEIGHT)
                loading=(if eager { "eager" } else { "lazy" });
            div.card-body {
                h3 { (p.title) }
                p { (p.description) }
                ul.tags {
                    @for t in p.tags {
                        li { (t) }
                    }
                }
                div.links {
                    @if let Some(url) = p.live_url {
                        a href=(url) { "Live" (external_mark()) }
                    }
                    @if let Some(url) = p.demo_url {
                        a href=(url) { "Demo" (external_mark()) }
                    }
                    @if let Some(url) = p.admin_url {
                        a href=(url) { "Admin" (external_mark()) }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The width/height attributes must match the real previews.
    #[test]
    fn previews_are_640_by_360() {
        for project in PROJECTS {
            let file = project.image.trim_start_matches('/');
            let svg = std::fs::read_to_string(file).unwrap_or_default();
            assert!(
                svg.contains(r#"viewBox="0 0 640 360""#),
                "{}: {file} should have viewBox 0 0 640 360",
                project.title
            );
        }
    }

    /// Search results show about 160 characters; shorter than 120 wastes space.
    #[test]
    fn descriptions_fit_search_results() {
        for page in [&HOME, &crate::resume::PAGE, &status::PAGE] {
            let length = page.description.chars().count();
            assert!(
                (120..=160).contains(&length),
                "{}: description is {length} characters",
                page.path
            );
        }
    }

    #[test]
    fn head_has_seo_tags() {
        let config = Config::new("https://portfolio.example.com", vec![]);
        let html = index(&config).into_string();
        for expected in [
            r#"<link rel="canonical" href="https://portfolio.example.com/">"#,
            r#"<meta name="description" content=""#,
            r#"<meta property="og:image" content="https://portfolio.example.com/static/og-image.png?v="#,
            r#"<script type="application/ld+json">{"@context":"https://schema.org""#,
            r#"<a href="https://github.com/makela" rel="me">GitHub <span aria-hidden="true">↗</span><span class="visually-hidden"> (opens external site)</span></a>"#,
            r#"<a href="/resume" data-label="Resume">Resume</a>"#,
            r#"<span aria-current="page" data-label="Projects">Projects</span>"#,
            r#"<script src="/static/theme.js?v="#,
            r#"id="theme-toggle""#,
        ] {
            assert!(html.contains(expected), "missing: {expected}");
        }
    }

    #[test]
    fn not_found_page_is_not_indexed() {
        let html = not_found(&Config::new("https://portfolio.example.com", vec![])).into_string();
        assert!(html.contains(r#"<meta name="robots" content="noindex">"#));
        assert!(!html.contains(r#"rel="canonical""#));
        assert!(!html.contains("og:url"));
        assert!(html.contains("<h1>Page not found</h1>"));
        // Both pages are links, neither is marked current.
        assert!(html.contains(r#"<a href="/" data-label="Projects">Projects</a>"#));
        assert!(!html.contains("aria-current"));
        // The pages that are indexed do have their canonical URL.
        let home = index(&Config::new("https://portfolio.example.com", vec![])).into_string();
        assert!(!home.contains("noindex"));
    }

    /// Each page has one h1: the name in the header on the pages about the
    /// owner, the page's own heading on the others.
    #[test]
    fn one_main_heading_per_page() {
        let config = Config::new("https://portfolio.example.com", vec![]);
        let name_heading = r#"<h1 class="site-name"><a href="/">"#;
        for (page, name_is_heading) in [
            (index(&config).into_string(), true),
            (crate::resume::page(&config).into_string(), true),
            (not_found(&config).into_string(), false),
            (
                layout(&config, &status::PAGE, &html! { h1 { "Server status" } }).into_string(),
                false,
            ),
        ] {
            assert_eq!(page.matches("<h1").count(), 1, "{page}");
            assert_eq!(page.contains(name_heading), name_is_heading, "{page}");
            assert!(page.contains(r#"<nav class="site-nav" aria-label="Pages">"#));
        }
    }

    /// status.js runs on /status, and on the other pages only while a site is
    /// running (none here), for the footer's charts and their history.
    #[test]
    fn status_script_only_where_needed() {
        let config = Config::new("https://portfolio.example.com", vec![]);
        let script = r#"<script src="/static/status.js?v="#;
        assert!(!index(&config).into_string().contains(script));
        assert!(!crate::resume::page(&config).into_string().contains(script));
        let status = layout(&config, &status::PAGE, &html! {}).into_string();
        assert!(status.contains(script));
        assert!(
            !status.contains("cpu-chart"),
            "no footer chart without sites"
        );
    }

    #[test]
    fn footer_charts_on_every_page_but_status() {
        assert!(footer_charts(&HOME, true));
        assert!(footer_charts(&crate::resume::PAGE, true));
        assert!(!footer_charts(&status::PAGE, true));
        assert!(!footer_charts(&HOME, false));
        assert!(!footer_charts(&crate::resume::PAGE, false));
    }

    /// Every font is in static/, versioned, and preloads use the same URLs
    /// as the `@font-face` rules (otherwise the browser downloads twice).
    #[test]
    fn fonts_are_versioned_and_preloads_match() {
        let faces = font_faces();
        for (_, file, _) in FONT_FACES {
            let url = assets::url(file);
            assert!(url.contains("?v="), "{file} not found in static/");
            assert!(
                faces.contains(url.as_ref()),
                "{file} missing from @font-face"
            );
        }
        for file in PRELOAD_FONTS {
            assert!(
                FONT_FACES.iter().any(|(_, face, _)| face == file),
                "{file} is preloaded but has no @font-face"
            );
        }
        assert!(std::path::Path::new("static/fonts/LICENSE.txt").exists());
    }
}
