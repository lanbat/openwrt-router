use std::collections::HashMap;
use std::path::Path;

/// Configuration loaded from `<base_dir>/<iface>-notify.conf`
#[derive(Default, Clone)]
pub struct NetworkConf {
    pub iface: String,
    pub notify_url: String,
    pub subnet: String,
    pub rate_limit: String,
    pub rate_limit_per_device: String,
    pub dns_server: String,
    pub dns_server_v6: String,
    pub dot: bool,
    pub lan_access: bool,
    pub isolate: bool,
    pub notify_join: bool,
    pub join_approval: bool,
    pub join_history_retention: String,
    pub rotate_password: bool,
    pub show_qr: bool,
    pub description: String,
    pub bandwidth_threshold_mb: u64,
    pub device_control: bool,
    pub default_duration: String,
}

pub async fn read_all_network_confs(base_dir: &Path) -> Vec<NetworkConf> {
    let mut entries = match tokio::fs::read_dir(base_dir).await {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut confs = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with("-notify.conf") {
            continue;
        }
        let content = match tokio::fs::read_to_string(entry.path()).await {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(conf) = parse_notify_conf(&name, &content) {
            confs.push(conf);
        }
    }

    confs.sort_by(|a, b| a.iface.cmp(&b.iface));
    confs
}

fn parse_notify_conf(filename: &str, content: &str) -> Option<NetworkConf> {
    let vars = parse_sh_vars(content);
    let iface = vars
        .get("IFACE_NAME")
        .cloned()
        .or_else(|| {
            filename.strip_suffix("-notify.conf").map(|s| s.to_string())
        })?;

    if iface.is_empty() {
        return None;
    }

    Some(NetworkConf {
        iface: iface.clone(),
        notify_url: vars.get("NOTIFY_URL").cloned().unwrap_or_default(),
        subnet: vars.get("SUBNET").cloned().unwrap_or_default(),
        rate_limit: vars.get("RATE_LIMIT").cloned().unwrap_or_default(),
        rate_limit_per_device: vars.get("RATE_LIMIT_PER_DEVICE").cloned().unwrap_or_default(),
        dns_server: vars.get("DNS_SERVER").cloned().unwrap_or_default(),
        dns_server_v6: vars.get("DNS_SERVER_V6").cloned().unwrap_or_default(),
        dot: vars.get("DOT").map(|v| v == "yes").unwrap_or(false),
        lan_access: vars.get("LAN_ACCESS").map(|v| v == "yes").unwrap_or(false),
        isolate: vars.get("ISOLATE").map(|v| v != "no").unwrap_or(true),
        notify_join: vars.get("NOTIFY_JOIN").map(|v| v == "yes").unwrap_or(false),
        join_approval: vars.get("JOIN_APPROVAL").map(|v| v == "yes").unwrap_or(false),
        join_history_retention: vars
            .get("JOIN_HISTORY_RETENTION")
            .cloned()
            .unwrap_or_else(|| "90d".to_string()),
        rotate_password: vars.get("ROTATE_PASSWORD").map(|v| v == "yes").unwrap_or(false),
        show_qr: if iface == "untrusted" {
            false
        } else {
            vars.get("SHOW_QR").map(|v| v == "yes").unwrap_or(false)
        },
        description: vars.get("DESCRIPTION").cloned().unwrap_or_default(),
        bandwidth_threshold_mb: vars
            .get("BANDWIDTH_THRESHOLD_MB")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        device_control: vars.get("DEVICE_CONTROL").map(|v| v == "yes").unwrap_or(false),
        default_duration: vars
            .get("DEFAULT_DURATION")
            .cloned()
            .unwrap_or_else(|| "24h".to_string()),
    })
}

/// Parse `KEY=value` or `KEY='value'` from a shell-style config file.
/// Handles comments and ignores lines that are not simple assignments.
pub fn parse_sh_vars(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = &line[..eq];
            if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                let val = line[eq + 1..].trim_matches('"').trim_matches('\'').to_string();
                map.insert(key.to_string(), val);
            }
        }
    }
    map
}

/// Read lines from a simple file (one entry per line, ignoring blanks).
pub async fn read_lines(path: &Path) -> Vec<String> {
    tokio::fs::read_to_string(path)
        .await
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect()
}

/// Read a tab-separated labels file: `mac<TAB>label`
/// Returns mac → label map (lowercase MAC keys).
pub async fn read_labels(path: &Path) -> HashMap<String, String> {
    let lines = read_lines(path).await;
    lines
        .iter()
        .filter_map(|l| {
            let mut parts = l.splitn(2, '\t');
            let mac = parts.next()?.trim().to_lowercase();
            let label = parts.next()?.trim().to_string();
            if mac.is_empty() || label.is_empty() {
                None
            } else {
                Some((mac, label))
            }
        })
        .collect()
}

/// Read join-history file. Each line has tab-separated fields:
/// ts<TAB>action<TAB>mac<TAB>ip4<TAB>ip6<TAB>hostname<TAB>actor<TAB>actor_ip4<TAB>actor_ip6<TAB>actor_mac
pub async fn read_join_history(path: &Path) -> Vec<Vec<String>> {
    let lines = read_lines(path).await;
    lines
        .iter()
        .map(|l| l.split('\t').map(|s| s.to_string()).collect())
        .filter(|v: &Vec<String>| v.len() >= 3)
        .collect()
}

/// Check if a MAC is present in a simple list file (one MAC per line, case-insensitive).
pub async fn mac_in_file(path: &Path, mac: &str) -> bool {
    let mac_lc = mac.to_lowercase();
    read_lines(path)
        .await
        .iter()
        .any(|l| l.trim().to_lowercase() == mac_lc)
}

/// Read join-pending file: `mac ip` per line. Returns mac → ip map.
pub async fn read_pending(path: &Path) -> HashMap<String, String> {
    read_lines(path)
        .await
        .iter()
        .filter_map(|l| {
            let mut parts = l.splitn(2, ' ');
            let mac = parts.next()?.trim().to_lowercase();
            let ip = parts.next().unwrap_or("").trim().to_string();
            Some((mac, ip))
        })
        .collect()
}

/// Read /tmp/extra-networks-joins: `mac<TAB>timestamp`
pub async fn read_joins() -> HashMap<String, String> {
    let path = Path::new("/tmp/extra-networks-joins");
    let lines = read_lines(path).await;
    lines
        .iter()
        .filter_map(|l| {
            let mut parts = l.splitn(2, '\t');
            let mac = parts.next()?.trim().to_lowercase();
            let ts = parts.next().unwrap_or("").trim().to_string();
            Some((mac, ts))
        })
        .collect()
}
