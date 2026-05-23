use std::path::{Path, PathBuf};

pub fn decode(uri_path: &str) -> Option<String> {
    let mut out = Vec::with_capacity(uri_path.len());
    let mut bytes = uri_path.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = bytes.next().and_then(hex_val);
            let lo = bytes.next().and_then(hex_val);
            match (hi, lo) {
                (Some(h), Some(l)) => {
                    let decoded = h << 4 | l;
                    if decoded <= 0x1f
                        || decoded == 0x7f
                        || decoded == 0x2f
                        || decoded == 0x5c
                    {
                        return None;
                    }
                    out.push(decoded);
                }
                _ => return None,
            }
        } else {
            out.push(b);
        }
    }
    String::from_utf8(out).ok()
}

pub fn has_dot_segment(decoded_path: &str) -> bool {
    decoded_path
        .split('/')
        .any(|s| s.starts_with('.') && s.len() > 1)
}

pub fn resolve(root: &Path, decoded_path: &str) -> Option<PathBuf> {
    let mut result = root.to_path_buf();
    for seg in decoded_path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." || seg.contains('\0') || seg.contains('\\') {
            return None;
        }
        result.push(seg);
    }
    Some(result)
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn decode_normal_path() {
        assert_eq!(decode("/foo/bar").unwrap(), "/foo/bar");
    }

    #[test]
    fn decode_percent_encoded() {
        assert_eq!(decode("/foo%20bar").unwrap(), "/foo bar");
    }

    #[test]
    fn decode_rejects_overlong_c0() {
        assert!(decode("/%c0%af").is_none());
    }

    #[test]
    fn decode_rejects_overlong_c1() {
        assert!(decode("/%c1%9c").is_none());
    }

    #[test]
    fn decode_rejects_fe_ff() {
        assert!(decode("/%fe").is_none());
        assert!(decode("/%ff").is_none());
    }

    #[test]
    fn decode_rejects_malformed_percent() {
        assert!(decode("/%zz").is_none());
        assert!(decode("/%0").is_none());
    }

    #[test]
    fn decode_rejects_encoded_controls() {
        assert!(decode("/%00").is_none());
        assert!(decode("/%09").is_none());
        assert!(decode("/%0a").is_none());
        assert!(decode("/%0d").is_none());
        assert!(decode("/%1f").is_none());
        assert!(decode("/%7f").is_none());
    }

    #[test]
    fn decode_rejects_encoded_slash() {
        assert!(decode("/a%2fb").is_none());
        assert!(decode("/a%2Fb").is_none());
    }

    #[test]
    fn decode_rejects_encoded_backslash() {
        assert!(decode("/a%5cb").is_none());
        assert!(decode("/a%5Cb").is_none());
    }

    #[test]
    fn dot_segment_detects_dotenv() {
        assert!(has_dot_segment("/.env"));
        assert!(has_dot_segment("/foo/.env"));
        assert!(has_dot_segment("/.git/config"));
    }

    #[test]
    fn dot_segment_allows_normal_paths() {
        assert!(!has_dot_segment("/foo/bar"));
        assert!(!has_dot_segment("/assets/style.css"));
        assert!(!has_dot_segment("/"));
    }

    #[test]
    fn dot_segment_ignores_single_dot() {
        assert!(!has_dot_segment("/."));
        assert!(!has_dot_segment("/./foo"));
    }

    #[test]
    fn resolve_blocks_traversal() {
        let root = Path::new("/static");
        assert!(resolve(root, "/../etc/passwd").is_none());
        assert!(resolve(root, "/foo/../../etc/passwd").is_none());
    }

    #[test]
    fn resolve_blocks_null_bytes() {
        let root = Path::new("/static");
        assert!(resolve(root, "/foo\0bar").is_none());
    }

    #[test]
    fn resolve_blocks_backslash() {
        let root = Path::new("/static");
        assert!(resolve(root, "/foo\\bar").is_none());
    }

    #[test]
    fn resolve_normal_path() {
        let root = Path::new("/static");
        assert_eq!(
            resolve(root, "/foo/bar.html").unwrap(),
            Path::new("/static/foo/bar.html")
        );
    }

    #[test]
    fn resolve_skips_empty_and_dot() {
        let root = Path::new("/static");
        assert_eq!(
            resolve(root, "//foo/./bar//").unwrap(),
            Path::new("/static/foo/bar")
        );
    }

    #[test]
    fn encoded_dot_detected_after_decode() {
        let decoded = decode("/%2Eenv").unwrap();
        assert!(has_dot_segment(&decoded));
    }

    #[test]
    fn encoded_dotdot_detected_after_decode() {
        let decoded = decode("/%2E%2E/etc/passwd").unwrap();
        assert_eq!(decoded, "/../etc/passwd");
        assert!(resolve(Path::new("/static"), &decoded).is_none());
    }
}
