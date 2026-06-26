#!/bin/sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CONFIG="$SCRIPT_DIR/config"

[ -f "$CONFIG" ] || { echo "ERROR: copy config.example to config and set WIFI_KEY"; exit 1; }
. "$CONFIG"

[ -z "${WIFI_KEY:-}" ] && { echo "ERROR: WIFI_KEY is not set in config"; exit 1; }
[ ${#WIFI_KEY} -lt 8 ]  && { echo "ERROR: WIFI_KEY must be at least 8 characters"; exit 1; }

# ── network ──────────────────────────────────────────────────────────────────

# Explicit bridge device required for DSA — netifd won't auto-create one for WiFi-only networks.
uci -q delete network."br_${IFACE}" || true
uci set network."br_${IFACE}"=device
uci set network."br_${IFACE}".name="br-${IFACE}"
uci set network."br_${IFACE}".type=bridge

uci -q delete network."$IFACE" || true
uci set network."$IFACE"=interface
uci set network."$IFACE".proto=static
uci set network."$IFACE".device="br-${IFACE}"
uci set network."$IFACE".ipaddr="${SUBNET}.1"
uci set network."$IFACE".netmask=255.255.255.0

uci commit network

# ── dhcp ───────────────────────────────────────────────────────────────────────

uci -q delete dhcp."$IFACE" || true
uci set dhcp."$IFACE"=dhcp
uci set dhcp."$IFACE".interface="$IFACE"
uci set dhcp."$IFACE".start=100
uci set dhcp."$IFACE".limit=150
uci set dhcp."$IFACE".leasetime=12h
# Hand out the filtered DNS server directly — clients never talk to the router's dnsmasq.
uci set dhcp."$IFACE".dhcp_option="6,$DNS_SERVER"

uci commit dhcp

# ── firewall ───────────────────────────────────────────────────────────────────

# Zone: untrusted devices can reach the internet, nothing else.
uci -q delete firewall."${IFACE}_zone" || true
uci set firewall."${IFACE}_zone"=zone
uci set firewall."${IFACE}_zone".name="$IFACE"
uci set firewall."${IFACE}_zone".network="$IFACE"
uci set firewall."${IFACE}_zone".input=REJECT
uci set firewall."${IFACE}_zone".output=ACCEPT
uci set firewall."${IFACE}_zone".forward=REJECT

# Allow DHCP requests (needed before a client has an IP).
uci -q delete firewall."${IFACE}_dhcp" || true
uci set firewall."${IFACE}_dhcp"=rule
uci set firewall."${IFACE}_dhcp".name="Allow-DHCP-${IFACE}"
uci set firewall."${IFACE}_dhcp".src="$IFACE"
uci set firewall."${IFACE}_dhcp".proto=udp
uci set firewall."${IFACE}_dhcp".dest_port=67
uci set firewall."${IFACE}_dhcp".target=ACCEPT

# Forward untrusted → WAN (internet access).
uci -q delete firewall."${IFACE}_wan" || true
uci set firewall."${IFACE}_wan"=forwarding
uci set firewall."${IFACE}_wan".src="$IFACE"
uci set firewall."${IFACE}_wan".dest=wan

# Block DNS to anything except the assigned server, so clients can't bypass filtering.
uci -q delete firewall."${IFACE}_dns_block" || true
uci set firewall."${IFACE}_dns_block"=rule
uci set firewall."${IFACE}_dns_block".name="Block-DNS-bypass-${IFACE}"
uci set firewall."${IFACE}_dns_block".src="$IFACE"
uci set firewall."${IFACE}_dns_block".dest=wan
uci set firewall."${IFACE}_dns_block".dest_ip="!$DNS_SERVER"
uci set firewall."${IFACE}_dns_block".proto="tcp udp"
uci set firewall."${IFACE}_dns_block".dest_port=53
uci set firewall."${IFACE}_dns_block".target=REJECT

uci commit firewall

# ── wireless ───────────────────────────────────────────────────────────────────

uci set wireless."$WIFI_UCI".ssid="$SSID"
uci set wireless."$WIFI_UCI".key="$WIFI_KEY"
uci set wireless."$WIFI_UCI".encryption=psk+psk2  # WPA/WPA2 mixed for legacy device compatibility
uci set wireless."$WIFI_UCI".network="$IFACE"
uci set wireless."$WIFI_UCI".disabled=0

uci commit wireless

# ── rate limiting via nftables ─────────────────────────────────────────────────
# tc tbf isn't available (kmod-sched-core not packaged); use nft drop instead.
# Convert RATE_LIMIT (e.g. "1mbit") to kbytes/second for nft.
# nft uses bytes/second: 1mbit = 125 kbytes/second.
_rate_kbps=$(echo "$RATE_LIMIT" | sed 's/mbit$//; s/[Mm]bit$//' )
_rate_kbytes=$(( _rate_kbps * 125 ))

NFT_RATE_SCRIPT=/etc/nftables.d/20-untrusted-ratelimit.nft
mkdir -p /etc/nftables.d
# Included inside table inet fw4 by fw4, so chain syntax (no 'add rule' prefix).
cat >"$NFT_RATE_SCRIPT" <<EOF
chain untrusted_ratelimit {
    type filter hook forward priority 1; policy accept;
    iifname "br-${IFACE}" limit rate over ${_rate_kbytes} kbytes/second drop
    oifname "br-${IFACE}" limit rate over ${_rate_kbytes} kbytes/second drop
}
EOF

# ── apply ──────────────────────────────────────────────────────────────────────

/etc/init.d/network reload
/etc/init.d/dnsmasq restart
fw4 reload
wifi reload

# ── summary ────────────────────────────────────────────────────────────────────

echo "Installed."
echo "  Network:   $IFACE — ${SUBNET}.1/24, DHCP ${SUBNET}.100–${SUBNET}.249"
echo "  DNS:       $DNS_SERVER (via DHCP option 6, bypass blocked)"
echo "  Wireless:  $SSID on UCI '$WIFI_UCI' (WPA/WPA2, $IFACE bridge)"
echo "  Rate:      $RATE_LIMIT cap (nft drop) on br-${IFACE}"
echo "  Firewall:  ${IFACE}→WAN yes  |  ${IFACE}→LAN no  |  ${IFACE}→router no"
