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

fn hash_of(path: &str) -> Option<&'static str> {
    ASSETS
        .iter()
        .find(|(asset, _)| *asset == path)
        .map(|(_, hash)| *hash)
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
    fn every_project_image_exists() {
        for project in PROJECTS {
            assert!(
                hash_of(project.image).is_some(),
                "{}: image {} not found in static/",
                project.title,
                project.image
            );
        }
    }

    #[test]
    fn unknown_paths_are_unchanged() {
        assert_eq!(url("/static/missing.png"), "/static/missing.png");
    }
}
