mod auto;
mod hosts;
mod adblock;
mod url;
mod dnsmasq;
mod rpz;
mod unbound;
mod ipset;
mod clash;
mod oneper;

pub use auto::parse_auto;
pub use hosts::parse_hosts;
pub use adblock::parse_adblock;
pub use url::parse_url;
pub use dnsmasq::parse_dnsmasq;
pub use rpz::parse_rpz;
pub use unbound::parse_unbound;
pub use ipset::parse_ipset;
pub use clash::parse_clash;
pub use oneper::parse_oneper;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Auto,
    Domain,
    Hosts,
    Adblock,
    Url,
    Dnsmasq,
    Rpz,
    Unbound,
    Ipset,
    Clash,
    Ip,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto"                              => Some(Self::Auto),
            "domain" | "domains"               => Some(Self::Domain),
            "hosts"  | "host"                  => Some(Self::Hosts),
            "adblock"| "abp" | "ublock"        => Some(Self::Adblock),
            "url"    | "urls" | "tracker" | "trackers" => Some(Self::Url),
            "dnsmasq"                          => Some(Self::Dnsmasq),
            "rpz"                              => Some(Self::Rpz),
            "unbound"                          => Some(Self::Unbound),
            "ipset"                            => Some(Self::Ipset),
            "clash"  | "surge"                 => Some(Self::Clash),
            "ip"     | "ips" | "cidr"          => Some(Self::Ip),
            _                                  => None,
        }
    }
}

pub fn parse(fmt: Format, content: &str) -> Vec<String> {
    match fmt {
        Format::Auto    => parse_auto(content),
        Format::Domain  => parse_oneper(content),
        Format::Hosts   => parse_hosts(content),
        Format::Adblock => parse_adblock(content),
        Format::Url     => parse_url(content),
        Format::Dnsmasq => parse_dnsmasq(content),
        Format::Rpz     => parse_rpz(content),
        Format::Unbound => parse_unbound(content),
        Format::Ipset   => parse_ipset(content),
        Format::Clash   => parse_clash(content),
        Format::Ip      => parse_oneper(content),
    }
}

// Shared helpers used by multiple parsers

pub fn trim_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None    => line,
    }
}

pub fn strip_crlf(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Format::parse ─────────────────────────────────────────────────────────

    #[test]
    fn format_parse_all_variants() {
        assert_eq!(Format::parse("auto"),     Some(Format::Auto));
        assert_eq!(Format::parse("domain"),   Some(Format::Domain));
        assert_eq!(Format::parse("domains"),  Some(Format::Domain));
        assert_eq!(Format::parse("hosts"),    Some(Format::Hosts));
        assert_eq!(Format::parse("host"),     Some(Format::Hosts));
        assert_eq!(Format::parse("adblock"),  Some(Format::Adblock));
        assert_eq!(Format::parse("abp"),      Some(Format::Adblock));
        assert_eq!(Format::parse("ublock"),   Some(Format::Adblock));
        assert_eq!(Format::parse("url"),      Some(Format::Url));
        assert_eq!(Format::parse("urls"),     Some(Format::Url));
        assert_eq!(Format::parse("tracker"),  Some(Format::Url));
        assert_eq!(Format::parse("trackers"), Some(Format::Url));
        assert_eq!(Format::parse("dnsmasq"),  Some(Format::Dnsmasq));
        assert_eq!(Format::parse("rpz"),      Some(Format::Rpz));
        assert_eq!(Format::parse("unbound"),  Some(Format::Unbound));
        assert_eq!(Format::parse("ipset"),    Some(Format::Ipset));
        assert_eq!(Format::parse("clash"),    Some(Format::Clash));
        assert_eq!(Format::parse("surge"),    Some(Format::Clash));
        assert_eq!(Format::parse("ip"),       Some(Format::Ip));
        assert_eq!(Format::parse("ips"),      Some(Format::Ip));
        assert_eq!(Format::parse("cidr"),     Some(Format::Ip));
    }

    #[test]
    fn format_parse_unknown_returns_none() {
        assert_eq!(Format::parse("unknown"), None);
        assert_eq!(Format::parse(""), None);
        assert_eq!(Format::parse("AUTO"), None);
    }

    // ── parse dispatch ────────────────────────────────────────────────────────

    #[test]
    fn dispatch_hosts_format() {
        let out = parse(Format::Hosts, "0.0.0.0 example.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn dispatch_domain_format() {
        let out = parse(Format::Domain, "example.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn dispatch_ip_uses_oneper() {
        let out = parse(Format::Ip, "1.2.3.4\n10.0.0.0/8\n");
        assert_eq!(out, ["1.2.3.4", "10.0.0.0/8"]);
    }

    #[test]
    fn dispatch_adblock_format() {
        let out = parse(Format::Adblock, "||example.com^\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn dispatch_dnsmasq_format() {
        let out = parse(Format::Dnsmasq, "address=/example.com/0.0.0.0\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn dispatch_ipset_format() {
        let out = parse(Format::Ipset, "add myset 1.2.3.4\n");
        assert_eq!(out, ["1.2.3.4"]);
    }

    #[test]
    fn dispatch_clash_format() {
        let out = parse(Format::Clash, "DOMAIN,example.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn dispatch_rpz_format() {
        let out = parse(Format::Rpz, "example.com CNAME .\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn dispatch_unbound_format() {
        let out = parse(Format::Unbound, "local-zone: \"example.com\" always_nxdomain\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn dispatch_url_format() {
        let out = parse(Format::Url, "udp://tracker.example.com:1337/announce\n");
        assert_eq!(out, ["tracker.example.com"]);
    }

    // ── shared helpers ────────────────────────────────────────────────────────

    #[test]
    fn trim_comment_strips_hash() {
        assert_eq!(trim_comment("example.com # blocked"), "example.com ");
    }

    #[test]
    fn trim_comment_no_comment() {
        assert_eq!(trim_comment("example.com"), "example.com");
    }

    #[test]
    fn trim_comment_leading_hash() {
        assert_eq!(trim_comment("# comment"), "");
    }

    #[test]
    fn strip_crlf_removes_cr() {
        assert_eq!(strip_crlf("example.com\r"), "example.com");
    }

    #[test]
    fn strip_crlf_no_cr() {
        assert_eq!(strip_crlf("example.com"), "example.com");
    }
}
