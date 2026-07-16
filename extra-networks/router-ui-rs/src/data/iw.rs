use std::collections::HashMap;
use tokio::process::Command;

#[derive(Default, Clone)]
pub struct IwState {
    pub phys: Vec<WifiPhy>,
    /// vap interface name → (channel, band)
    pub vap_info: HashMap<String, (String, String)>,
}

#[derive(Clone)]
pub struct WifiPhy {
    pub name: String,
    pub band: String,
    pub channel: String,
    pub vap_count: usize,
    pub expected_vaps: usize,
    pub status_ok: bool,
    pub status_label: String,
}

pub async fn fetch() -> IwState {
    let iw_raw = Command::new("iw")
        .arg("dev")
        .output()
        .await
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let uci_raw = Command::new("uci")
        .args(["show", "wireless"])
        .output()
        .await
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    parse(&iw_raw, &uci_raw)
}

struct VapRecord {
    phy: String,
    iface: String,
    channel: String,
    band: String,
}

fn parse(iw_raw: &str, uci_raw: &str) -> IwState {
    let mut vaps: Vec<VapRecord> = Vec::new();
    let mut cur_phy = String::new();
    let mut cur_iface = String::new();
    let mut cur_ch = String::new();
    let mut cur_band = String::new();

    for line in iw_raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = line.strip_prefix("phy#") {
            commit(&mut vaps, &cur_phy, &cur_iface, &cur_ch, &cur_band);
            cur_phy = format!("phy{}", rest.split_whitespace().next().unwrap_or("?"));
            cur_iface.clear();
            cur_ch.clear();
            cur_band.clear();
        } else if let Some(rest) = trimmed.strip_prefix("Interface ") {
            commit(&mut vaps, &cur_phy, &cur_iface, &cur_ch, &cur_band);
            cur_iface = rest.split_whitespace().next().unwrap_or("").to_string();
            cur_ch.clear();
            cur_band.clear();
        } else if let Some(rest) = trimmed.strip_prefix("channel ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            cur_ch = parts.first().copied().unwrap_or("").to_string();
            let freq: u32 = parts
                .get(1)
                .and_then(|s| s.trim_start_matches('(').parse().ok())
                .unwrap_or(0);
            cur_band = freq_to_band(freq).to_string();
        }
    }
    commit(&mut vaps, &cur_phy, &cur_iface, &cur_ch, &cur_band);

    // Count expected VAPs per radio from `uci show wireless`
    // Lines like: wireless.<section>.device='radio0'
    let mut expected: HashMap<String, usize> = HashMap::new();
    for line in uci_raw.lines() {
        if line.contains(".device=") {
            if let Some(val) = line.splitn(2, '=').nth(1) {
                let radio = val.trim().trim_matches('\'');
                if radio.starts_with("radio") {
                    *expected.entry(radio.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    // Group vaps by phy
    let mut phy_groups: HashMap<&str, Vec<&VapRecord>> = HashMap::new();
    for vap in &vaps {
        phy_groups.entry(&vap.phy).or_default().push(vap);
    }

    let mut phy_names: Vec<&str> = phy_groups.keys().copied().collect();
    phy_names.sort_unstable();

    let mut phys = Vec::new();
    let mut vap_info: HashMap<String, (String, String)> = HashMap::new();

    for phy_name in phy_names {
        let group = &phy_groups[phy_name];
        let vap_count = group.len();
        let band = group.iter().find_map(|v| {
            (!v.band.is_empty()).then(|| v.band.clone())
        }).unwrap_or_default();
        let channel = group.iter().find_map(|v| {
            (!v.channel.is_empty()).then(|| v.channel.clone())
        }).unwrap_or_default();

        let radio_name = format!("radio{}", phy_name.trim_start_matches("phy"));
        let exp = expected.get(&radio_name).copied().unwrap_or(0);

        let status_ok = vap_count > 0 && (exp == 0 || vap_count >= exp);
        let status_label = if vap_count == 0 {
            "down — no VAPs".to_string()
        } else if exp > 0 && vap_count < exp {
            format!("{vap_count} of {exp} VAPs up")
        } else {
            format!("{vap_count} VAP{}", if vap_count != 1 { "s" } else { "" })
        };

        for vap in group {
            vap_info.insert(vap.iface.clone(), (vap.channel.clone(), vap.band.clone()));
        }

        phys.push(WifiPhy {
            name: phy_name.to_string(),
            band,
            channel,
            vap_count,
            expected_vaps: exp,
            status_ok,
            status_label,
        });
    }

    IwState { phys, vap_info }
}

fn commit(vaps: &mut Vec<VapRecord>, phy: &str, iface: &str, ch: &str, band: &str) {
    if !iface.is_empty() {
        vaps.push(VapRecord {
            phy: phy.to_string(),
            iface: iface.to_string(),
            channel: ch.to_string(),
            band: band.to_string(),
        });
    }
}

fn freq_to_band(freq: u32) -> &'static str {
    if freq > 0 && freq < 3000 {
        "2.4 GHz"
    } else if freq >= 5000 && freq < 6000 {
        "5 GHz"
    } else if freq >= 6000 {
        "6 GHz"
    } else {
        ""
    }
}
