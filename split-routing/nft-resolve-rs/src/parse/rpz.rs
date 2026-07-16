use super::strip_crlf;

/// BIND RPZ zone: `example.com CNAME .`
/// Only A, AAAA, CNAME record types; skip SOA, NS, etc.
pub fn parse_rpz(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = {
            let s = strip_crlf(raw);
            match s.find(';') {
                Some(i) => s[..i].trim(),
                None    => s.trim(),
            }
        };
        if line.is_empty()
            || line.starts_with('$')
            || line.starts_with('@')
            || line.starts_with('(')
            || line.starts_with(')')
        {
            continue;
        }

        let mut fields = line.split_whitespace();
        let Some(domain) = fields.next() else { continue };

        // find the record type, skipping IN and TTL numbers
        let rtype = fields
            .find(|f| {
                let u = f.to_uppercase();
                u != "IN" && !f.chars().all(|c| c.is_ascii_digit())
            })
            .map(|f| f.to_uppercase());

        let Some(rtype) = rtype else { continue };

        match rtype.as_str() {
            "SOA" | "NS" | "MX" | "TXT" | "PTR" | "SRV"
            | "DNSKEY" | "RRSIG" | "NSEC" | "CAA" => continue,
            "A" | "AAAA" | "CNAME" => {}
            _ => continue,
        }

        // strip wildcard prefix and trailing dot
        let d = domain
            .trim_end_matches('.')
            .trim_start_matches("*.")
            .trim_start_matches('.');

        if d.is_empty() || !d.contains('.') || !d.chars().any(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        out.push(d.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cname_record() {
        let out = parse_rpz("example.com CNAME .\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn a_record() {
        let out = parse_rpz("blocked.example.com A 127.0.0.1\n");
        assert_eq!(out, ["blocked.example.com"]);
    }

    #[test]
    fn aaaa_record() {
        let out = parse_rpz("blocked.example.com AAAA ::1\n");
        assert_eq!(out, ["blocked.example.com"]);
    }

    #[test]
    fn with_ttl_and_in() {
        let out = parse_rpz("example.com 3600 IN CNAME .\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_soa() {
        let out = parse_rpz("@ SOA ns1. admin. 1 3600 900 86400 3600\n");
        assert!(out.is_empty());
    }

    #[test]
    fn skips_ns() {
        let out = parse_rpz("example.com NS ns1.example.com.\n");
        assert!(out.is_empty());
    }

    #[test]
    fn skips_mx() {
        let out = parse_rpz("example.com MX 10 mail.example.com.\n");
        assert!(out.is_empty());
    }

    #[test]
    fn skips_dollar_lines() {
        let out = parse_rpz("$ORIGIN rpz.example.com.\n$TTL 1h\nexample.com CNAME .\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_at_lines() {
        let out = parse_rpz("@ IN SOA ns1. admin. 1 3600 900 86400 3600\nexample.com CNAME .\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn strips_wildcard_prefix() {
        let out = parse_rpz("*.wildcard.example.com CNAME .\n");
        assert_eq!(out, ["wildcard.example.com"]);
    }

    #[test]
    fn strips_trailing_dot_from_domain() {
        let out = parse_rpz("trailing.example.com. CNAME .\n");
        assert_eq!(out, ["trailing.example.com"]);
    }

    #[test]
    fn skips_inline_comment() {
        let out = parse_rpz("example.com CNAME . ; this is blocked\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn real_rpz_zone_snippet() {
        let input = "\
$ORIGIN rpz.example.com.
$TTL 1h
@ SOA ns1.example.com. admin.example.com. 2024010101 3600 900 86400 3600
@ NS ns1.example.com.
doubleclick.net CNAME .
*.doubleclick.net CNAME .
google-analytics.com A 127.0.0.1
";
        let out = parse_rpz(input);
        assert_eq!(out, ["doubleclick.net", "doubleclick.net", "google-analytics.com"]);
    }
}
