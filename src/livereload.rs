//! Development live reload (`--features livereload`, used by `just watch`).
//!
//! Injects a small script into HTML responses that reloads the browser tab
//! when the server restarts (after `watchexec` rebuilds it) or when a file
//! under `static/` changes (no restart needed, see `assets::url`).

use std::{io, path::Path};

use axum::Router;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tower_livereload::LiveReloadLayer;

/// Adds the live-reload layer to `app`. Keep the returned watcher alive for as
/// long as the server runs; dropping it stops watching `static/`.
pub(crate) fn enable(app: Router) -> io::Result<(Router, RecommendedWatcher)> {
    let layer = LiveReloadLayer::new();
    let reloader = layer.reloader();

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok_and(|e| e.kind.is_create() || e.kind.is_modify() || e.kind.is_remove()) {
            reloader.reload();
        }
    })
    .map_err(io::Error::other)?;
    watcher
        .watch(Path::new("static"), RecursiveMode::Recursive)
        .map_err(io::Error::other)?;

    println!("Live reload enabled (watching static/)");
    Ok((app.layer(layer), watcher))
}
