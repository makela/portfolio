mod assets;
#[cfg(debug_assertions)]
mod hash;
mod json;
#[cfg(feature = "livereload")]
mod livereload;
mod projects;
mod resume;
mod seo;
mod site;
mod status;
mod templates;

use axum::{
    extract::State,
    http::{header::CACHE_CONTROL, StatusCode},
    middleware,
    response::Redirect,
    routing::get,
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

/// Any other address: the not-found page, with its 404 status.
async fn not_found(
    State(config): State<site::Config>,
) -> (
    StatusCode,
    [(axum::http::HeaderName, axum::http::HeaderValue); 1],
    Markup,
) {
    (
        StatusCode::NOT_FOUND,
        [(CACHE_CONTROL, assets::NO_CACHE)],
        templates::not_found(&config),
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

/// Logs each request, with its status and latency, at `level`.
fn request_log(
    level: Level,
) -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    tower_http::trace::DefaultOnRequest,
    DefaultOnResponse,
> {
    TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(level))
        .on_response(DefaultOnResponse::new().level(level))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // For the package's build (flake.nix): print nginx's cache policy for
    // /static/ and stop.
    if std::env::args().nth(1).as_deref() == Some("--nginx-cache-map") {
        print!("{}", assets::nginx_cache_map());
        return Ok(());
    }
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
        .route("/favicon.ico", get(favicon))
        .nest("/static", static_files)
        // Not under /static/, which has its own (plain) 404.
        .fallback(not_found)
        // One line per request: method, path, status and latency.
        .layer(request_log(Level::INFO))
        // status.js polls this every 5 s from every open page, so its lines
        // are only logged with RUST_LOG=debug (added after the layer above,
        // so it doesn't log them too).
        .route(
            "/status.json",
            get(status::json).layer(request_log(Level::DEBUG)),
        )
        .with_state(config);

    #[cfg(feature = "livereload")]
    let (app, _static_watcher) = livereload::enable(app)?;

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app).await
}
