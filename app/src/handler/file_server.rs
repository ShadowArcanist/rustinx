use std::path::Path;

use hyper::body::Incoming;
use hyper::header;
use hyper::{Method, Request, Response, StatusCode};

use super::body::ResponseBody;
use crate::config::ResolvedVhost;
use crate::mime_types;
use super::response;

pub async fn serve(
    file_path: &Path,
    vhost: &ResolvedVhost,
    req: &Request<Incoming>,
    max_file_size: u64,
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

    let etag = if vhost.etag {
        Some(make_etag(&meta))
    } else {
        None
    };

    // ETag 304 (no file read for either GET or HEAD)
    if let Some(ref etag) = etag {
        if let Some(inm) = req.headers().get(header::IF_NONE_MATCH) {
            if let Ok(v) = inm.to_str() {
                if v == etag.as_str() {
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
        }
    }

    let mime = mime_types::from_path(&canonical);
    let size = meta.len();

    // HEAD: metadata only, no file read
    if req.method() == Method::HEAD {
        let mut builder = Response::builder()
            .status(StatusCode::OK)
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
        .status(StatusCode::OK)
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
