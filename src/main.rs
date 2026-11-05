mod assets;
#[cfg(debug_assertions)]
mod hash;
#[cfg(feature = "livereload")]
mod livereload;
mod projects;
mod resume;
mod seo;
mod site;
mod status;
mod templates;

use axum::{
    extract::State, http::header::CACHE_CONTROL, middleware, response::Redirect, routing::get,
    Router,
};
use maud::Markup;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::EnvFilter;

/// The page itself is never cached without revalidation, so browsers always
/// see the current versioned asset URLs.
async fn index(
    State(config): State<site::Config>,
) -> (
    [(axum::http::HeaderName, axum::http::HeaderValue); 1],
    Markup,
) {
    (
        [(CACHE_CONTROL, assets::NO_CACHE)],
        templates::index(&config),
    )
}

async fn resume_page(
    State(config): State<site::Config>,
) -> (
    [(axum::http::HeaderName, axum::http::HeaderValue); 1],
    Markup,
) {
    ([(CACHE_CONTROL, assets::NO_CACHE)], resume::page(&config))
}

/// Browsers and crawlers ask for /favicon.ico regardless of the page's icon
/// link. Temporary redirect, because the versioned target changes with the file.
async fn favicon() -> Redirect {
    Redirect::temporary(&assets::url("/static/favicon.svg"))
}

/// Whether stdout is connected to journald. systemd sets `JOURNAL_STREAM` to
/// the `device:inode` of the journal stream, but child processes (e.g. of a
/// terminal started by systemd) inherit it, so compare it with stdout itself.
fn stdout_is_journal() -> bool {
    use std::os::unix::fs::MetadataExt;

    let Some(stream) = std::env::var_os("JOURNAL_STREAM") else {
        return false;
    };
    let Ok(stdout) = std::fs::metadata("/proc/self/fd/1") else {
        return false;
    };
    stream.to_string_lossy() == format!("{}:{}", stdout.dev(), stdout.ino())
}

/// Logs to stdout, filtered by `RUST_LOG` (default `info`). Colours only on a
/// terminal; under journald, timestamps are left to the journal.
fn init_tracing() {
    use std::io::IsTerminal;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(std::io::stdout().is_terminal());
    if stdout_is_journal() {
        fmt.without_time().init();
    } else {
        fmt.init();
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    let static_files = Router::new()
        .fallback_service(ServeDir::new("static"))
        .layer(middleware::from_fn(assets::cache_control));

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let config = site::Config::from_env(&addr).map_err(std::io::Error::other)?;
    tracing::info!("site URL: {}", config.site_url);

    let app = Router::new()
        .route("/", get(index))
        .route("/resume", get(resume_page))
        .route("/robots.txt", get(seo::robots))
        .route("/sitemap.xml", get(seo::sitemap))
        .route("/status", get(status::page))
        .route("/status.json", get(status::json))
        .route("/favicon.ico", get(favicon))
        .nest("/static", static_files)
        // One line per request: method, path, status and latency.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(config);

    #[cfg(feature = "livereload")]
    let (app, _static_watcher) = livereload::enable(app)?;

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app).await
}
