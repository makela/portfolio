//! Cache-busting asset URLs and the matching `Cache-Control` policy.

use std::borrow::Cow;

use axum::{
    extract::Request,
    http::{header::CACHE_CONTROL, HeaderValue},
    middleware::Next,
    response::Response,
};

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// Cache policy for versioned assets: the URL changes whenever the content
/// does, so browsers may keep a copy for a year without rechecking.
pub(crate) const IMMUTABLE: HeaderValue =
    HeaderValue::from_static("public, max-age=31536000, immutable");

/// Cache policy for everything else: browsers may store it but must check
/// with the server before reusing it.
pub(crate) const NO_CACHE: HeaderValue = HeaderValue::from_static("no-cache");

/// Hash recorded by `build.rs` when the binary was compiled.
fn compiled_hash(path: &str) -> Option<&'static str> {
    ASSETS
        .iter()
        .find(|(asset, _)| *asset == path)
        .map(|(_, hash)| *hash)
}

/// Debug builds hash the file on disk on every call, so an edited asset gets a
/// new URL on the next page load without rebuilding or restarting. The few
/// small files make this negligible, and release builds never do it.
#[cfg(debug_assertions)]
fn live_hash(path: &str) -> Option<String> {
    let relative = path.strip_prefix("/static/")?;
    let bytes = std::fs::read(std::path::Path::new("static").join(relative)).ok()?;
    Some(crate::hash::content_hash(&bytes))
}

fn hash_of(path: &str) -> Option<Cow<'static, str>> {
    #[cfg(debug_assertions)]
    if let Some(hash) = live_hash(path) {
        return Some(Cow::Owned(hash));
    }
    compiled_hash(path).map(Cow::Borrowed)
}

/// Returns `path` with its content hash appended, e.g.
/// `/static/style.css?v=1a2b3c4d5e6f7a8b`. Paths that aren't in `static/` are
/// returned unchanged (the tests make sure every asset we link exists).
pub(crate) fn url(path: &str) -> Cow<'_, str> {
    match hash_of(path) {
        Some(hash) => Cow::Owned(format!("{path}?v={hash}")),
        None => Cow::Borrowed(path),
    }
}

/// Middleware for `/static`: successful responses to versioned URLs
/// (`?v=...`) are cached forever, anything else must be revalidated.
pub(crate) async fn cache_control(request: Request, next: Next) -> Response {
    let versioned = request
        .uri()
        .query()
        .is_some_and(|q| q.split('&').any(|pair| pair.starts_with("v=")));

    let mut response = next.run(request).await;
    let policy = if versioned && response.status().is_success() {
        IMMUTABLE
    } else {
        NO_CACHE
    };
    response.headers_mut().insert(CACHE_CONTROL, policy);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::PROJECTS;

    #[test]
    fn stylesheet_is_versioned() {
        assert!(url("/static/style.css").starts_with("/static/style.css?v="));
    }

    #[test]
    fn every_project_image_is_compiled_in() {
        for project in PROJECTS {
            assert!(
                compiled_hash(project.image).is_some(),
                "{}: image {} not found in static/",
                project.title,
                project.image
            );
        }
    }

    #[test]
    fn live_and_compiled_hashes_agree() {
        for (path, compiled) in ASSETS {
            assert_eq!(
                hash_of(path).as_deref(),
                Some(*compiled),
                "{path} changed since build"
            );
        }
    }

    #[test]
    fn unknown_paths_are_unchanged() {
        assert_eq!(url("/static/missing.png"), "/static/missing.png");
    }
}
