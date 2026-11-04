use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{
    assets,
    projects::{Project, PROJECTS},
    seo,
    site::{self, Config},
};

/// Size of every project preview (all SVGs use a 640×360 viewBox; a test checks
/// it). Explicit sizes let the browser reserve space, so the layout doesn't
/// shift while images load.
const PREVIEW_WIDTH: u32 = 640;
const PREVIEW_HEIGHT: u32 = 360;

/// Cards visible without scrolling on a desktop (one row of the grid). They
/// load immediately; the rest are lazy-loaded.
const EAGER_CARDS: usize = 3;

/// Per-page metadata for the shared layout.
pub(crate) struct Page {
    /// Path on this site, e.g. `/resume`; also the canonical URL.
    pub(crate) path: &'static str,
    pub(crate) title: &'static str,
    /// Snippet for search results and link previews (120–160 characters).
    pub(crate) description: &'static str,
}

pub(crate) const HOME: Page = Page {
    path: "/",
    title: site::TITLE,
    description: site::DESCRIPTION,
};

/// Navigation between the pages; the current page is marked, not linked.
const NAV: &[(&str, &str)] = &[("/", "Portfolio"), ("/resume", "Resume")];

/// HTML shell shared by every page: `<head>` with the SEO tags, and the footer.
pub(crate) fn layout(config: &Config, page: &Page, content: &Markup) -> Markup {
    let canonical = config.absolute(page.path);
    let og_image = config.absolute(&assets::url(site::OG_IMAGE));
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (page.title) }
                meta name="description" content=(page.description);
                link rel="canonical" href=(canonical);

                // Link previews (Open Graph, also read by LinkedIn, Slack, …)
                meta property="og:type" content="website";
                meta property="og:url" content=(canonical);
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
                link rel="stylesheet" href=(assets::url("/static/style.css"));

                script type="application/ld+json" { (PreEscaped(seo::person_json_ld(config))) }
            }
            body {
                (content)
                footer {
                    p {
                        "© " (site::NAME)
                        @for (path, label) in NAV {
                            " · "
                            @if *path == page.path {
                                span aria-current="page" { (label) }
                            } @else {
                                a href=(path) { (label) }
                            }
                        }
                        " · "
                        // rel="me": this profile belongs to the page owner.
                        a href=(site::GITHUB) rel="me" { "GitHub" }
                    }
                }
            }
        }
    }
}

pub(crate) fn index(config: &Config) -> Markup {
    layout(
        config,
        &HOME,
        &html! {
            header.hero {
                h1 { (site::NAME) }
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
                        a href=(url) { "Live ↗" }
                    }
                    @if let Some(url) = p.repo_url {
                        a href=(url) { "Source ↗" }
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
        for page in [&HOME, &crate::resume::PAGE] {
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
        let config = Config {
            site_url: "https://portfolio.example.com".to_string(),
        };
        let html = index(&config).into_string();
        for expected in [
            r#"<link rel="canonical" href="https://portfolio.example.com/">"#,
            r#"<meta name="description" content=""#,
            r#"<meta property="og:image" content="https://portfolio.example.com/static/og-image.png?v="#,
            r#"<script type="application/ld+json">{"@context":"https://schema.org""#,
            r#"<a href="https://github.com/makela" rel="me">GitHub</a>"#,
            r#"<a href="/resume">Resume</a>"#,
            r#"<span aria-current="page">Portfolio</span>"#,
        ] {
            assert!(html.contains(expected), "missing: {expected}");
        }
    }
}
