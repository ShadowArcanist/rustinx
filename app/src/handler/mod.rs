pub mod body;
mod file_server;
mod path;
pub mod response;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use hyper::body::Incoming;
use hyper::header;
use hyper::{Request, Response, StatusCode};

use body::ResponseBody;
use crate::config::ResolvedVhost;
use crate::logging::AccessLogger;

pub struct AppState {
    pub vhosts: Vec<ResolvedVhost>,
    pub logger: Option<AccessLogger>,
    pub behind_proxy: bool,
    pub trusted_proxy_depth: usize,
    pub max_file_size: u64,
}

pub async fn handle(
    req: Request<Incoming>,
    state: Arc<AppState>,
    remote_addr: SocketAddr,
) -> Result<Response<ResponseBody>, Infallible> {
    let start = Instant::now();
    let method = req.method().clone();
    let raw_path = req.uri().path();
    let uri_path = if raw_path.len() > 2048 {
        let mut end = 2048;
        while end > 0 && !raw_path.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &raw_path[..end])
    } else {
        raw_path.to_string()
    };

    let resp = dispatch(&req, &state).await;

    if let Some(ref logger) = state.logger {
        let ip = client_ip(
            &req,
            remote_addr,
            state.behind_proxy,
            state.trusted_proxy_depth,
        );
        logger.log(&ip, &method, &uri_path, resp.status().as_u16(), start.elapsed());
    }

    Ok(resp)
}

async fn dispatch(req: &Request<Incoming>, state: &AppState) -> Response<ResponseBody> {
    if req.uri().path().len() > 8192 {
        return response::with_baseline(response::error(
            StatusCode::URI_TOO_LONG,
            "uri too long",
        ));
    }

    let host = match extract_host(req) {
        Some(h) => h,
        None => return response::with_baseline(response::error(StatusCode::BAD_REQUEST, "bad request")),
    };
    match find_vhost(&state.vhosts, &host) {
        Some(vhost) => serve_vhost(req, vhost, state.max_file_size).await,
        None => response::with_baseline(response::error(StatusCode::NOT_FOUND, "not found")),
    }
}

async fn serve_vhost(
    req: &Request<Incoming>,
    vhost: &ResolvedVhost,
    max_file_size: u64,
) -> Response<ResponseBody> {
    let method = req.method();
    let uri_path = req.uri().path();

    // 1. Method allowlist check
    if let Some(ref allowed) = vhost.allowed_methods {
        if !allowed
            .iter()
            .any(|m| m.eq_ignore_ascii_case(method.as_str()))
        {
            return response::add_headers(
                response::method_not_allowed(allowed),
                vhost,
            );
        }
    }

    // 1b. Only GET and HEAD serve static content
    if method != hyper::Method::GET && method != hyper::Method::HEAD {
        return response::add_headers(
            response::method_not_allowed(&["GET".into(), "HEAD".into()]),
            vhost,
        );
    }

    // 2. Decode URI path (single canonical form for ALL routing decisions)
    let decoded = match path::decode(uri_path) {
        Some(d) => d,
        None => {
            return response::add_headers(
                response::error(StatusCode::BAD_REQUEST, "bad request"),
                vhost,
            )
        }
    };

    // 3. Exact-match redirects (on decoded path — prevents /%61dmin bypass)
    if let Some((url, code)) = vhost.redirects.get(decoded.as_str()) {
        let status = StatusCode::from_u16(*code).unwrap_or(StatusCode::MOVED_PERMANENTLY);
        return response::add_headers(response::redirect(status, url), vhost);
    }

    // 4. Routes (prefix match on decoded path)
    for route in &vhost.routes {
        if path::matches_prefix(&decoded, &route.path) {
            let file_path = vhost.root.join(&route.serve);
            if let Some(resp) =
                file_server::serve(&file_path, vhost, req, max_file_size, StatusCode::OK).await
            {
                return resp;
            }
            return response::add_headers(
                response::error(StatusCode::NOT_FOUND, "not found"),
                vhost,
            );
        }
    }

    // 5. Deny all
    if vhost.deny_all {
        return response::add_headers(
            response::error(StatusCode::FORBIDDEN, "forbidden"),
            vhost,
        );
    }

    // 6. Block dot paths (on decoded path — prevents %2E bypass)
    if vhost.block_dot_paths && path::has_dot_segment(&decoded) {
        return response::add_headers(
            response::error(StatusCode::NOT_FOUND, "not found"),
            vhost,
        );
    }

    // 7. Resolve to filesystem path
    let file_path = match path::resolve(&vhost.root, &decoded) {
        Some(p) => p,
        None => {
            return response::add_headers(
                response::error(StatusCode::BAD_REQUEST, "bad request"),
                vhost,
            )
        }
    };

    // 8. Try exact file
    if let Some(resp) =
        file_server::serve(&file_path, vhost, req, max_file_size, StatusCode::OK).await
    {
        return resp;
    }

    // 9. Try directory index
    let index_path = file_path.join(&vhost.index);
    if let Some(resp) =
        file_server::serve(&index_path, vhost, req, max_file_size, StatusCode::OK).await
    {
        return resp;
    }

    // 10. SPA fallback
    if vhost.spa
        && !vhost
            .spa_ignore
            .iter()
            .any(|p| path::matches_prefix(&decoded, p))
    {
        let spa_path = vhost.root.join(&vhost.index);
        if let Some(resp) =
            file_server::serve(&spa_path, vhost, req, max_file_size, StatusCode::OK).await
        {
            return resp;
        }
    }

    // 11. Custom 404
    if let Some(ref custom) = vhost.custom_404 {
        let custom_path = vhost.root.join(custom);
        if let Some(resp) =
            file_server::serve(&custom_path, vhost, req, max_file_size, StatusCode::NOT_FOUND)
                .await
        {
            return resp;
        }
    }

    response::add_headers(
        response::error(StatusCode::NOT_FOUND, "not found"),
        vhost,
    )
}

fn extract_host<T>(req: &Request<T>) -> Option<String> {
    let raw = req.headers().get(header::HOST)?.to_str().ok()?;

    // IPv6 literal ("[::1]" or "[::1]:9090"): only the catch-all can match it
    if let Some(rest) = raw.strip_prefix('[') {
        let (addr, after) = rest.split_once(']')?;
        addr.parse::<std::net::Ipv6Addr>().ok()?;
        if !after.is_empty() && !valid_port(after.strip_prefix(':')?) {
            return None;
        }
        return Some(format!("[{}]", addr.to_lowercase()));
    }

    let (host_part, port_part) = match raw.rfind(':') {
        Some(i) => (&raw[..i], Some(&raw[i + 1..])),
        None => (raw, None),
    };

    if port_part.is_some_and(|p| !valid_port(p)) {
        return None;
    }

    // Reject colons in host part (catches example.com:443:80)
    if host_part.contains(':') {
        return None;
    }

    let host = host_part.to_lowercase();
    let host = host.strip_suffix('.').unwrap_or(&host).to_string();

    if host.is_empty() || host.len() > 253 {
        return None;
    }

    // Validate DNS labels
    for label in host.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return None;
        }
    }

    Some(host)
}

fn valid_port(port: &str) -> bool {
    !port.is_empty()
        && port.bytes().all(|b| b.is_ascii_digit())
        && port.parse::<u16>().is_ok_and(|p| p != 0)
}

fn find_vhost<'a>(vhosts: &'a [ResolvedVhost], host: &str) -> Option<&'a ResolvedVhost> {
    vhosts
        .iter()
        .find(|v| v.hostnames.iter().any(|h| h != "_" && !h.starts_with("*.") && h == host))
        .or_else(|| {
            // Most specific wildcard wins: *.a.example.com before *.example.com
            vhosts
                .iter()
                .filter_map(|v| {
                    v.hostnames
                        .iter()
                        .filter_map(|h| h.strip_prefix("*."))
                        .filter(|root| {
                            host.len() > root.len()
                                && host.ends_with(root)
                                && host.as_bytes()[host.len() - root.len() - 1] == b'.'
                        })
                        .map(str::len)
                        .max()
                        .map(|len| (len, v))
                })
                .max_by_key(|(len, _)| *len)
                .map(|(_, v)| v)
        })
        .or_else(|| {
            vhosts
                .iter()
                .find(|v| v.hostnames.iter().any(|h| h == "_"))
        })
}

fn client_ip<T>(req: &Request<T>, addr: SocketAddr, behind_proxy: bool, depth: usize) -> String {
    if !behind_proxy {
        return addr.ip().to_string();
    }

    let fallback = addr.ip().to_string();
    let depth = depth.max(1);

    // Proxies may append a new header line instead of extending the first one,
    // so the chain is all lines in order (RFC 9110 §5.3)
    let xff = req.headers().get_all("x-forwarded-for");
    if xff.iter().next().is_some() {
        if let Ok(lines) = xff.iter().map(|v| v.to_str()).collect::<Result<Vec<_>, _>>() {
            let parts: Vec<&str> = lines.iter().flat_map(|l| l.split(',')).map(str::trim).collect();
            if parts.len() >= depth {
                let ip = parts[parts.len() - depth];
                if ip.parse::<std::net::IpAddr>().is_ok() {
                    return ip.to_string();
                }
            }
        }
        return fallback;
    }

    if let Some(xri) = req.headers().get("x-real-ip") {
        if let Ok(v) = xri.to_str() {
            if v.parse::<std::net::IpAddr>().is_ok() {
                return v.to_string();
            }
        }
    }

    fallback
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn req_with(headers: &[(&str, &str)]) -> Request<()> {
        let mut b = Request::builder();
        for (k, v) in headers {
            b = b.header(*k, *v);
        }
        b.body(()).unwrap()
    }

    fn vhosts(toml_str: &str) -> Vec<ResolvedVhost> {
        toml::from_str::<Config>(toml_str).unwrap().resolve().unwrap()
    }

    #[test]
    fn host_ipv6_literal() {
        assert_eq!(extract_host(&req_with(&[("host", "[::1]:9090")])).unwrap(), "[::1]");
        assert_eq!(extract_host(&req_with(&[("host", "[::1]")])).unwrap(), "[::1]");
        assert!(extract_host(&req_with(&[("host", "[::1]:0")])).is_none());
        assert!(extract_host(&req_with(&[("host", "[nope]")])).is_none());
    }

    #[test]
    fn host_name_with_port() {
        assert_eq!(extract_host(&req_with(&[("host", "Example.COM.:443")])).unwrap(), "example.com");
        assert!(extract_host(&req_with(&[("host", "example.com:443:80")])).is_none());
    }

    #[test]
    fn most_specific_wildcard_wins() {
        let v = vhosts(r#"
            [[vhost]]
            hostnames = ["*.example.com"]
            root = "/tmp"
            index = "broad.html"
            [[vhost]]
            hostnames = ["*.app.example.com"]
            root = "/tmp"
            index = "narrow.html"
        "#);
        assert_eq!(find_vhost(&v, "x.app.example.com").unwrap().index, "narrow.html");
        assert_eq!(find_vhost(&v, "x.example.com").unwrap().index, "broad.html");
        assert!(find_vhost(&v, "example.com").is_none());
    }

    #[test]
    fn ipv6_host_uses_catch_all() {
        let v = vhosts(r#"
            [[vhost]]
            hostnames = ["_"]
            root = "/tmp"
        "#);
        assert!(find_vhost(&v, "[::1]").is_some());
    }

    #[test]
    fn xff_reads_all_header_lines() {
        let addr: SocketAddr = "10.0.0.1:1234".parse().unwrap();
        let req = req_with(&[
            ("x-forwarded-for", "6.6.6.6"),
            ("x-forwarded-for", "1.2.3.4"),
        ]);
        // The last entry is the one the trusted proxy appended
        assert_eq!(client_ip(&req, addr, true, 1), "1.2.3.4");
        assert_eq!(client_ip(&req, addr, true, 2), "6.6.6.6");
        assert_eq!(client_ip(&req, addr, true, 3), "10.0.0.1");
    }
}
