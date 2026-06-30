# openwrt-router

Two cooperating toolkits for OpenWrt routers, managed from a single repo: one segments your WiFi into isolated trust zones with approval workflows and live monitoring, the other selectively routes traffic through a VPN by domain or category instead of tunneling everything.

## Sub-projects

### [extra-networks](extra-networks/README.md)

Isolated WiFi networks (guest, untrusted IoT) with push notifications,
join approval, per-device outbound control, password rotation, and a
web dashboard. Uses fw4/nftables and dnsmasq.

### [split-routing](split-routing/docs/mullvad-routing.md)

Routes specific domains and IPs through a WireGuard VPN without
moving the default gateway. Supports domain lists in many formats,
dnsmasq-based lazy resolution, and daily auto-updates.

## Install

Clone onto the router and run the top-level installer:

```sh
git clone <repo-url> /root/openwrt-router
cd /root/openwrt-router
sh install.sh
```

To install only one sub-project:

```sh
sh install.sh extra-networks
sh install.sh split-routing
```

## How they interact

- Both write to `/etc/dnsmasq.d/` and `/etc/nftables.d/` with distinct
  filenames — no conflicts.
- split-routing's VPN mark chain automatically excludes traffic from
  extra-network bridges (`br-guest`, `br-untrusted`, etc.) so isolated
  network traffic always uses the normal WAN, never the VPN tunnel.
- `nft-resolve` (installed by split-routing) is available to extra-networks
  for bulk domain resolution on device allowlist pages.
