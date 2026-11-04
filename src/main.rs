mod projects;
mod templates;

use axum::{routing::get, Router};
use maud::Markup;
use tower_http::services::ServeDir;

async fn index() -> Markup {
    templates::index()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .nest_service("/static", ServeDir::new("static"));

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await
}
