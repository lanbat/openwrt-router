use super::{strip_crlf, trim_comment};
use crate::classify::looks_like_ip;

/// Hosts-file format: `0.0.0.0 example.com another.com`
/// The first field is an IP address; subsequent fields are domains.
pub fn parse_hosts(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = trim_comment(strip_crlf(raw)).trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let Some(first) = fields.next() else { continue };
        if looks_like_ip(first) {
            out.extend(fields.map(|s| s.to_string()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_0000_format() {
        let out = parse_hosts("0.0.0.0 ads.example.com\n");
        assert_eq!(out, ["ads.example.com"]);
    }

    #[test]
    fn loopback_format() {
        let out = parse_hosts("127.0.0.1 tracking.example.com\n");
        assert_eq!(out, ["tracking.example.com"]);
    }

    #[test]
    fn multiple_domains_per_line() {
        let out = parse_hosts("0.0.0.0 a.example.com b.example.com c.example.com\n");
        assert_eq!(out, ["a.example.com", "b.example.com", "c.example.com"]);
    }

    #[test]
    fn strips_inline_comment() {
        let out = parse_hosts("0.0.0.0 ads.example.com # this is blocked\n");
        assert_eq!(out, ["ads.example.com"]);
    }

    #[test]
    fn skips_comment_only_lines() {
        let out = parse_hosts("# this is a hosts file\n0.0.0.0 ads.com\n");
        assert_eq!(out, ["ads.com"]);
    }

    #[test]
    fn skips_empty_lines() {
        let out = parse_hosts("\n\n0.0.0.0 ads.com\n\n");
        assert_eq!(out, ["ads.com"]);
    }

    #[test]
    fn skips_lines_without_ip_first() {
        // lines where the first field is not an IP are ignored
        let out = parse_hosts("example.com 1.2.3.4\n");
        assert!(out.is_empty());
    }

    #[test]
    fn ipv6_host_lines_work() {
        let out = parse_hosts("::1 ipv6-only.example.com\n");
        assert_eq!(out, ["ipv6-only.example.com"]);
    }

    #[test]
    fn real_hosts_snippet() {
        let input = "\
# Hosts file for blocking
127.0.0.1 localhost
::1 localhost
0.0.0.0 doubleclick.net
0.0.0.0 googleads.g.doubleclick.net
0.0.0.0 www.google-analytics.com
";
        let out = parse_hosts(input);
        assert_eq!(out, ["localhost", "localhost", "doubleclick.net",
                          "googleads.g.doubleclick.net", "www.google-analytics.com"]);
    }
}
