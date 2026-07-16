use super::strip_crlf;

/// URL/tracker format: `udp://tracker.example.org:1337/announce`
/// Extracts the host portion.
pub fn parse_url(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = {
            let s = strip_crlf(raw);
            // strip inline comment
            let s = match s.find(|c: char| c == '\t' || c == ' ') {
                Some(i) if s[i..].contains('#') => s[..s[i..].find('#').map(|j| i+j).unwrap_or(s.len())].trim(),
                _ => s.trim(),
            };
            s
        };
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let first = line.split_whitespace().next().unwrap_or(line);
        if let Some(host) = extract_host(first) {
            if !host.is_empty() {
                out.push(host);
            }
        }
    }
    out
}

pub fn extract_host(s: &str) -> Option<String> {
    // strip scheme
    let s = if let Some(i) = s.find("://") {
        &s[i + 3..]
    } else {
        s
    };
    // strip path and query
    let s = s.split('/').next().unwrap_or(s);
    let s = s.split('?').next().unwrap_or(s);

    // IPv6 literal: [::1]:port
    if s.starts_with('[') {
        let end = s.find(']')?;
        return Some(s[1..end].to_string());
    }

    // strip userinfo
    let s = if let Some(i) = s.find('@') { &s[i+1..] } else { s };
    // strip port
    let s = if let Some(i) = s.rfind(':') {
        if s[i+1..].chars().all(|c| c.is_ascii_digit()) { &s[..i] } else { s }
    } else { s };

    Some(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_host ─────────────────────────────────────────────────────────

    #[test]
    fn extract_http_plain() {
        assert_eq!(extract_host("http://example.com/path"), Some("example.com".to_string()));
    }

    #[test]
    fn extract_https_with_port_and_path() {
        assert_eq!(extract_host("https://example.com:443/path?q=1"), Some("example.com".to_string()));
    }

    #[test]
    fn extract_udp_with_port() {
        assert_eq!(extract_host("udp://tracker.example.org:1337/announce"), Some("tracker.example.org".to_string()));
    }

    #[test]
    fn extract_strips_userinfo() {
        assert_eq!(extract_host("http://user:pass@example.com/"), Some("example.com".to_string()));
    }

    #[test]
    fn extract_ipv6_bracket_literal() {
        assert_eq!(extract_host("http://[2001:db8::1]:8080/"), Some("2001:db8::1".to_string()));
    }

    #[test]
    fn extract_bare_hostname_no_scheme() {
        // without scheme, rfind(':') strips port
        assert_eq!(extract_host("tracker.example.com:1337"), Some("tracker.example.com".to_string()));
    }

    #[test]
    fn extract_bare_hostname_no_port() {
        assert_eq!(extract_host("tracker.example.com"), Some("tracker.example.com".to_string()));
    }

    // ── parse_url ────────────────────────────────────────────────────────────

    #[test]
    fn parse_url_single_line() {
        let out = parse_url("udp://tracker.example.org:6881/announce\n");
        assert_eq!(out, ["tracker.example.org"]);
    }

    #[test]
    fn parse_url_skips_comment() {
        let out = parse_url("# commented out\nhttp://tracker.example.com/\n");
        assert_eq!(out, ["tracker.example.com"]);
    }

    #[test]
    fn parse_url_skips_semicolon_comment() {
        let out = parse_url("; tracker list\nhttp://a.example.com/\n");
        assert_eq!(out, ["a.example.com"]);
    }

    #[test]
    fn parse_url_multiple() {
        let input = "\
udp://tracker.opentrackr.org:1337/announce
udp://tracker.torrent.eu.org:451/announce
http://tracker.trackerfix.com/announce
";
        let out = parse_url(input);
        assert_eq!(out, [
            "tracker.opentrackr.org",
            "tracker.torrent.eu.org",
            "tracker.trackerfix.com",
        ]);
    }
}
