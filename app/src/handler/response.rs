use hyper::header::{self, HeaderValue};
use hyper::{Response, StatusCode};

use super::body::ResponseBody;
use crate::config::ResolvedVhost;

pub fn error(status: StatusCode, msg: &str) -> Response<ResponseBody> {
    let body = format!("{} {}", status.as_u16(), msg);
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(ResponseBody::text(body))
        .unwrap()
}

pub fn method_not_allowed(allowed: &[String]) -> Response<ResponseBody> {
    let allow = allowed.join(", ");
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .header(header::ALLOW, allow)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(ResponseBody::text("405 method not allowed".into()))
        .unwrap()
}

pub fn redirect(status: StatusCode, url: &str) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .header(header::LOCATION, url)
        .header(header::CONTENT_LENGTH, "0")
        .body(ResponseBody::empty())
        .unwrap()
}

pub fn add_headers(
    mut resp: Response<ResponseBody>,
    vhost: &ResolvedVhost,
) -> Response<ResponseBody> {
    let hdrs = resp.headers_mut();
    for (name, value) in &vhost.headers {
        hdrs.insert(name.clone(), value.clone());
    }
    resp
}

pub fn with_baseline(mut resp: Response<ResponseBody>) -> Response<ResponseBody> {
    let hdrs = resp.headers_mut();
    hdrs.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    hdrs.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    hdrs.insert(
        header::HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'none'"),
    );
    hdrs.insert(
        header::HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    resp
}
