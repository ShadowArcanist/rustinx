use std::path::Path;

use hyper::body::Incoming;
use hyper::header;
use hyper::{Method, Request, Response, StatusCode};

use super::body::ResponseBody;
use crate::config::ResolvedVhost;
use crate::mime_types;
use super::response;

/// Serves `file_path` with `status`. Only 200 responses get an ETag and can
/// become 304: an error page (custom 404) must always send its body.
pub async fn serve(
    file_path: &Path,
    vhost: &ResolvedVhost,
    req: &Request<Incoming>,
    max_file_size: u64,
    status: StatusCode,
) -> Option<Response<ResponseBody>> {
    let canonical = tokio::fs::canonicalize(file_path).await.ok()?;

    if !canonical.starts_with(&vhost.root) {
        return None;
    }

    // Reject symlinks that resolve to dotfiles when block_dot_paths is enabled
    if vhost.block_dot_paths {
        if let Ok(relative) = canonical.strip_prefix(&vhost.root) {
            for component in relative.components() {
                if let std::path::Component::Normal(s) = component {
                    let s = s.to_string_lossy();
                    if s.starts_with('.') && s.len() > 1 {
                        return None;
                    }
                }
            }
        }
    }

    // Stat before open: open() on a FIFO blocks until a writer appears
    if !tokio::fs::metadata(&canonical).await.ok()?.is_file() {
        return None;
    }

    let file = tokio::fs::File::open(&canonical).await.ok()?;
    let meta = file.metadata().await.ok()?;

    if !meta.is_file() {
        return None;
    }

    if meta.len() > max_file_size {
        return Some(response::add_headers(
            response::error(StatusCode::PAYLOAD_TOO_LARGE, "file too large"),
            vhost,
        ));
    }

    let etag = if vhost.etag && status == StatusCode::OK {
        Some(make_etag(&meta))
    } else {
        None
    };

    // ETag 304 (no file read for either GET or HEAD)
    if let Some(ref etag) = etag {
        let not_modified = req
            .headers()
            .get_all(header::IF_NONE_MATCH)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .any(|v| etag_matches(v, etag));
        if not_modified {
            return Some(response::add_headers(
                Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .header(header::ETAG, etag.as_str())
                    .body(ResponseBody::empty())
                    .unwrap(),
                vhost,
            ));
        }
    }

    let mime = mime_types::from_path(&canonical);
    let size = meta.len();

    // HEAD: metadata only, no file read
    if req.method() == Method::HEAD {
        let mut builder = Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CONTENT_LENGTH, size);
        if let Some(ref etag) = etag {
            builder = builder.header(header::ETAG, etag.as_str());
        }
        return Some(response::add_headers(
            builder.body(ResponseBody::empty()).unwrap(),
            vhost,
        ));
    }

    // GET: stream file (8KB chunks, not buffered in memory)
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, size);

    if let Some(ref etag) = etag {
        builder = builder.header(header::ETAG, etag.as_str());
    }

    Some(response::add_headers(
        builder.body(ResponseBody::stream(file, size)).unwrap(),
        vhost,
    ))
}

/// Weak comparison (RFC 9110 §13.1.2): `*` or any listed tag, `W/` ignored.
fn etag_matches(if_none_match: &str, etag: &str) -> bool {
    if_none_match
        .split(',')
        .map(str::trim)
        .any(|t| t == "*" || t.strip_prefix("W/").unwrap_or(t) == etag)
}

#[cfg(unix)]
fn make_etag(meta: &std::fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!(
        "\"{:x}-{:x}-{:x}-{:x}\"",
        meta.mtime(),
        meta.mtime_nsec(),
        meta.len(),
        meta.ino()
    )
}

#[cfg(not(unix))]
fn make_etag(meta: &std::fs::Metadata) -> String {
    use std::time::UNIX_EPOCH;
    let mtime = meta
        .modified()
        .unwrap_or(UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("\"{:x}-{:x}\"", mtime, meta.len())
}

#[cfg(test)]
mod tests {
    use super::etag_matches;

    #[test]
    fn etag_match_forms() {
        let etag = "\"a-1\"";
        assert!(etag_matches("\"a-1\"", etag));
        assert!(etag_matches("W/\"a-1\"", etag));
        assert!(etag_matches("\"x\", \"a-1\"", etag));
        assert!(etag_matches("*", etag));
        assert!(!etag_matches("\"a-2\"", etag));
        assert!(!etag_matches("", etag));
    }
}
