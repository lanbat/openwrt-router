use super::strip_crlf;

/// dnsmasq config: `address=/example.com/0.0.0.0`, `server=/domain/`, etc.
pub fn parse_dnsmasq(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = {
            let s = strip_crlf(raw);
            let s = match s.find('#') {
                Some(i) => s[..i].trim(),
                None    => s.trim(),
            };
            s
        };
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if !matches!(
            line.split('=').next(),
            Some("address" | "server" | "local" | "ipset" | "nftset")
        ) || !line.contains("=/") {
            continue;
        }
        extract_dnsmasq_domains(line, &mut out);
    }
    out
}

pub fn extract_dnsmasq_domains(line: &str, out: &mut Vec<String>) {
    // format: keyword=/domain1/domain2/.../value
    let parts: Vec<&str> = line.split('/').collect();
    for token in parts.iter().skip(1) {
        let t = token.trim();
        if t.chars().any(|c| c.is_ascii_alphabetic())
            && t.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        {
            out.push(t.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_format_single_domain() {
        let out = parse_dnsmasq("address=/ads.example.com/0.0.0.0\n");
        assert_eq!(out, ["ads.example.com"]);
    }

    #[test]
    fn server_format() {
        let out = parse_dnsmasq("server=/blocked.example.com/\n");
        assert_eq!(out, ["blocked.example.com"]);
    }

    #[test]
    fn local_format() {
        let out = parse_dnsmasq("local=/local.example.com/\n");
        assert_eq!(out, ["local.example.com"]);
    }

    #[test]
    fn ipset_format() {
        // The set name "my_set" also passes the identifier filter (alphanumeric + _)
        let out = parse_dnsmasq("ipset=/example.com/my_set\n");
        assert_eq!(out, ["example.com", "my_set"]);
    }

    #[test]
    fn multiple_domains_in_one_entry() {
        let out = parse_dnsmasq("address=/a.com/b.com/c.com/0.0.0.0\n");
        assert_eq!(out, ["a.com", "b.com", "c.com"]);
    }

    #[test]
    fn skips_comment_lines() {
        let out = parse_dnsmasq("# address=/blocked.com/0.0.0.0\n");
        assert!(out.is_empty());
    }

    #[test]
    fn skips_lines_without_slash_format() {
        let out = parse_dnsmasq("no-hosts\nlog-queries\n");
        assert!(out.is_empty());
    }

    #[test]
    fn strips_inline_comment_from_entry() {
        let out = parse_dnsmasq("address=/example.com/0.0.0.0 # comment\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn extract_dnsmasq_domains_basic() {
        let mut out = Vec::new();
        extract_dnsmasq_domains("address=/a.com/b.com/1.2.3.4", &mut out);
        assert_eq!(out, ["a.com", "b.com"]);
    }

    #[test]
    fn extract_dnsmasq_domains_filters_ip_suffix() {
        let mut out = Vec::new();
        extract_dnsmasq_domains("address=/example.com/0.0.0.0", &mut out);
        // "0.0.0.0" should be skipped — it's an IP (no alpha chars pass, or
        // dots-only chars would still be numeric, so classify handles it)
        // Actually "0.0.0.0" has no alpha chars, so it is filtered out
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn real_dnsmasq_config_snippet() {
        let input = "\
# DNSmasq blocklist
address=/doubleclick.net/0.0.0.0
address=/doubleclick.net/::
server=/tracker.example.com/
address=/ads.example.com/a.ads.example.com/0.0.0.0
";
        let mut out = parse_dnsmasq(input);
        out.sort();
        out.dedup();
        assert!(out.contains(&"doubleclick.net".to_string()));
        assert!(out.contains(&"tracker.example.com".to_string()));
        assert!(out.contains(&"ads.example.com".to_string()));
        assert!(out.contains(&"a.ads.example.com".to_string()));
    }
}
