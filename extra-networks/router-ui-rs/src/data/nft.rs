use std::collections::HashMap;
use tokio::process::Command;

#[derive(Default, Clone)]
pub struct NftState {
    pub raw: String,
}

pub async fn fetch() -> NftState {
    let raw = Command::new("nft")
        .args(["-j", "list", "ruleset"])
        .output()
        .await
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        // fallback: plain text if -j not available / not useful
        .or_else(|| {
            // We'll parse the text format instead
            None
        })
        .unwrap_or_default();

    NftState { raw }
}

impl NftState {
    /// Total bytes for a counter chain (in or out direction).
    /// Parses: `nft list chain inet fw4 <chain>` text output cached in raw.
    /// Since we have the full ruleset, we search within it.
    pub fn chain_bytes(&self, chain: &str, direction: &str) -> u64 {
        // Parse from `nft list ruleset` text output
        // We look for the chain block and then extract the counter bytes
        let iface_kw = if direction == "in" { "iifname" } else { "oifname" };

        let mut in_chain = false;
        let mut depth = 0usize;

        for line in self.raw.lines() {
            let trimmed = line.trim();

            if trimmed == format!("chain {chain} {{") || trimmed.starts_with(&format!("chain {chain} {{")) {
                in_chain = true;
                depth = 1;
                continue;
            }

            if in_chain {
                depth += trimmed.chars().filter(|&c| c == '{').count();
                depth = depth.saturating_sub(trimmed.chars().filter(|&c| c == '}').count());

                if depth == 0 {
                    break;
                }

                if trimmed.contains(iface_kw) && trimmed.contains("counter") {
                    if let Some(bytes) = extract_counter_bytes(trimmed) {
                        return bytes;
                    }
                }
            }
        }
        0
    }

    /// Per-device bytes from a named set (e.g. `guest_device_bytes`).
    /// Returns ip → bytes map.
    pub fn device_bytes(&self, set_name: &str) -> HashMap<String, u64> {
        let mut result = HashMap::new();
        let mut in_set = false;

        for line in self.raw.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with(&format!("set {set_name} {{")) {
                in_set = true;
                continue;
            }

            if in_set {
                if trimmed == "}" {
                    break;
                }
                // Elements look like: 192.168.10.5 counter packets 12 bytes 34567 ,
                // or: { 192.168.10.5 counter packets 12 bytes 34567 }
                let s = trimmed.trim_matches(|c| c == '{' || c == '}').trim();
                for part in s.split(',') {
                    let part = part.trim();
                    if let Some(ip_end) = part.find(' ') {
                        let ip = &part[..ip_end];
                        if let Some(bytes) = extract_counter_bytes(part) {
                            result.insert(ip.to_string(), bytes);
                        }
                    }
                }
            }
        }
        result
    }
}

fn extract_counter_bytes(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    parts
        .windows(2)
        .find(|w| w[0] == "bytes")
        .and_then(|w| w[1].trim_end_matches(',').parse().ok())
}
