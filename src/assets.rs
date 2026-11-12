//! Cache-busting asset URLs and the matching `Cache-Control` policy.

use std::{borrow::Cow, fmt::Write as _};

use axum::{
    extract::Request,
    http::{header::CACHE_CONTROL, HeaderValue},
    middleware::Next,
    response::Response,
};

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// Cache policy for versioned assets: the URL changes whenever the content
/// does, so browsers may keep a copy for a year without rechecking.
const IMMUTABLE_POLICY: &str = "public, max-age=31536000, immutable";
pub(crate) const IMMUTABLE: HeaderValue = HeaderValue::from_static(IMMUTABLE_POLICY);

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

/// Whether `query` asks for the current version of the asset at `path`
/// (`/static/…`): its `v` is the file's hash now. An old or made-up `v`
/// gets today's file, which mustn't be cached as that version for a year.
fn is_current_version(path: &str, query: Option<&str>) -> bool {
    let v = query.and_then(|q| q.split('&').find_map(|pair| pair.strip_prefix("v=")));
    v.is_some_and(|v| hash_of(path).is_some_and(|hash| hash == v))
}

/// Middleware for `/static`: successful responses to the current version's
/// URL (`?v=<its hash>`) are cached forever, anything else must be
/// revalidated.
pub(crate) async fn cache_control(request: Request, next: Next) -> Response {
    // Nested under /static, the router sees the path without it.
    let path = format!("/static{}", request.uri().path());
    let current = is_current_version(&path, request.uri().query());

    let mut response = next.run(request).await;
    let policy = if current && response.status().is_success() {
        IMMUTABLE
    } else {
        NO_CACHE
    };
    response.headers_mut().insert(CACHE_CONTROL, policy);
    response
}

/// The same policy for nginx, which serves `/static/` in production (see
/// flake.nix): a `map` from `$uri:$arg_v` to `Cache-Control`, one line per
/// file with its hash as compiled in. The package writes it at build time
/// with `portfolio --nginx-cache-map`.
pub(crate) fn nginx_cache_map() -> String {
    let mut map = String::new();
    for (path, hash) in ASSETS {
        // Writing to a String can't fail.
        let _ = writeln!(map, "\"{path}:{hash}\" \"{IMMUTABLE_POLICY}\";");
    }
    map
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
    fn only_the_current_version_is_immutable() {
        let path = "/static/style.css";
        let hash = hash_of(path).unwrap_or_default();
        assert!(is_current_version(path, Some(&format!("v={hash}"))));
        assert!(is_current_version(path, Some(&format!("x=1&v={hash}"))));
        for query in [None, Some("v="), Some("v=0123456789abcdef"), Some("x=1")] {
            assert!(!is_current_version(path, query), "{query:?}");
        }
        assert!(!is_current_version(
            "/static/missing.css",
            Some(&format!("v={hash}"))
        ));
    }

    #[test]
    fn nginx_map_has_every_asset_with_its_hash() {
        let map = nginx_cache_map();
        assert_eq!(map.lines().count(), ASSETS.len());
        let hash = compiled_hash("/static/style.css").unwrap_or_default();
        assert!(map.contains(&format!(
            "\"/static/style.css:{hash}\" \"public, max-age=31536000, immutable\";\n"
        )));
    }

    /// nginx won't start if a key doesn't fit its map's buckets, 128 bytes
    /// with nginx's own overhead (`mapHashBucketSize` in flake.nix).
    #[test]
    fn nginx_map_keys_fit_its_buckets() {
        for (path, hash) in ASSETS {
            let key = format!("{path}:{hash}");
            assert!(key.len() <= 100, "{key} is too long for nginx's map");
        }
    }

    #[test]
    fn unknown_paths_are_unchanged() {
        assert_eq!(url("/static/missing.png"), "/static/missing.png");
    }
}
