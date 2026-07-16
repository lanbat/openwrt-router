use super::{strip_crlf, trim_comment};

/// One entry per line: domain or IP, optional # comment. Takes the first field.
pub fn parse_oneper(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = trim_comment(strip_crlf(raw)).trim();
        if line.is_empty() || line.starts_with('!') || line.starts_with(';') {
            continue;
        }
        if let Some(first) = line.split_whitespace().next() {
            out.push(first.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_domain() {
        let out = parse_oneper("example.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn plain_ip() {
        let out = parse_oneper("1.2.3.4\n");
        assert_eq!(out, ["1.2.3.4"]);
    }

    #[test]
    fn strips_hash_comment() {
        let out = parse_oneper("example.com # this is blocked\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_hash_only_line() {
        let out = parse_oneper("# this is a comment\nexample.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_exclamation_line() {
        let out = parse_oneper("! comment line\nexample.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_semicolon_line() {
        let out = parse_oneper("; semicolon comment\nexample.com\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn skips_empty_lines() {
        let out = parse_oneper("\nexample.com\n\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn takes_first_field_ignoring_extra_words() {
        // extra words after domain are treated as trailing — first field wins
        let out = parse_oneper("example.com extra garbage\n");
        assert_eq!(out, ["example.com"]);
    }

    #[test]
    fn multiple_entries() {
        let out = parse_oneper("a.com\nb.com\nc.com\n");
        assert_eq!(out, ["a.com", "b.com", "c.com"]);
    }

    #[test]
    fn crlf_line_endings() {
        let out = parse_oneper("a.com\r\nb.com\r\n");
        assert_eq!(out, ["a.com", "b.com"]);
    }
}
