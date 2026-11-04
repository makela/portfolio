pub struct Project {
    pub title: &'static str,
    pub description: &'static str,
    pub image: &'static str,
    pub tags: &'static [&'static str],
    pub live_url: Option<&'static str>,
    pub repo_url: Option<&'static str>,
}

pub const PROJECTS: &[Project] = &[
    Project {
        title: "Project One",
        description: "A short description of what this project does and why it's interesting.",
        image: "/static/images/project-one.svg",
        tags: &["Rust", "Axum", "SQLite"],
        live_url: Some("https://example.com"),
        repo_url: Some("https://github.com/you/project-one"),
    },
    Project {
        title: "Project Two",
        description: "Another project. Mention the problem it solves, the interesting technical bits, and the outcome.",
        image: "/static/images/project-two.svg",
        tags: &["TypeScript", "WebSockets"],
        live_url: None,
        repo_url: Some("https://github.com/you/project-two"),
    },
];
