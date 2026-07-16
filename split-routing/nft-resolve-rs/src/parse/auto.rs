use super::{strip_crlf, trim_comment};
use super::dnsmasq::extract_dnsmasq_domains;
use super::url::extract_host;
use crate::classify::looks_like_ip;

/// Auto-detect format per line, combining all known formats.
pub fn parse_auto(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let raw = strip_crlf(raw);
        let trimmed = raw.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('!')
            || trimmed.starts_with('#')
            || trimmed.starts_with(';')
            || trimmed.starts_with('[')
            || trimmed.starts_with("@@")
        {
            continue;
        }

        // strip inline comment from a copy for field parsing
        let line = match trimmed.find(|c: char| c == '\t' || c == ' ') {
            Some(i) => match trimmed[i..].find('#') {
                Some(j) => trimmed[..i+j].trim(),
                None    => trimmed,
            },
            None => trimmed,
        };

        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.is_empty() { continue; }

        // hosts file: first field is IP, rest are domains
        if looks_like_ip(fields[0]) {
            if fields.len() > 1 {
                out.extend(fields[1..].iter().map(|s| s.to_string()));
            } else {
                out.push(fields[0].to_string());
            }
            continue;
        }

        // dnsmasq: address=/domain/ip
        if matches!(fields[0].split('=').next(), Some("address"|"server"|"local"|"ipset"|"nftset"))
            && line.contains("=/")
        {
            extract_dnsmasq_domains(line, &mut out);
            continue;
        }

        // unbound local-zone
        if let Some(rest) = line.strip_prefix("local-zone:") {
            let first = rest.trim().split_whitespace().next().unwrap_or("");
            let d = first.trim_matches('"').trim_end_matches('.');
            if d.contains('.') && d.chars().any(|c| c.is_ascii_alphabetic()) {
                out.push(d.to_string());
            }
            continue;
        }

        // unbound local-data
        if let Some(rest) = line.strip_prefix("local-data:") {
            let rest = rest.trim().trim_start_matches('"');
            if let Some(d) = rest.split_whitespace().next() {
                let d = d.trim_matches('"').trim_end_matches('.');
                if d.contains('.') && d.chars().any(|c| c.is_ascii_alphabetic()) {
                    out.push(d.to_string());
                }
            }
            continue;
        }

        // ipset: add setname ip
        if fields[0] == "add" && fields.len() >= 3 {
            out.push(fields[2].to_string());
            continue;
        }

        // Clash: DOMAIN,val or IP-CIDR,val
        if let Some(comma) = line.find(',') {
            let kind = line[..comma].trim().to_uppercase();
            match kind.as_str() {
                "DOMAIN" | "DOMAIN-SUFFIX" | "DOMAIN-KEYWORD" | "IP-CIDR" | "IP-CIDR6" => {
                    let val = line[comma+1..].split(',').next().unwrap_or("").trim();
                    if !val.is_empty() {
                        out.push(val.to_string());
                    }
                    continue;
                }
                _ => {}
            }
        }

        // adblock ||domain^
        if line.starts_with("||") {
            let s = &line[2..];
            let end = s.find(|c: char| matches!(c, '/' | '^' | '$' | ':' | ',' | '*'))
                .unwrap_or(s.len());
            let d = s[..end].trim();
            if !d.is_empty() { out.push(d.to_string()); }
            continue;
        }

        // URL with scheme
        if line.contains("://") || line.starts_with("|http://") || line.starts_with("|https://") {
            let s = if line.starts_with('|') { &line[1..] } else { line };
            if let Some(h) = extract_host(s) {
                if !h.is_empty() { out.push(h); }
            }
            continue;
        }

        // bare domain or IP — strip comment and take first field
        let line = trim_comment(trimmed).trim();
        let first = line.split_whitespace().next().unwrap_or(line);
        if !first.is_empty() {
            out.push(first.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_hosts_format_line() {
        let out = parse_auto("0.0.0.0 ads.example.com\n");
        assert_eq!(out, ["ads.example.com"]);
    }

    #[test]
    fn auto_hosts_multiple_domains() {
        let out = parse_auto("0.0.0.0 a.com b.com\n");
        assert_eq!(out, ["a.com", "b.com"]);
    }

    #[test]
    fn auto_lone_ip_kept() {
        let out = parse_auto("1.2.3.4\n");
        assert_eq!(out, ["1.2.3.4"]);
    }

    #[test]
    fn auto_dnsmasq_address() {
        let out = parse_auto("address=/example.com/0.0.0.0\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn auto_unbound_local_zone() {
        let out = parse_auto("local-zone: \"blocked.example.com\" always_nxdomain\n");
        assert_eq!(out, ["blocked.example.com"]);
    }

    #[test]
    fn auto_unbound_local_data() {
        let out = parse_auto("local-data: \"blocked.example.com A 0.0.0.0\"\n");
        assert_eq!(out, ["blocked.example.com"]);
    }

    #[test]
    fn auto_adblock_pipe_pipe() {
        let out = parse_auto("||doubleclick.net^\n");
        assert_eq!(out, ["doubleclick.net"]);
    }

    #[test]
    fn auto_url_with_scheme() {
        let out = parse_auto("http://tracker.example.com/announce\n");
        assert_eq!(out, ["tracker.example.com"]);
    }

    #[test]
    fn auto_clash_domain_rule() {
        let out = parse_auto("DOMAIN,clash.example.com\n");
        assert_eq!(out, ["clash.example.com"]);
    }

    #[test]
    fn auto_ipset_add() {
        let out = parse_auto("add myset 5.5.5.5\n");
        assert_eq!(out, ["5.5.5.5"]);
    }

    #[test]
    fn auto_bare_domain() {
        let out = parse_auto("bare.example.com\n");
        assert_eq!(out, ["bare.example.com"]);
    }

    #[test]
    fn auto_skips_hash_comment() {
        assert!(parse_auto("# comment\n").is_empty());
    }

    #[test]
    fn auto_skips_exclamation() {
        assert!(parse_auto("! adblock comment\n").is_empty());
    }

    #[test]
    fn auto_skips_semicolon() {
        assert!(parse_auto("; semicolon comment\n").is_empty());
    }

    #[test]
    fn auto_mixed_real_world_file() {
        let input = "\
# Mixed blocklist
0.0.0.0 doubleclick.net
||google-analytics.com^
address=/ads.example.com/0.0.0.0
udp://tracker.example.org:1337/announce
DOMAIN,clash-blocked.example.com
bare-domain.example.com
";
        let out = parse_auto(input);
        assert!(out.contains(&"doubleclick.net".to_string()), "hosts format missed");
        assert!(out.contains(&"google-analytics.com".to_string()), "adblock format missed");
        assert!(out.contains(&"ads.example.com".to_string()), "dnsmasq format missed");
        assert!(out.contains(&"tracker.example.org".to_string()), "url format missed");
        assert!(out.contains(&"clash-blocked.example.com".to_string()), "clash format missed");
        assert!(out.contains(&"bare-domain.example.com".to_string()), "bare domain missed");
    }
}
