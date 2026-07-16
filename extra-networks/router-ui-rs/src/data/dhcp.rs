#[derive(Default, Clone)]
pub struct Lease {
    pub expiry: u64,
    pub mac: String,
    pub ip: String,
    pub hostname: String,
}

pub async fn fetch() -> Vec<Lease> {
    let content = tokio::fs::read_to_string("/tmp/dhcp.leases")
        .await
        .unwrap_or_default();
    parse(&content)
}

fn parse(content: &str) -> Vec<Lease> {
    content
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                return None;
            }
            Some(Lease {
                expiry: parts[0].parse().unwrap_or(0),
                mac: parts[1].to_lowercase(),
                ip: parts[2].to_string(),
                hostname: parts[3].to_string(),
            })
        })
        .collect()
}
