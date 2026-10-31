use hyper::header::{HeaderName, HeaderValue};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub vhost: Vec<VhostConfig>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default)]
    pub behind_proxy: bool,
    #[serde(default = "default_proxy_depth")]
    pub trusted_proxy_depth: usize,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            behind_proxy: false,
            trusted_proxy_depth: 1,
            max_file_size: default_max_file_size(),
            max_connections: default_max_connections(),
            connection_timeout: default_connection_timeout(),
        }
    }
}

fn default_listen() -> String {
    "0.0.0.0:9090".into()
}
fn default_true() -> bool {
    true
}
fn default_proxy_depth() -> usize {
    1
}
fn default_max_file_size() -> u64 {
    50 * 1024 * 1024
}
fn default_max_connections() -> usize {
    1024
}
fn default_connection_timeout() -> u64 {
    30
}

fn default_allowed_methods() -> Option<Vec<String>> {
    Some(vec!["GET".into(), "HEAD".into()])
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_log_path")]
    pub path: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_timezone() -> String {
    "UTC".into()
}

fn default_log_path() -> String {
    "/var/log/rustinx/access.log".into()
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DefaultsConfig {
    #[serde(default = "default_true")]
    pub block_dot_paths: bool,
    #[serde(default = "default_true")]
    pub etag: bool,
    #[serde(default = "default_allowed_methods")]
    pub allowed_methods: Option<Vec<String>>,
    #[serde(default = "default_security_headers")]
    pub headers: HashMap<String, String>,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            block_dot_paths: true,
            etag: true,
            allowed_methods: default_allowed_methods(),
            headers: default_security_headers(),
        }
    }
}

fn default_security_headers() -> HashMap<String, String> {
    let mut h = HashMap::with_capacity(5);
    h.insert("X-Content-Type-Options".into(), "nosniff".into());
    h.insert("X-Frame-Options".into(), "DENY".into());
    h.insert(
        "Content-Security-Policy".into(),
        "default-src 'self'; frame-ancestors 'none'".into(),
    );
    h.insert(
        "Strict-Transport-Security".into(),
        "max-age=31536000; includeSubDomains; preload".into(),
    );
    h.insert(
        "Referrer-Policy".into(),
        "strict-origin-when-cross-origin".into(),
    );
    h
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct VhostConfig {
    pub hostnames: Vec<String>,
    pub root: String,
    #[serde(default = "default_index")]
    pub index: String,
    #[serde(default)]
    pub spa: bool,
    #[serde(default)]
    pub spa_ignore: Vec<String>,
    pub block_dot_paths: Option<bool>,
    pub etag: Option<bool>,
    #[serde(default)]
    pub allowed_methods: Option<Vec<String>>,
    #[serde(default)]
    pub deny_all: bool,
    #[serde(default)]
    pub custom_404: Option<String>,
    #[serde(default)]
    pub redirects_file: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub redirects: Vec<RedirectEntry>,
    #[serde(default)]
    pub routes: Vec<RouteEntry>,
}

fn default_index() -> String {
    "index.html".into()
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RedirectEntry {
    pub path: String,
    pub url: String,
    #[serde(default = "default_redirect_code")]
    pub code: u16,
}

fn default_redirect_code() -> u16 {
    301
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RouteEntry {
    pub path: String,
    pub serve: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RedirectFile {
    #[serde(default)]
    redirect: Vec<RedirectEntry>,
}

pub struct ResolvedVhost {
    pub hostnames: Vec<String>,
    pub root: PathBuf,
    pub index: String,
    pub spa: bool,
    pub spa_ignore: Vec<String>,
    pub block_dot_paths: bool,
    pub etag: bool,
    pub allowed_methods: Option<Vec<String>>,
    pub deny_all: bool,
    pub custom_404: Option<String>,
    pub headers: Vec<(HeaderName, HeaderValue)>,
    pub redirects: HashMap<String, (String, u16)>,
    pub routes: Vec<RouteEntry>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            defaults: DefaultsConfig::default(),
            vhost: vec![VhostConfig {
                hostnames: vec!["_".into()],
                root: "/static".into(),
                index: "index.html".into(),
                spa: false,
                spa_ignore: vec![],
                block_dot_paths: None,
                etag: None,
                allowed_methods: None,
                deny_all: false,
                custom_404: None,
                redirects_file: None,
                headers: HashMap::new(),
                redirects: vec![],
                routes: vec![],
            }],
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self, String> {
        check_file_size(path, 1024 * 1024, "config")?;
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
        toml::from_str(&content).map_err(|e| format!("parse config: {e}"))
    }

    pub fn resolve(&self) -> Result<Vec<ResolvedVhost>, String> {
        validate_server_limits(&self.server)?;

        if let Some(ref methods) = self.defaults.allowed_methods {
            validate_methods(methods)?;
        }

        if self.vhost.len() > 100 {
            return Err(format!("too many vhosts: {} (max 100)", self.vhost.len()));
        }

        let resolved: Vec<ResolvedVhost> = self
            .vhost
            .iter()
            .map(|v| self.resolve_vhost(v))
            .collect::<Result<Vec<_>, _>>()?;

        // Check for duplicate hostnames
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for (i, v) in resolved.iter().enumerate() {
            for h in &v.hostnames {
                if let Some(prev) = seen.insert(h.as_str(), i) {
                    return Err(format!(
                        "hostname '{h}' duplicated in vhost #{prev} and #{i}"
                    ));
                }
            }
        }

        Ok(resolved)
    }

    fn resolve_vhost(&self, vhost: &VhostConfig) -> Result<ResolvedVhost, String> {
        // Collection count caps
        if vhost.hostnames.is_empty() {
            return Err("vhost must have at least one hostname".into());
        }
        if vhost.hostnames.len() > 50 {
            return Err(format!("too many hostnames: {} (max 50)", vhost.hostnames.len()));
        }
        if vhost.routes.len() > 1000 {
            return Err(format!("too many routes: {} (max 1000)", vhost.routes.len()));
        }
        if vhost.redirects.len() > 10000 {
            return Err(format!("too many inline redirects: {} (max 10000)", vhost.redirects.len()));
        }
        if vhost.spa_ignore.len() > 100 {
            return Err(format!("too many spa_ignore entries: {} (max 100)", vhost.spa_ignore.len()));
        }
        for h in &vhost.hostnames {
            validate_hostname(h)?;
        }

        // Validate vhost-level methods
        if let Some(ref methods) = vhost.allowed_methods {
            validate_methods(methods)?;
        }

        // Validate relative paths (no traversal, no dotfiles via config)
        validate_relative_path("index", &vhost.index)?;
        validate_no_dot_segments("index", &vhost.index)?;
        if let Some(ref p) = vhost.custom_404 {
            validate_relative_path("custom_404", p)?;
            validate_no_dot_segments("custom_404", p)?;
        }
        for r in &vhost.routes {
            validate_relative_path("routes.serve", &r.serve)?;
            validate_no_dot_segments("routes.serve", &r.serve)?;
            validate_match_path("routes.path", &r.path)?;
        }

        // Validate spa_ignore entries
        for entry in &vhost.spa_ignore {
            validate_match_path("spa_ignore", entry)?;
        }

        // Normalize "/api/" to "/api" so prefix matching also covers "/api/x"
        let mut routes: Vec<RouteEntry> = vhost
            .routes
            .iter()
            .map(|r| RouteEntry {
                path: trim_trailing_slash(&r.path),
                serve: r.serve.clone(),
            })
            .collect();
        let spa_ignore: Vec<String> =
            vhost.spa_ignore.iter().map(|p| trim_trailing_slash(p)).collect();

        // Validate no duplicate route paths
        {
            let mut seen = std::collections::HashSet::new();
            for r in &routes {
                if !seen.insert(&r.path) {
                    return Err(format!("duplicate route path: '{}'", r.path));
                }
            }
        }
        // Longest first, so the most specific route wins
        routes.sort_by_key(|r| std::cmp::Reverse(r.path.len()));

        // Canonicalize root at startup (fail fast if inaccessible)
        let root = std::fs::canonicalize(&vhost.root)
            .map_err(|e| format!("root '{}': {e}", vhost.root))?;

        // Reject dangerously broad roots
        if root.components().count() < 2 {
            return Err(format!(
                "root '{}' is too broad — would expose the entire filesystem",
                vhost.root
            ));
        }
        if is_blocked_root(&root) {
            return Err(format!(
                "root '{}' is a system directory — use a dedicated static asset path",
                root.display()
            ));
        }

        // Load + validate redirects
        let mut redirects = HashMap::new();

        if let Some(ref path) = vhost.redirects_file {
            check_file_size(path, 10 * 1024 * 1024, "redirects")?;
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("read redirects {path}: {e}"))?;
            let file: RedirectFile = toml::from_str(&content)
                .map_err(|e| format!("parse redirects {path}: {e}"))?;
            if file.redirect.len() > 10000 {
                return Err(format!(
                    "redirect file '{path}' has too many entries: {} (max 10000)",
                    file.redirect.len()
                ));
            }
            for r in file.redirect {
                validate_redirect(&r)?;
                validate_match_path("redirect.path", &r.path)?;
                if redirects.insert(r.path.clone(), (r.url, r.code)).is_some() {
                    return Err(format!("duplicate redirect path: '{}'", r.path));
                }
            }
        }

        for r in &vhost.redirects {
            validate_redirect(r)?;
            validate_match_path("redirect.path", &r.path)?;
            if redirects
                .insert(r.path.clone(), (r.url.clone(), r.code))
                .is_some()
            {
                return Err(format!("duplicate redirect path: '{}'", r.path));
            }
        }

        // Check for case-insensitive duplicate header keys within each source
        check_header_case_dups(&self.defaults.headers, "defaults.headers")?;
        check_header_case_dups(&vhost.headers, "vhost.headers")?;

        // Merge and pre-parse headers (fail on invalid headers at startup)
        let mut raw_headers: Vec<(String, String)> = self
            .defaults
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (k, v) in &vhost.headers {
            if let Some(entry) = raw_headers
                .iter_mut()
                .find(|(ek, _)| ek.eq_ignore_ascii_case(k))
            {
                entry.1 = v.clone();
            } else {
                raw_headers.push((k.clone(), v.clone()));
            }
        }

        if raw_headers.len() > 100 {
            return Err(format!(
                "too many headers after merge: {} (max 100)",
                raw_headers.len()
            ));
        }

        let headers: Vec<(HeaderName, HeaderValue)> = raw_headers
            .iter()
            .map(|(k, v)| {
                let lower = k.to_lowercase();
                if is_reserved_header(&lower) {
                    return Err(format!(
                        "header '{k}' is reserved and cannot be set via config"
                    ));
                }
                if is_security_header(&lower) {
                    validate_security_value(&lower, v, k)?;
                }
                let name = HeaderName::from_bytes(lower.as_bytes())
                    .map_err(|_| format!("invalid header name: {k}"))?;
                let value = HeaderValue::from_str(v)
                    .map_err(|_| format!("invalid header value for {k}: {v}"))?;
                Ok((name, value))
            })
            .collect::<Result<Vec<_>, String>>()?;

        // Reject unsafe CORS combination
        let has_cors_wildcard = headers.iter().any(|(n, v)| {
            n.as_str() == "access-control-allow-origin" && v == "*"
        });
        let has_cors_creds = headers.iter().any(|(n, v)| {
            n.as_str() == "access-control-allow-credentials" && v == "true"
        });
        if has_cors_wildcard && has_cors_creds {
            return Err(
                "access-control-allow-origin '*' with credentials 'true' is unsafe"
                    .into(),
            );
        }
        let has_cors_pna = headers.iter().any(|(n, v)| {
            n.as_str() == "access-control-allow-private-network" && v == "true"
        });
        if has_cors_wildcard && has_cors_pna {
            return Err(
                "access-control-allow-origin '*' with private-network 'true' is unsafe"
                    .into(),
            );
        }
        let has_cors_null = headers.iter().any(|(n, v)| {
            n.as_str() == "access-control-allow-origin" && v == "null"
        });
        if has_cors_null {
            return Err(
                "access-control-allow-origin 'null' is unsafe — use explicit origins"
                    .into(),
            );
        }

        Ok(ResolvedVhost {
            hostnames: vhost.hostnames.iter().map(|h| h.to_lowercase()).collect(),
            root,
            index: vhost.index.clone(),
            spa: vhost.spa,
            spa_ignore,
            block_dot_paths: vhost
                .block_dot_paths
                .unwrap_or(self.defaults.block_dot_paths),
            etag: vhost.etag.unwrap_or(self.defaults.etag),
            allowed_methods: vhost
                .allowed_methods
                .as_ref()
                .or(self.defaults.allowed_methods.as_ref())
                .map(|ms| ms.iter().map(|m| m.to_ascii_uppercase()).collect()),
            deny_all: vhost.deny_all,
            custom_404: vhost.custom_404.clone(),
            headers,
            redirects,
            routes,
        })
    }
}

fn trim_trailing_slash(path: &str) -> String {
    match path.trim_end_matches('/') {
        "" => "/".into(),
        p => p.into(),
    }
}

fn validate_relative_path(name: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{name} cannot be empty"));
    }
    if value.contains("..") || value.starts_with('/') || value.starts_with('\\') {
        return Err(format!(
            "{name} must be relative without '..': '{value}'"
        ));
    }
    if value.contains('\\') || value.contains('\0') || value.chars().any(|c| c.is_control()) {
        return Err(format!(
            "{name} contains invalid characters: '{value}'"
        ));
    }
    Ok(())
}

fn validate_no_dot_segments(name: &str, value: &str) -> Result<(), String> {
    for seg in value.split('/') {
        if seg.starts_with('.') && seg.len() > 1 {
            return Err(format!(
                "{name} contains dotfile segment '{seg}' — not allowed with block_dot_paths"
            ));
        }
    }
    Ok(())
}

fn validate_match_path(name: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || !value.starts_with('/') {
        return Err(format!(
            "{name} must be an absolute path starting with '/': '{value}'"
        ));
    }
    if value.contains('\\') || value.chars().any(|c| c.is_control()) {
        return Err(format!("{name} contains invalid characters: '{value}'"));
    }
    for seg in value.split('/') {
        if seg == ".." || seg == "." {
            return Err(format!("{name} contains traversal segment: '{value}'"));
        }
        if seg.starts_with('.') && seg.len() > 1 {
            return Err(format!(
                "{name} contains dot segment '{seg}' — use explicit routes for .well-known"
            ));
        }
    }
    Ok(())
}

fn validate_redirect(r: &RedirectEntry) -> Result<(), String> {
    // 300/304/305/306 are not usable redirects (304 means "not modified")
    if !matches!(r.code, 301 | 302 | 303 | 307 | 308) {
        return Err(format!(
            "redirect '{}' has invalid status code {} (must be 301, 302, 303, 307 or 308)",
            r.path, r.code
        ));
    }
    if !(r.url.starts_with("https://") || r.url.starts_with('/'))
    {
        return Err(format!(
            "redirect '{}' has invalid URL '{}' (must be https:// or /path)",
            r.path, r.url
        ));
    }
    if r.url.starts_with("//") || r.url.starts_with("/\\") {
        return Err(format!(
            "redirect '{}' has protocol-relative URL '{}' (use explicit scheme)",
            r.path, r.url
        ));
    }
    // Reject backslashes in any redirect URL
    if r.url.contains('\\') {
        return Err(format!(
            "redirect '{}' URL contains backslash: '{}'",
            r.path, r.url
        ));
    }
    // Reject encoded protocol-relative forms and double-encoding
    if r.url.starts_with('/') {
        let lower = r.url.to_lowercase();
        if lower.starts_with("/%2f")
            || lower.starts_with("/%5c")
            || lower.starts_with("/%25")
        {
            return Err(format!(
                "redirect '{}' URL contains encoded/double-encoded prefix",
                r.path
            ));
        }
    }
    // Reject embedded credentials in absolute URLs
    if r.url.starts_with("https://") {
        let authority = &r.url[8..];
        let authority_end = authority.find('/').unwrap_or(authority.len());
        let auth = &authority[..authority_end];
        if auth.contains('@') {
            return Err(format!(
                "redirect '{}' URL contains embedded credentials",
                r.path
            ));
        }
        let host = auth.split(':').next().unwrap_or(auth);
        if host.is_empty()
            || host.starts_with('-')
            || host.ends_with('-')
            || host.contains("..")
        {
            return Err(format!(
                "redirect '{}' URL has invalid host: '{host}'",
                r.path
            ));
        }
    }
    // Reject percent-encoded controls in redirect URLs
    reject_encoded_controls(&r.url, &r.path)?;
    HeaderValue::from_str(&r.url).map_err(|_| {
        format!(
            "redirect '{}' URL contains control characters: '{}'",
            r.path, r.url
        )
    })?;
    Ok(())
}

fn validate_methods(methods: &[String]) -> Result<(), String> {
    if methods.is_empty() {
        return Err("allowed_methods cannot be empty".into());
    }
    let mut seen = std::collections::HashSet::new();
    for m in methods {
        let upper = m.to_ascii_uppercase();
        if upper != "GET" && upper != "HEAD" {
            return Err(format!(
                "method '{m}' not supported — only GET and HEAD are valid for static serving"
            ));
        }
        if !seen.insert(upper) {
            return Err(format!("duplicate method: '{m}'"));
        }
    }
    Ok(())
}

fn validate_server_limits(s: &ServerConfig) -> Result<(), String> {
    if !(1..=4096).contains(&s.max_connections) {
        return Err(format!("max_connections must be 1-4096, got {}", s.max_connections));
    }
    if !(1..=3600).contains(&s.connection_timeout) {
        return Err(format!("connection_timeout must be 1-3600s, got {}", s.connection_timeout));
    }
    if s.max_file_size < 1 || s.max_file_size > 1024 * 1024 * 1024 {
        return Err(format!("max_file_size must be 1-1073741824 (1GB), got {}", s.max_file_size));
    }
    if !(1..=16).contains(&s.trusted_proxy_depth) {
        return Err(format!("trusted_proxy_depth must be 1-16, got {}", s.trusted_proxy_depth));
    }
    Ok(())
}

fn validate_hostname(h: &str) -> Result<(), String> {
    if h == "_" {
        return Ok(());
    }
    if let Some(rest) = h.strip_prefix("*.") {
        if !rest.contains('.') {
            return Err(format!(
                "wildcard '{h}' too broad — need at least two labels (e.g. *.example.com)"
            ));
        }
        return validate_dns_name(rest)
            .map_err(|e| format!("wildcard '{h}': {e}"));
    }
    validate_dns_name(h).map_err(|e| format!("hostname '{h}': {e}"))
}

fn validate_dns_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 253 {
        return Err("invalid length".into());
    }
    for label in name.split('.') {
        if label.is_empty() {
            return Err("empty label".into());
        }
        if label.len() > 63 {
            return Err(format!("label '{label}' exceeds 63 chars"));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(format!("label '{label}' has leading/trailing hyphen"));
        }
        if !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(format!("label '{label}' contains invalid characters"));
        }
    }
    Ok(())
}

fn is_reserved_header(name: &str) -> bool {
    matches!(
        name,
        "content-length"
            | "transfer-encoding"
            | "connection"
            | "keep-alive"
            | "upgrade"
            | "te"
            | "trailer"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "host"
            | "date"
            | "location"
            | "allow"
            | "etag"
            | "content-type"
            | "set-cookie"
            | "set-cookie2"
    )
}

fn is_security_header(name: &str) -> bool {
    matches!(
        name,
        "content-security-policy"
            | "x-frame-options"
            | "x-content-type-options"
            | "strict-transport-security"
            | "referrer-policy"
    )
}

fn is_blocked_root(path: &std::path::Path) -> bool {
    const BLOCKED: &[&str] = &[
        "/etc", "/var", "/usr", "/bin", "/sbin", "/proc", "/sys", "/dev",
        "/boot", "/lib", "/lib64", "/run", "/root",
        "/private/etc", "/private/var",
        "/System", "/Applications", "/Library", "/cores",
        "/Users", "/home",
    ];
    let s = path.to_string_lossy();
    BLOCKED.iter().any(|b| s.as_ref() == *b || s.starts_with(&format!("{b}/")))
}

fn validate_csp_source(src: &str) -> Result<(), String> {
    if src.starts_with('\'') {
        if !src.ends_with('\'') || src.len() < 3 {
            return Err(format!("malformed quoted CSP source: {src}"));
        }
        let inner = &src[1..src.len() - 1];
        const KNOWN: &[&str] = &[
            "self",
            "none",
            "strict-dynamic",
            "report-sample",
            "wasm-unsafe-eval",
        ];
        if KNOWN.contains(&inner) {
            return Ok(());
        }
        if inner.starts_with("nonce-") || inner.starts_with("sha256-")
            || inner.starts_with("sha384-") || inner.starts_with("sha512-")
        {
            let val = inner.split_once('-').map(|x| x.1).unwrap_or("");
            if !val.is_empty()
                && val.bytes().all(|b| {
                    b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'='
                })
            {
                return Ok(());
            }
            return Err(format!("malformed CSP nonce/hash: {src}"));
        }
        return Err(format!("unknown CSP keyword: {src}"));
    }
    if src.starts_with("nonce-") || src.starts_with("sha256-")
        || src.starts_with("sha384-") || src.starts_with("sha512-")
    {
        return Err(format!("CSP nonce/hash must be quoted: '{src}'"));
    }
    if !src.starts_with("https://") {
        return Err(format!(
            "CSP source '{src}' must be a quoted keyword or https:// origin"
        ));
    }
    let host_and_rest = &src[8..];
    let host = host_and_rest.split('/').next().unwrap_or(host_and_rest);
    let host = host.split(':').next().unwrap_or(host);
    if host.is_empty()
        || host.starts_with('-')
        || host.ends_with('-')
        || host.contains("..")
        || host.contains('@')
        || host.contains('\\')
    {
        return Err(format!("CSP source '{src}' has invalid host"));
    }
    Ok(())
}

fn check_header_case_dups(
    headers: &HashMap<String, String>,
    source: &str,
) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for k in headers.keys() {
        if !seen.insert(k.to_lowercase()) {
            return Err(format!(
                "duplicate header in {source} (case-insensitive): '{k}'"
            ));
        }
    }
    Ok(())
}

fn reject_encoded_controls(url: &str, path: &str) -> Result<(), String> {
    let bytes = url.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(s) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(s, 16) {
                    if val <= 0x1f || val == 0x7f {
                        return Err(format!(
                            "redirect '{path}' URL contains encoded control character"
                        ));
                    }
                }
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    Ok(())
}

fn check_file_size(path: &str, max: u64, label: &str) -> Result<(), String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("stat {path}: {e}"))?;
    if !meta.is_file() {
        return Err(format!("{label} '{path}' is not a regular file"));
    }
    if meta.len() > max {
        return Err(format!(
            "{label} file '{path}' too large ({} bytes, max {max})",
            meta.len()
        ));
    }
    Ok(())
}

fn validate_security_value(lower: &str, value: &str, display: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("security header '{display}' cannot be empty"));
    }
    match lower {
        "x-frame-options" => {
            if !matches!(value, "DENY" | "SAMEORIGIN") {
                return Err(format!(
                    "x-frame-options must be 'DENY' or 'SAMEORIGIN', got '{value}'"
                ));
            }
        }
        "x-content-type-options" => {
            if value != "nosniff" {
                return Err(format!(
                    "x-content-type-options must be 'nosniff', got '{value}'"
                ));
            }
        }
        "referrer-policy" => {
            const VALID: &[&str] = &[
                "no-referrer",
                "no-referrer-when-downgrade",
                "origin",
                "origin-when-cross-origin",
                "same-origin",
                "strict-origin",
                "strict-origin-when-cross-origin",
            ];
            if !VALID.contains(&value) {
                return Err(format!(
                    "referrer-policy '{value}' is not allowed (unsafe-url is blocked)"
                ));
            }
        }
        "strict-transport-security" => {
            let lower_val = value.to_lowercase();
            let max_age_count = lower_val
                .split(';')
                .map(|d| d.trim())
                .filter(|d| d.starts_with("max-age="))
                .count();
            if max_age_count == 0 {
                return Err(
                    "strict-transport-security must contain max-age".into(),
                );
            }
            if max_age_count > 1 {
                return Err(
                    "strict-transport-security has duplicate max-age directives"
                        .into(),
                );
            }
            let valid = lower_val
                .split(';')
                .map(|d| d.trim())
                .filter(|d| d.starts_with("max-age="))
                .all(|d| {
                    d.strip_prefix("max-age=")
                        .and_then(|v| v.trim().parse::<u64>().ok())
                        .is_some_and(|v| v > 0)
                });
            if !valid {
                return Err(
                    "strict-transport-security max-age must be a positive number"
                        .into(),
                );
            }
        }
        "content-security-policy" => {
            let lower_val = value.to_lowercase();
            let directives: Vec<(&str, &str)> = lower_val
                .split(';')
                .filter_map(|d| {
                    let d = d.trim();
                    if d.is_empty() {
                        return None;
                    }
                    let mut parts = d.splitn(2, |c: char| c.is_ascii_whitespace());
                    let name = parts.next()?;
                    let val = parts.next().unwrap_or("");
                    Some((name, val))
                })
                .collect();

            // Reject duplicate directives
            {
                let mut seen = std::collections::HashSet::new();
                for (name, _) in &directives {
                    if !seen.insert(*name) {
                        return Err(format!(
                            "content-security-policy has duplicate directive: '{name}'"
                        ));
                    }
                }
            }

            let has_default_src = directives.iter().any(|(n, _)| *n == "default-src");
            if !has_default_src {
                return Err(
                    "content-security-policy must contain a 'default-src' directive"
                        .into(),
                );
            }

            let has_frame_ancestors =
                directives.iter().any(|(n, _)| *n == "frame-ancestors");
            if !has_frame_ancestors {
                return Err(
                    "content-security-policy must contain a 'frame-ancestors' directive"
                        .into(),
                );
            }

            const UNSAFE_SOURCES: &[&str] = &[
                "*",
                "'unsafe-inline'",
                "'unsafe-eval'",
                "'unsafe-hashes'",
                "data:",
                "blob:",
                "filesystem:",
                "http:",
                "https:",
                "ws:",
                "wss:",
            ];
            const CHECKED_DIRECTIVES: &[&str] = &[
                "default-src",
                "script-src",
                "script-src-elem",
                "script-src-attr",
                "style-src",
                "object-src",
                "base-uri",
                "worker-src",
                "form-action",
                "connect-src",
                "img-src",
                "frame-src",
                "child-src",
                "media-src",
                "font-src",
                "manifest-src",
                "prefetch-src",
                "navigate-to",
            ];

            for (name, val) in &directives {
                if CHECKED_DIRECTIVES.contains(name) {
                    let sources: Vec<&str> =
                        val.split_ascii_whitespace().collect();
                    if sources.is_empty() {
                        return Err(format!(
                            "content-security-policy '{name}' must have at least one source"
                        ));
                    }
                    for s in &sources {
                        if UNSAFE_SOURCES.contains(s) {
                            return Err(format!(
                                "content-security-policy '{name}' must not contain {s}"
                            ));
                        }
                        if s.contains('*') {
                            return Err(format!(
                                "content-security-policy '{name}' must not contain wildcard source '{s}'"
                            ));
                        }
                        validate_csp_source(s)?;
                    }
                }
                if *name == "frame-ancestors" {
                    let sources: Vec<&str> = val.split_ascii_whitespace().collect();
                    if sources.is_empty() {
                        return Err(
                            "content-security-policy frame-ancestors must have a value"
                                .into(),
                        );
                    }
                    if sources.contains(&"'none'") && sources.len() > 1 {
                        return Err(
                            "content-security-policy frame-ancestors 'none' cannot be combined with other sources"
                                .into(),
                        );
                    }
                    if !sources.iter().all(|s| *s == "'none'" || *s == "'self'") {
                        return Err(
                            "content-security-policy frame-ancestors must be 'none' or 'self'"
                                .into(),
                        );
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect_err(toml_str: &str) -> String {
        let cfg: Config = match toml::from_str(toml_str) {
            Ok(c) => c,
            Err(e) => return e.to_string(),
        };
        match cfg.resolve() {
            Ok(_) => panic!("expected error but config resolved successfully"),
            Err(e) => e,
        }
    }

    #[test]
    fn reject_wildcard_too_broad() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["*.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("too broad"));
    }

    #[test]
    fn reject_invalid_hostname_chars() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["bad/host.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("invalid characters"));
    }

    #[test]
    fn reject_empty_hostnames() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = []
            root = "/tmp"
        "#);
        assert!(r.contains("at least one hostname"));
    }

    #[test]
    fn reject_duplicate_hostnames() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("duplicated"));
    }

    #[test]
    fn reject_traversal_in_index() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            index = "../etc/passwd"
        "#);
        assert!(r.contains("relative without"));
    }

    #[test]
    fn reject_traversal_in_custom_404() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            custom_404 = "../../etc/shadow"
        "#);
        assert!(r.contains("relative without"));
    }

    #[test]
    fn reject_invalid_redirect_code() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            [[vhost.redirects]]
            path = "/x"
            url = "https://example.com"
            code = 200
        "#);
        assert!(r.contains("status code"));
    }

    #[test]
    fn reject_javascript_redirect() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            [[vhost.redirects]]
            path = "/x"
            url = "javascript:alert(1)"
        "#);
        assert!(r.contains("must be http"));
    }

    #[test]
    fn reject_unsupported_method() {
        let r = expect_err(r#"
            [defaults]
            allowed_methods = ["OPTIONS"]
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("not supported"), "unsupported method: {r}");
    }

    #[test]
    fn reject_duplicate_methods() {
        let r = expect_err(r#"
            [defaults]
            allowed_methods = ["GET", "get"]
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("duplicate"));
    }

    #[test]
    fn reject_empty_methods() {
        let r = expect_err(r#"
            [defaults]
            allowed_methods = []
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("cannot be empty"));
    }

    #[test]
    fn reject_reserved_header_content_length() {
        let r = expect_err(r#"
            [defaults.headers]
            Content-Length = "999"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("reserved"));
    }

    #[test]
    fn reject_reserved_header_transfer_encoding() {
        let r = expect_err(r#"
            [defaults.headers]
            Transfer-Encoding = "chunked"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("reserved"));
    }

    #[test]
    fn reject_zero_max_connections() {
        let r = expect_err(r#"
            [server]
            max_connections = 0
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("max_connections"));
    }

    #[test]
    fn reject_zero_connection_timeout() {
        let r = expect_err(r#"
            [server]
            connection_timeout = 0
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("connection_timeout"));
    }

    #[test]
    fn reject_zero_trusted_proxy_depth() {
        let r = expect_err(r#"
            [server]
            trusted_proxy_depth = 0
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
        "#);
        assert!(r.contains("trusted_proxy_depth"));
    }

    #[test]
    fn valid_wildcard_accepted() {
        let cfg: Config = toml::from_str(r#"
            [[vhost]]
            hostnames = ["*.example.com"]
            root = "/tmp"
        "#).unwrap();
        cfg.resolve().expect("valid wildcard should resolve");
    }

    #[test]
    fn catch_all_accepted() {
        let cfg: Config = toml::from_str(r#"
            [[vhost]]
            hostnames = ["_"]
            root = "/tmp"
        "#).unwrap();
        cfg.resolve().expect("catch-all should resolve");
    }

    #[test]
    fn reject_empty_route_path() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            deny_all = true
            [[vhost.routes]]
            path = ""
            serve = "index.html"
        "#);
        assert!(r.contains("absolute path"), "empty route path: {r}");
    }

    #[test]
    fn reject_route_path_without_slash() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            [[vhost.routes]]
            path = "admin"
            serve = "index.html"
        "#);
        assert!(r.contains("absolute path"), "relative route path: {r}");
    }

    #[test]
    fn reject_empty_redirect_path() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            [[vhost.redirects]]
            path = ""
            url = "https://example.com"
        "#);
        assert!(r.contains("absolute path"), "empty redirect path: {r}");
    }

    #[test]
    fn valid_route_path_accepted() {
        let cfg: Config = toml::from_str(r#"
            [[vhost]]
            hostnames = ["_"]
            root = "/tmp"
            [[vhost.routes]]
            path = "/page"
            serve = "index.html"
        "#).unwrap();
        cfg.resolve().expect("valid route should resolve");
    }

    #[test]
    fn reject_duplicate_redirect_paths() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            [[vhost.redirects]]
            path = "/go"
            url = "/safe"
            [[vhost.redirects]]
            path = "/go"
            url = "https://evil.example/"
        "#);
        assert!(r.contains("duplicate redirect"), "dup redirect: {r}");
    }

    #[test]
    fn reject_not_modified_redirect_code() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            [[vhost.redirects]]
            path = "/x"
            url = "/y"
            code = 304
        "#);
        assert!(r.contains("status code"), "304 redirect: {r}");
    }

    #[test]
    fn reject_empty_route_serve() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            [[vhost.routes]]
            path = "/x"
            serve = ""
        "#);
        assert!(r.contains("cannot be empty"), "empty serve: {r}");
    }

    #[test]
    fn trailing_slash_paths_normalized() {
        let cfg: Config = toml::from_str(r#"
            [[vhost]]
            hostnames = ["_"]
            root = "/tmp"
            spa_ignore = ["/api/", "/"]
            allowed_methods = ["get"]
            [[vhost.routes]]
            path = "/docs/"
            serve = "docs.html"
        "#).unwrap();
        let v = &cfg.resolve().unwrap()[0];
        assert_eq!(v.spa_ignore, ["/api", "/"]);
        assert_eq!(v.routes[0].path, "/docs");
        assert_eq!(v.allowed_methods.as_deref(), Some(&["GET".to_string()][..]));
    }

    #[test]
    fn reject_duplicate_route_after_normalize() {
        let r = expect_err(r#"
            [[vhost]]
            hostnames = ["a.example.com"]
            root = "/tmp"
            [[vhost.routes]]
            path = "/a"
            serve = "a.html"
            [[vhost.routes]]
            path = "/a/"
            serve = "b.html"
        "#);
        assert!(r.contains("duplicate route"), "dup route: {r}");
    }
}

