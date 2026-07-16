use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

#[derive(Default)]
pub struct Classified {
    pub domains: Vec<String>,
    pub ip4:     Vec<String>,
    pub ip6:     Vec<String>,
}

const SKIP: &[&str] = &[
    "localhost", "localdomain", "broadcasthost",
    "0.0.0.0", "127.0.0.1", "::", "::1",
];

pub fn classify(candidates: &[String]) -> Classified {
    let mut c = Classified::default();
    for raw in candidates {
        let s = raw.trim().to_lowercase();
        let s = s.trim_end_matches('.');

        // unwrap IPv6 bracket notation
        let s = if s.starts_with('[') {
            s.trim_start_matches('[').split(']').next().unwrap_or(s)
        } else {
            s
        };

        // strip wildcard/leading dot
        let s = s.trim_start_matches("*.")
                  .trim_start_matches('.');

        // strip trailing port `host:1234`
        let s = strip_port(s);

        if s.is_empty() || SKIP.contains(&s) {
            continue;
        }

        if let Some(cat) = classify_one(s) {
            match cat {
                Cat::V4(v) => c.ip4.push(v),
                Cat::V6(v) => c.ip6.push(v),
                Cat::Domain(v) => c.domains.push(v),
            }
        }
    }
    c
}

enum Cat { V4(String), V6(String), Domain(String) }

fn classify_one(s: &str) -> Option<Cat> {
    // IPv4 or IPv4 CIDR
    if let Ok(_) = Ipv4Addr::from_str(s) {
        return Some(Cat::V4(s.to_string()));
    }
    if is_v4_cidr(s) {
        return Some(Cat::V4(s.to_string()));
    }

    // IPv6 or IPv6 CIDR
    if let Ok(_) = Ipv6Addr::from_str(s) {
        return Some(Cat::V6(s.to_string()));
    }
    if is_v6_cidr(s) {
        return Some(Cat::V6(s.to_string()));
    }

    // domain: only [a-z0-9_.-], must have a dot, must have a letter, no double-dot
    if s.contains('.')
        && !s.contains("..")
        && s.chars().any(|c| c.is_ascii_alphabetic())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Some(Cat::Domain(s.to_string()));
    }

    None
}

fn is_v4_cidr(s: &str) -> bool {
    let Some((addr, prefix)) = s.split_once('/') else { return false };
    let Ok(p) = prefix.parse::<u8>() else { return false };
    p <= 32 && Ipv4Addr::from_str(addr).is_ok()
}

fn is_v6_cidr(s: &str) -> bool {
    let Some((addr, prefix)) = s.split_once('/') else { return false };
    let Ok(p) = prefix.parse::<u8>() else { return false };
    p <= 128 && Ipv6Addr::from_str(addr).is_ok()
}

fn strip_port(s: &str) -> &str {
    // only strip trailing `:digits` for non-IPv6
    if s.contains(':') && !s.contains("::") {
        if let Some(i) = s.rfind(':') {
            if s[i+1..].chars().all(|c| c.is_ascii_digit()) {
                return &s[..i];
            }
        }
    }
    s
}

/// Quick check: does this string look like an IPv4 address (used by parsers)?
pub fn looks_like_ip(s: &str) -> bool {
    Ipv4Addr::from_str(s).is_ok()
        || Ipv6Addr::from_str(s).is_ok()
        || is_v4_cidr(s)
        || is_v6_cidr(s)
        // heuristic: starts with digit and has dots (catches 0.0.0.0 etc.)
        || (s.starts_with(|c: char| c.is_ascii_digit()) && s.contains('.'))
        || s.starts_with('[')  // IPv6 bracket
}

pub fn dedup_sorted(v: &mut Vec<String>) {
    v.sort_unstable();
    v.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // ── looks_like_ip ────────────────────────────────────────────────────────

    #[test]
    fn ip_plain_ipv4() { assert!(looks_like_ip("1.2.3.4")); }
    #[test]
    fn ip_ipv4_cidr() { assert!(looks_like_ip("10.0.0.0/8")); }
    #[test]
    fn ip_plain_ipv6() { assert!(looks_like_ip("2001:db8::1")); }
    #[test]
    fn ip_ipv6_cidr() { assert!(looks_like_ip("2001:db8::/32")); }
    #[test]
    fn ip_bracket_ipv6() { assert!(looks_like_ip("[::1]")); }
    #[test]
    fn ip_all_zeros() { assert!(looks_like_ip("0.0.0.0")); }
    #[test]
    fn ip_rejects_domain() { assert!(!looks_like_ip("example.com")); }
    #[test]
    fn ip_rejects_localhost() { assert!(!looks_like_ip("localhost")); }

    // ── classify ─────────────────────────────────────────────────────────────

    #[test]
    fn classify_ipv4() {
        let c = classify(&strings(&["1.2.3.4"]));
        assert_eq!(c.ip4, ["1.2.3.4"]);
        assert!(c.domains.is_empty() && c.ip6.is_empty());
    }

    #[test]
    fn classify_ipv4_cidr() {
        let c = classify(&strings(&["192.168.1.0/24"]));
        assert_eq!(c.ip4, ["192.168.1.0/24"]);
    }

    #[test]
    fn classify_ipv6() {
        let c = classify(&strings(&["2001:db8::1"]));
        assert_eq!(c.ip6, ["2001:db8::1"]);
        assert!(c.domains.is_empty() && c.ip4.is_empty());
    }

    #[test]
    fn classify_ipv6_cidr() {
        let c = classify(&strings(&["2001:db8::/32"]));
        assert_eq!(c.ip6, ["2001:db8::/32"]);
    }

    #[test]
    fn classify_domain() {
        let c = classify(&strings(&["example.com"]));
        assert_eq!(c.domains, ["example.com"]);
        assert!(c.ip4.is_empty() && c.ip6.is_empty());
    }

    #[test]
    fn classify_skips_reserved_entries() {
        let input = strings(&["localhost", "localdomain", "0.0.0.0", "127.0.0.1", "::", "::1"]);
        let c = classify(&input);
        assert!(c.domains.is_empty() && c.ip4.is_empty() && c.ip6.is_empty());
    }

    #[test]
    fn classify_strips_trailing_dot() {
        let c = classify(&strings(&["example.com."]));
        assert_eq!(c.domains, ["example.com"]);
    }

    #[test]
    fn classify_strips_wildcard_prefix() {
        let c = classify(&strings(&["*.example.com"]));
        assert_eq!(c.domains, ["example.com"]);
    }

    #[test]
    fn classify_strips_leading_dot() {
        let c = classify(&strings(&[".example.com"]));
        assert_eq!(c.domains, ["example.com"]);
    }

    #[test]
    fn classify_strips_port_from_domain() {
        let c = classify(&strings(&["tracker.example.com:1337"]));
        assert_eq!(c.domains, ["tracker.example.com"]);
    }

    #[test]
    fn classify_unwraps_bracket_ipv6() {
        let c = classify(&strings(&["[2001:db8::1]"]));
        assert_eq!(c.ip6, ["2001:db8::1"]);
    }

    #[test]
    fn classify_skips_no_dot_string() {
        let c = classify(&strings(&["nodothere"]));
        assert!(c.domains.is_empty() && c.ip4.is_empty() && c.ip6.is_empty());
    }

    #[test]
    fn classify_rejects_double_dot() {
        let c = classify(&strings(&["exam..ple.com"]));
        assert!(c.domains.is_empty());
    }

    #[test]
    fn classify_rejects_non_ascii_domain() {
        let c = classify(&strings(&["ex@mple.com"]));
        assert!(c.domains.is_empty());
    }

    // ── dedup_sorted ─────────────────────────────────────────────────────────

    #[test]
    fn dedup_sorts_and_removes_duplicates() {
        let mut v = strings(&["c.com", "a.com", "b.com", "a.com"]);
        dedup_sorted(&mut v);
        assert_eq!(v, strings(&["a.com", "b.com", "c.com"]));
    }

    #[test]
    fn dedup_empty_is_noop() {
        let mut v: Vec<String> = vec![];
        dedup_sorted(&mut v);
        assert!(v.is_empty());
    }
}
