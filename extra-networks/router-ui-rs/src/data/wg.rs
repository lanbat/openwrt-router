use tokio::process::Command;

#[derive(Default, Clone)]
pub struct WgServer {
    pub name: String,
    pub peers: Vec<WgPeer>,
}

#[derive(Clone)]
pub struct WgPeer {
    pub online: bool,
    pub label: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub last_seen: String,
    pub traffic: String,
}

pub async fn fetch_servers(now_ts: u64) -> Vec<WgServer> {
    let uci_raw = Command::new("uci")
        .args(["show", "network"])
        .output()
        .await
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    // Find WireGuard interfaces
    let mut wg_ifaces: Vec<String> = Vec::new();
    for line in uci_raw.lines() {
        if line.contains(".proto='wireguard'") {
            if let Some(section) = line.splitn(2, '.').next() {
                let iface = section.trim_start_matches("network.");
                wg_ifaces.push(iface.to_string());
            }
        }
    }

    let mut servers = Vec::new();

    for iface in wg_ifaces {
        // Only show server-mode interfaces (no endpoint_host on any peer)
        if !is_server(&iface, &uci_raw) {
            continue;
        }

        let dump = Command::new("wg")
            .args(["show", &iface, "dump"])
            .output()
            .await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        // Skip header line (the interface itself)
        let peer_lines: Vec<&str> = dump.lines().skip(1).collect();

        let mut peers = Vec::new();

        // Enumerate UCI peers
        let mut idx = 0;
        loop {
            let pk_key = format!("network.@wireguard_{iface}[{idx}].public_key");
            let pk = uci_get(&uci_raw, &pk_key);
            if pk.is_empty() {
                break;
            }
            let desc = uci_get(&uci_raw, &format!("network.@wireguard_{iface}[{idx}].description"));
            let aips = uci_get(&uci_raw, &format!("network.@wireguard_{iface}[{idx}].allowed_ips"));
            idx += 1;

            let label = if desc.is_empty() {
                format!("{}…", &pk[..8.min(pk.len())])
            } else {
                desc
            };

            // Find matching dump line by public key
            let peer_line = peer_lines.iter().find(|l| l.starts_with(&pk));

            let (online, endpoint, last_seen, traffic) = if let Some(line) = peer_line {
                let cols: Vec<&str> = line.split('\t').collect();
                let ep = cols.get(2).copied().unwrap_or("").to_string();
                let ep = if ep == "(none)" { String::new() } else { ep };
                let hs: u64 = cols.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
                let rx: u64 = cols.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
                let tx: u64 = cols.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);

                let online = hs > 0 && (now_ts.saturating_sub(hs) < 180);
                let last_seen = if hs == 0 {
                    "—".to_string()
                } else {
                    format_ago(now_ts.saturating_sub(hs))
                };
                let traffic = if rx > 0 || tx > 0 {
                    format!("{} / {}", human_bytes(rx), human_bytes(tx))
                } else {
                    "—".to_string()
                };
                (online, ep, last_seen, traffic)
            } else {
                (false, String::new(), "—".to_string(), "—".to_string())
            };

            peers.push(WgPeer {
                online,
                label,
                endpoint,
                allowed_ips: aips.replace(' ', ", "),
                last_seen,
                traffic,
            });
        }

        if !peers.is_empty() {
            servers.push(WgServer { name: iface, peers });
        }
    }

    servers
}

fn is_server(iface: &str, uci_raw: &str) -> bool {
    // Server if none of the peers have endpoint_host set
    let mut idx = 0;
    loop {
        let pk_key = format!("network.@wireguard_{iface}[{idx}].public_key");
        if uci_get(uci_raw, &pk_key).is_empty() {
            break;
        }
        let ep_key = format!("network.@wireguard_{iface}[{idx}].endpoint_host");
        if !uci_get(uci_raw, &ep_key).is_empty() {
            return false;
        }
        idx += 1;
    }
    true
}

fn uci_get(raw: &str, key: &str) -> String {
    // key format: network.section.option
    // uci show output: network.section.option='value'
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            if rest.starts_with('=') {
                return rest[1..].trim().trim_matches('\'').to_string();
            }
        }
    }
    String::new()
}

fn format_ago(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

pub fn human_bytes(b: u64) -> String {
    if b >= 1_073_741_824 {
        format!("{:.1} GB", b as f64 / 1_073_741_824.0)
    } else if b >= 1_048_576 {
        format!("{:.1} MB", b as f64 / 1_048_576.0)
    } else if b >= 1024 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else {
        format!("{b} B")
    }
}
