use std::fmt::Write as FmtWrite;
use tokio::process::Command;

pub struct NftConfig<'a> {
    pub family: &'a str,
    pub table:  &'a str,
    pub set4:   &'a str,
    pub set6:   &'a str,
    pub ip4:    &'a [String],
    pub ip6:    &'a [String],
    pub chunk4: usize,
    pub chunk6: usize,
}

pub async fn apply(cfg: &NftConfig<'_>) -> anyhow::Result<()> {
    // Ensure the table exists
    let check = Command::new("nft")
        .args(["list", "table", cfg.family, cfg.table])
        .output()
        .await?;
    if !check.status.success() {
        anyhow::bail!(
            "nft table does not exist: {} {}",
            cfg.family, cfg.table
        );
    }

    // Ensure sets exist (idempotent)
    if cfg.set4 != "-" {
        let _ = Command::new("nft")
            .args([
                "add", "set", cfg.family, cfg.table, cfg.set4,
                "{ type ipv4_addr; flags interval; }",
            ])
            .status()
            .await;
    }
    if cfg.set6 != "-" {
        let _ = Command::new("nft")
            .args([
                "add", "set", cfg.family, cfg.table, cfg.set6,
                "{ type ipv6_addr; flags interval; }",
            ])
            .status()
            .await;
    }

    let cmds = build_commands(cfg);

    let mut child = Command::new("nft")
        .arg("-f")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(cmds.as_bytes()).await?;
    }

    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("nft -f - failed with status {status}");
    }
    Ok(())
}

fn build_commands(cfg: &NftConfig<'_>) -> String {
    let mut s = String::new();
    if cfg.set4 != "-" {
        emit_set(&mut s, cfg.family, cfg.table, cfg.set4, cfg.ip4, cfg.chunk4);
    }
    if cfg.set6 != "-" {
        emit_set(&mut s, cfg.family, cfg.table, cfg.set6, cfg.ip6, cfg.chunk6);
    }
    s
}

fn emit_set(
    out: &mut String,
    family: &str,
    table: &str,
    set: &str,
    ips: &[String],
    chunk: usize,
) {
    writeln!(out, "flush set {family} {table} {set}").unwrap();
    for chunk_ips in ips.chunks(chunk) {
        let elements = chunk_ips.join(", ");
        writeln!(out, "add element {family} {table} {set} {{ {elements} }}").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg<'a>(
        ip4: &'a [String],
        ip6: &'a [String],
    ) -> NftConfig<'a> {
        NftConfig {
            family: "inet",
            table:  "fw",
            set4:   "block4",
            set6:   "block6",
            ip4,
            ip6,
            chunk4: 100,
            chunk6: 100,
        }
    }

    #[test]
    fn empty_lists_produce_only_flush() {
        let c = cfg(&[], &[]);
        let out = build_commands(&c);
        assert_eq!(out, "flush set inet fw block4\nflush set inet fw block6\n");
    }

    #[test]
    fn single_ipv4_entry() {
        let ips4 = vec!["1.2.3.4".to_string()];
        let c = cfg(&ips4, &[]);
        let out = build_commands(&c);
        assert!(out.contains("flush set inet fw block4\n"), "missing flush4");
        assert!(out.contains("add element inet fw block4 { 1.2.3.4 }\n"), "missing add4");
        assert!(out.contains("flush set inet fw block6\n"), "missing flush6");
    }

    #[test]
    fn single_ipv6_entry() {
        let ips6 = vec!["2001:db8::/32".to_string()];
        let c = cfg(&[], &ips6);
        let out = build_commands(&c);
        assert!(out.contains("add element inet fw block6 { 2001:db8::/32 }\n"));
    }

    #[test]
    fn chunk_splits_into_multiple_add_commands() {
        let ips4: Vec<String> = (1u8..=5).map(|i| format!("1.2.3.{i}")).collect();
        let c = NftConfig {
            family: "inet",
            table:  "fw",
            set4:   "b4",
            set6:   "-",
            ip4:    &ips4,
            ip6:    &[],
            chunk4: 2,
            chunk6: 100,
        };
        let out = build_commands(&c);
        let add_count = out.lines().filter(|l| l.starts_with("add element")).count();
        assert_eq!(add_count, 3, "5 ips with chunk=2 → 3 add commands");
    }

    #[test]
    fn set4_dash_skips_ipv4() {
        let ips4 = vec!["1.2.3.4".to_string()];
        let c = NftConfig {
            family: "ip",
            table:  "t",
            set4:   "-",
            set6:   "s6",
            ip4:    &ips4,
            ip6:    &[],
            chunk4: 100,
            chunk6: 100,
        };
        let out = build_commands(&c);
        assert!(!out.contains("1.2.3.4"), "ipv4 should be skipped when set4 is '-'");
        assert!(out.contains("flush set ip t s6\n"));
    }

    #[test]
    fn set6_dash_skips_ipv6() {
        let ips6 = vec!["::1".to_string()];
        let c = NftConfig {
            family: "ip",
            table:  "t",
            set4:   "s4",
            set6:   "-",
            ip4:    &[],
            ip6:    &ips6,
            chunk4: 100,
            chunk6: 100,
        };
        let out = build_commands(&c);
        assert!(!out.contains("::1"), "ipv6 should be skipped when set6 is '-'");
        assert!(out.contains("flush set ip t s4\n"));
    }

    #[test]
    fn multiple_ips_joined_in_one_chunk() {
        let ips4 = vec!["1.1.1.1".to_string(), "2.2.2.2".to_string(), "3.3.3.3".to_string()];
        let c = NftConfig {
            family: "inet",
            table:  "fw",
            set4:   "b4",
            set6:   "-",
            ip4:    &ips4,
            ip6:    &[],
            chunk4: 100,
            chunk6: 100,
        };
        let out = build_commands(&c);
        assert!(out.contains("add element inet fw b4 { 1.1.1.1, 2.2.2.2, 3.3.3.3 }\n"));
    }
}
