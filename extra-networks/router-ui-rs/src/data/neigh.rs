use std::collections::HashMap;
use tokio::process::Command;

/// mac → list of IPs seen on that device
#[derive(Default, Clone)]
pub struct NeighTable {
    /// ip → (mac, state)
    pub by_ip: HashMap<String, (String, String)>,
    /// mac → [ipv6 global]
    pub ip6_by_mac: HashMap<String, Vec<String>>,
}

pub async fn fetch() -> NeighTable {
    let v4 = run_neigh(false).await;
    let v6 = run_neigh(true).await;

    let mut table = NeighTable::default();

    for line in v4.lines().chain(v6.lines()) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let ip = parts[0];
        let state = parts.last().unwrap_or(&"").to_string();
        let mac = parts
            .windows(2)
            .find(|w| w[0] == "lladdr")
            .map(|w| w[1].to_lowercase());

        if let Some(mac) = mac {
            table.by_ip.insert(ip.to_string(), (mac.clone(), state));

            // Track IPv6 global addresses per MAC
            if ip.contains(':') && !ip.starts_with("fe80") {
                table
                    .ip6_by_mac
                    .entry(mac)
                    .or_default()
                    .push(ip.to_string());
            }
        } else {
            // Still track reachability even without MAC (for IPv6)
            table
                .by_ip
                .insert(ip.to_string(), (String::new(), state));
        }
    }

    table
}

async fn run_neigh(ipv6: bool) -> String {
    let mut cmd = Command::new("ip");
    if ipv6 {
        cmd.arg("-6");
    }
    cmd.args(["neigh", "show"]);
    cmd.output()
        .await
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
}

impl NeighTable {
    pub fn is_reachable(&self, ip: &str) -> bool {
        self.by_ip
            .get(ip)
            .map(|(_, state)| {
                matches!(state.as_str(), "REACHABLE" | "DELAY" | "PROBE")
            })
            .unwrap_or(false)
    }

    pub fn mac_for_ip(&self, ip: &str) -> Option<&str> {
        self.by_ip
            .get(ip)
            .map(|(mac, _)| mac.as_str())
            .filter(|s| !s.is_empty())
    }

    /// First global IPv6 for a MAC
    pub fn ip6_for_mac(&self, mac: &str) -> Option<&str> {
        self.ip6_by_mac
            .get(&mac.to_lowercase())
            .and_then(|v| v.first())
            .map(|s| s.as_str())
    }
}
