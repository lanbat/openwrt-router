use super::{strip_crlf, trim_comment};

/// ipset save format: `add setname 1.2.3.4`
pub fn parse_ipset(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = trim_comment(strip_crlf(raw)).trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        if fields.next() == Some("add") {
            fields.next(); // set name
            if let Some(ip) = fields.next() {
                out.push(ip.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_add_command() {
        let out = parse_ipset("add myset 1.2.3.4\n");
        assert_eq!(out, ["1.2.3.4"]);
    }

    #[test]
    fn add_with_cidr() {
        let out = parse_ipset("add myset 10.0.0.0/8\n");
        assert_eq!(out, ["10.0.0.0/8"]);
    }

    #[test]
    fn skips_non_add_commands() {
        let out = parse_ipset("create myset hash:ip\ndestroy myset\n");
        assert!(out.is_empty());
    }

    #[test]
    fn skips_comment_lines() {
        let out = parse_ipset("# ipset save output\nadd myset 1.2.3.4\n");
        assert_eq!(out, ["1.2.3.4"]);
    }

    #[test]
    fn skips_empty_lines() {
        let out = parse_ipset("\nadd myset 5.6.7.8\n\n");
        assert_eq!(out, ["5.6.7.8"]);
    }

    #[test]
    fn multiple_entries() {
        let out = parse_ipset("add s 1.1.1.1\nadd s 2.2.2.2\nadd s 3.3.3.3\n");
        assert_eq!(out, ["1.1.1.1", "2.2.2.2", "3.3.3.3"]);
    }

    #[test]
    fn real_ipset_save_format() {
        let input = "\
create my_set hash:ip family inet hashsize 1024 maxelem 65536
add my_set 1.2.3.4
add my_set 5.6.7.0/24
add my_set 8.8.8.8
";
        let out = parse_ipset(input);
        assert_eq!(out, ["1.2.3.4", "5.6.7.0/24", "8.8.8.8"]);
    }
}
