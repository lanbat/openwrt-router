use super::{strip_crlf, trim_comment};

/// Unbound config: `local-zone: "example.com" always_nxdomain`
/// and `local-data: "example.com A 0.0.0.0"`
pub fn parse_unbound(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = trim_comment(strip_crlf(raw)).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("local-zone:") {
            // `local-zone: "example.com" always_nxdomain` — first field, strip all quotes
            let first = rest.trim().split_whitespace().next().unwrap_or("");
            let d = first.replace('"', "");
            let d = d.trim_end_matches('.');
            if d.contains('.') && d.chars().any(|c| c.is_ascii_alphabetic()) {
                out.push(d.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("local-data:") {
            // `local-data: "example.com A 127.0.0.1"` — first field inside quotes
            let rest = rest.trim().trim_start_matches('"');
            let first = rest.split_whitespace().next().unwrap_or("");
            let d = first.trim_end_matches('.');
            if d.contains('.') && d.chars().any(|c| c.is_ascii_alphabetic()) {
                out.push(d.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_zone_basic() {
        let out = parse_unbound("local-zone: \"example.com\" always_nxdomain\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn local_zone_strips_trailing_dot() {
        let out = parse_unbound("local-zone: \"example.com.\" always_nxdomain\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn local_data_a_record() {
        let out = parse_unbound("local-data: \"example.com A 0.0.0.0\"\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn local_data_strips_trailing_dot() {
        let out = parse_unbound("local-data: \"example.com. A 127.0.0.1\"\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_comment_lines() {
        let out = parse_unbound("# local-zone: \"commented.com\" always_nxdomain\n");
        assert!(out.is_empty());
    }

    #[test]
    fn skips_unrelated_lines() {
        let out = parse_unbound("server:\n  verbosity: 1\n  interface: 0.0.0.0\n");
        assert!(out.is_empty());
    }

    #[test]
    fn real_unbound_snippet() {
        let input = "\
# Unbound blocklist
local-zone: \"doubleclick.net\" always_nxdomain
local-zone: \"google-analytics.com.\" always_nxdomain
local-data: \"ads.example.com A 0.0.0.0\"
local-data: \"tracking.example.com. AAAA ::\"
";
        let out = parse_unbound(input);
        assert_eq!(out, [
            "doubleclick.net",
            "google-analytics.com",
            "ads.example.com",
            "tracking.example.com",
        ]);
    }
}