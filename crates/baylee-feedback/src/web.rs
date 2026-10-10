//! The built web UI (`web/feedback`, #311), served from `FEEDBACK_WEB_DIR`,
//! and the headers every response of the service carries.
//!
//! Files under `assets/` have a content hash in their name and are cached
//! for a year as immutable; everything else, `index.html` above all, is
//! revalidated each time. A path that names no file and has no extension is
//! one of the UI's own routes and gets `index.html`. A path with a `..`, a
//! `.`, a backslash or a NUL in it is refused before it touches the disk,
//! and a file that resolves outside the directory (a symlink out) is not
//! served.

use std::path::{Component, Path as FsPath, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderValue, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

use crate::refuse;
use crate::ui::Ui;

/// The one host besides the service's own that the page may load a picture
/// from: Scryfall's image host, for a card a report names
/// (`docs/legal.md` §3, #270). The admin's browser fetches it straight from
/// there; this service never does, so it neither proxies nor republishes a
/// card image. It is an image source only: `connect-src` stays `'self'`,
/// so the page cannot call Scryfall's API.
pub const SCRYFALL_IMAGES: &str = "https://cards.scryfall.io";

/// The service's content security policy: its own origin for everything,
/// images also as `data:` (a report's screenshot is a PNG in base64) and
/// from [`SCRYFALL_IMAGES`], no inline script, no plugin, no frame around
/// it.
pub const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
    img-src 'self' data: https://cards.scryfall.io; font-src 'self'; connect-src 'self'; \
    object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

/// Adds the security headers to every response, API or file.
pub(crate) async fn security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    for (name, value) in [
        ("content-security-policy", CSP),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "no-referrer"),
        ("x-content-type-options", "nosniff"),
        ("cross-origin-opener-policy", "same-origin"),
        ("cross-origin-resource-policy", "same-origin"),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    // A JSON answer holds reports; no cache along the way keeps one.
    if !headers.contains_key(header::CACHE_CONTROL) {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

/// The content type of a file the UI's build writes, by extension.
fn content_type(path: &FsPath) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// `%XX` decoded; `None` for a malformed escape or bytes that are not UTF-8.
fn percent_decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The file a request path names under the web directory, as a relative
/// path of plain names only; `None` for anything else.
pub(crate) fn relative(path: &str) -> Option<PathBuf> {
    let decoded = percent_decode(path)?;
    if decoded.contains(['\\', '\0']) {
        return None;
    }
    let mut out = PathBuf::new();
    for segment in decoded.split('/').filter(|s| !s.is_empty()) {
        let mut parts = FsPath::new(segment).components();
        match (parts.next(), parts.next()) {
            (Some(Component::Normal(name)), None) if segment != "." && segment != ".." => {
                out.push(name);
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Paths that belong to the service's JSON routes: a miss there is a 404,
/// never the UI's page.
fn is_api(path: &str) -> bool {
    ["/ui/api", "/reports", "/intake", "/health"]
        .iter()
        .any(|p| path == *p || path.starts_with(&format!("{p}/")))
}

/// The fallback route when a web directory is set.
pub(crate) async fn serve(State(ui): State<Arc<Ui>>, method: Method, uri: Uri) -> Response {
    let not_found = || refuse(StatusCode::NOT_FOUND, "not found").into_response();
    let Some(root) = ui.web_dir() else {
        return not_found();
    };
    if method != Method::GET && method != Method::HEAD {
        return refuse(StatusCode::METHOD_NOT_ALLOWED, "method not allowed").into_response();
    }
    let path = uri.path();
    if is_api(path) {
        return not_found();
    }
    let Some(relative) = relative(path) else {
        return refuse(StatusCode::BAD_REQUEST, "not a path").into_response();
    };
    let wanted = if relative.as_os_str().is_empty() {
        root.join("index.html")
    } else {
        root.join(&relative)
    };
    let file = match tokio::fs::canonicalize(&wanted).await {
        Ok(real) if real.starts_with(root) && real.is_file() => real,
        Ok(_) | Err(_) if relative.extension().is_none() => root.join("index.html"),
        _ => return not_found(),
    };
    let Ok(bytes) = tokio::fs::read(&file).await else {
        return not_found();
    };
    let immutable = file
        .strip_prefix(root)
        .is_ok_and(|p| p.starts_with("assets"));
    let cache = if immutable {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(bytes)
    };
    let mut response = body.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(&file)),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_plain_names_or_nothing() {
        assert_eq!(relative("/"), Some(PathBuf::new()));
        assert_eq!(
            relative("/assets/index-abc.js"),
            Some(PathBuf::from("assets/index-abc.js"))
        );
        assert_eq!(relative("/r/0199"), Some(PathBuf::from("r/0199")));
        for bad in [
            "/../etc/passwd",
            "/assets/../../x",
            "/%2e%2e/x",
            "/%2E%2E%2Fx",
            "/./index.html",
            "/a\\b",
            "/a%5cb",
            "/a%00b",
            "/%zz",
            "/%ff",
        ] {
            assert_eq!(relative(bad), None, "{bad}");
        }
    }

    #[test]
    fn each_built_file_gets_its_type() {
        for (file, kind) in [
            ("index.html", "text/html; charset=utf-8"),
            ("assets/a.js", "text/javascript; charset=utf-8"),
            ("assets/a.css", "text/css; charset=utf-8"),
            ("x.svg", "image/svg+xml"),
            ("x.woff2", "font/woff2"),
            ("x.unknown", "application/octet-stream"),
        ] {
            assert_eq!(content_type(FsPath::new(file)), kind, "{file}");
        }
    }

    #[test]
    fn the_json_routes_never_fall_back_to_the_page() {
        for api in [
            "/ui/api",
            "/ui/api/nothing",
            "/reports",
            "/reports/x",
            "/health",
        ] {
            assert!(is_api(api), "{api}");
        }
        for page in ["/", "/r/abc", "/reportsx", "/ui", "/ui/apix"] {
            assert!(!is_api(page), "{page}");
        }
    }

    #[test]
    fn the_policy_allows_no_inline_script_and_no_framing() {
        assert!(CSP.contains("default-src 'self'"));
        assert!(CSP.contains("script-src 'self';"));
        assert!(!CSP.contains("unsafe-inline"));
        assert!(!CSP.contains("unsafe-eval"));
        assert!(CSP.contains("frame-ancestors 'none'"));
    }

    /// Scryfall's image host is the one foreign origin in the policy, and
    /// only for images: a script, a style, a font or a request to anywhere
    /// else stays refused.
    #[test]
    fn scryfall_is_the_one_foreign_origin_and_only_for_images() {
        let directives: Vec<(&str, &str)> = CSP
            .split(';')
            .map(|d| d.trim().split_once(' ').unwrap_or((d.trim(), "")))
            .collect();
        for (name, sources) in &directives {
            let foreign: Vec<&str> = sources
                .split_whitespace()
                .filter(|s| s.contains("://"))
                .collect();
            if *name == "img-src" {
                assert_eq!(foreign, [SCRYFALL_IMAGES], "{name}");
            } else {
                assert!(foreign.is_empty(), "{name} names {foreign:?}");
            }
        }
        assert!(SCRYFALL_IMAGES.starts_with("https://"));
        assert!(CSP.contains("connect-src 'self';"));
    }
}
