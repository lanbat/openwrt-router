use super::strip_crlf;

/// Adblock/uBlock format: `||example.com^`, `|http://...`, cosmetic rules skipped.
pub fn parse_adblock(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = strip_crlf(raw).trim();
        if line.is_empty()
            || line.starts_with('!')
            || line.starts_with('[')
            || line.starts_with("@@")
            || line.contains("##")
            || line.contains("#@#")
            || line.contains("#?#")
        {
            continue;
        }

        let mut s: &str = line;
        if let Some(rest) = s.strip_prefix("||") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("|http://") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("|https://") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("http://") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("https://") {
            s = rest;
        } else if s.chars().next().map_or(false, |c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
            // bare domain or domain with trailing anchor — keep as-is
        } else {
            continue;
        }

        // strip everything from the first anchor/option char
        let end = s.find(|c: char| matches!(c, '/' | '^' | '$' | ':' | ',' | '*'))
            .unwrap_or(s.len());
        let domain = s[..end].trim();
        if !domain.is_empty() {
            out.push(domain.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_pipe_domain_anchor() {
        let out = parse_adblock("||example.com^\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn pipe_pipe_with_options() {
        let out = parse_adblock("||ads.example.com^$third-party\n");
        assert_eq!(out, ["ads.example.com"]);
    }

    #[test]
    fn pipe_http_scheme() {
        let out = parse_adblock("|http://tracker.example.org/path\n");
        assert_eq!(out, ["tracker.example.org"]);
    }

    #[test]
    fn pipe_https_scheme() {
        let out = parse_adblock("|https://phish.example.com/\n");
        assert_eq!(out, ["phish.example.com"]);
    }

    #[test]
    fn bare_http_scheme() {
        let out = parse_adblock("http://cdn.example.com/tracking.gif\n");
        assert_eq!(out, ["cdn.example.com"]);
    }

    #[test]
    fn bare_domain_line() {
        let out = parse_adblock("plain-domain.example.com\n");
        assert_eq!(out, ["plain-domain.example.com"]);
    }

    #[test]
    fn skips_comment_lines() {
        let out = parse_adblock("! This is a comment\n||example.com^\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_header_lines() {
        let out = parse_adblock("[Adblock Plus 2.0]\n||example.com^\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_exception_rules() {
        let out = parse_adblock("@@||whitelisted.com^\n");
        assert!(out.is_empty());
    }

    #[test]
    fn skips_cosmetic_filter() {
        let out = parse_adblock("example.com##.ad-banner\n");
        assert!(out.is_empty());
    }

    #[test]
    fn skips_element_hiding_exception() {
        let out = parse_adblock("example.com#@#.ad-banner\n");
        assert!(out.is_empty());
    }

    #[test]
    fn strips_path_after_slash() {
        let out = parse_adblock("||example.com/path/to/ad\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn real_adblock_snippet() {
        let input = "\
[Adblock Plus 2.0]
! Title: Test Filter
||doubleclick.net^
||google-analytics.com^$third-party
@@||trusted.com^
example.com##.promoted-content
||ads.example.org^
";
        let out = parse_adblock(input);
        assert_eq!(out, ["doubleclick.net", "google-analytics.com", "ads.example.org"]);
    }
}
