use super::{strip_crlf, trim_comment};

/// Clash/Surge rules: `DOMAIN,example.com` or `IP-CIDR,1.2.3.0/24`
pub fn parse_clash(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = trim_comment(strip_crlf(raw)).trim();
        if line.is_empty() {
            continue;
        }
        let line = line.strip_prefix("- ").unwrap_or(line);
        let mut parts = line.splitn(3, ',');
        let Some(kind) = parts.next() else { continue };
        let Some(val) = parts.next() else { continue };
        let val = val.trim();
        match kind.trim().to_uppercase().as_str() {
            "DOMAIN" | "DOMAIN-SUFFIX" | "DOMAIN-KEYWORD"
            | "IP-CIDR" | "IP-CIDR6" => {
                if !val.is_empty() {
                    out.push(val.to_string());
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_rule() {
        let out = parse_clash("DOMAIN,example.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn domain_suffix_rule() {
        let out = parse_clash("DOMAIN-SUFFIX,example.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn domain_keyword_rule() {
        let out = parse_clash("DOMAIN-KEYWORD,tracker\n");
        assert_eq!(out, ["tracker"]);
    }

    #[test]
    fn ip_cidr_rule() {
        let out = parse_clash("IP-CIDR,1.2.3.0/24\n");
        assert_eq!(out, ["1.2.3.0/24"]);
    }

    #[test]
    fn ip_cidr6_rule() {
        let out = parse_clash("IP-CIDR6,2001:db8::/32\n");
        assert_eq!(out, ["2001:db8::/32"]);
    }

    #[test]
    fn yaml_list_prefix_stripped() {
        let out = parse_clash("- DOMAIN,example.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_geoip_rule() {
        let out = parse_clash("GEOIP,CN\n");
        assert!(out.is_empty());
    }

    #[test]
    fn skips_match_rule() {
        let out = parse_clash("MATCH,DIRECT\n");
        assert!(out.is_empty());
    }

    #[test]
    fn strips_inline_comment() {
        let out = parse_clash("DOMAIN,example.com # blocked\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn third_csv_field_ignored() {
        // clash rules can have DOMAIN,value,policy — only value is returned
        let out = parse_clash("DOMAIN,example.com,REJECT\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn case_insensitive_kind() {
        let out = parse_clash("domain,example.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn real_clash_rules_snippet() {
        let input = "\
# Clash rules
- DOMAIN-SUFFIX,doubleclick.net
- DOMAIN,tracking.example.com
- IP-CIDR,1.2.3.0/24
- GEOIP,CN
- MATCH,DIRECT
";
        let out = parse_clash(input);
        assert_eq!(out, ["doubleclick.net", "tracking.example.com", "1.2.3.0/24"]);
    }
}
