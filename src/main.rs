mod assets;
#[cfg(debug_assertions)]
mod hash;
#[cfg(feature = "livereload")]
mod livereload;
mod projects;
mod templates;

use axum::{http::header::CACHE_CONTROL, middleware, routing::get, Router};
use maud::Markup;
use tower_http::services::ServeDir;

/// The page itself is never cached without revalidation, so browsers always
/// see the current versioned asset URLs.
async fn index() -> (
    [(axum::http::HeaderName, axum::http::HeaderValue); 1],
    Markup,
) {
    ([(CACHE_CONTROL, assets::NO_CACHE)], templates::index())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let static_files = Router::new()
        .fallback_service(ServeDir::new("static"))
        .layer(middleware::from_fn(assets::cache_control));

    let app = Router::new()
        .route("/", get(index))
        .nest("/static", static_files);

    #[cfg(feature = "livereload")]
    let (app, _static_watcher) = livereload::enable(app)?;

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await
}
