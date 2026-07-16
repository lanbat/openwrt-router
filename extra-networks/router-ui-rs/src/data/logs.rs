use tokio::process::Command;

#[derive(Default, Clone)]
pub struct LogData {
    pub lines: Vec<String>,
}

pub async fn fetch() -> LogData {
    let output = Command::new("logread")
        .output()
        .await
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    // Keep last 500 lines
    let lines: Vec<String> = output
        .lines()
        .rev()
        .take(500)
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    LogData { lines }
}

impl LogData {
    /// Lines containing the given prefix (e.g. "EXTNET-2LAN-guest:")
    pub fn grep(&self, prefix: &str) -> Vec<&str> {
        self.lines
            .iter()
            .filter(|l| l.contains(prefix))
            .map(|s| s.as_str())
            .collect()
    }

    /// Parse a log line's timestamp field (field index 3, e.g. "12:34:56")
    pub fn line_ts(line: &str) -> &str {
        line.split_whitespace().nth(3).unwrap_or("")
    }
}

/// Parse kernel netfilter log fields (SRC=, DST=, PROTO=, DPT=) from a log line.
pub struct NfFields<'a> {
    pub src: &'a str,
    pub dst: &'a str,
    pub proto: &'a str,
    pub dpt: &'a str,
}

pub fn parse_nf_fields(line: &str) -> Option<NfFields<'_>> {
    let mut src = "";
    let mut dst = "";
    let mut proto = "";
    let mut dpt = "";

    for tok in line.split_whitespace() {
        if let Some(v) = tok.strip_prefix("SRC=") {
            src = v;
        } else if let Some(v) = tok.strip_prefix("DST=") {
            dst = v;
        } else if let Some(v) = tok.strip_prefix("PROTO=") {
            proto = v;
        } else if let Some(v) = tok.strip_prefix("DPT=") {
            dpt = v;
        }
    }

    if src.is_empty() || dst.is_empty() || proto.is_empty() {
        return None;
    }
    Some(NfFields { src, dst, proto, dpt })
}
