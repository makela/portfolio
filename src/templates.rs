use maud::{html, Markup, DOCTYPE};

use crate::{
    assets,
    projects::{Project, PROJECTS},
};

pub(crate) fn index() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Your Name — Portfolio" }
                link rel="stylesheet" href=(assets::url("/static/style.css"));
            }
            body {
                header.hero {
                    h1 { "Your Name" }
                    p { "Software developer building things with Rust." }
                }
                main {
                    (projects_section())
                }
                footer {
                    p { "© Your Name" }
                }
            }
        }
    }
}

fn projects_section() -> Markup {
    html! {
        section #projects {
            h2 { "Projects" }
            div.grid {
                @for p in PROJECTS {
                    (project_card(p))
                }
            }
        }
    }
}

fn project_card(p: &Project) -> Markup {
    html! {
        article.card {
            img src=(assets::url(p.image)) alt={ "Screenshot of " (p.title) } loading="lazy";
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
