#!/bin/sh
# Show status of all isolated networks managed by install.sh.

_hr() { printf '%.0s─' $(seq 1 60); echo; }

_zone_info() {
    zone="$1"
    iface=$(uci -q get firewall."${zone}_zone".network 2>/dev/null || echo "$zone")
    bridge="br-$iface"

    printf '\n'
    _hr
    printf '  %s\n' "$zone"
    _hr

    # Bridge state
    if ip link show "$bridge" >/dev/null 2>&1; then
        ip=$(ip addr show "$bridge" 2>/dev/null \
             | awk '/inet / { split($2,a,"/"); print a[1]; exit }')
        state=$(ip link show "$bridge" | awk '/state/ { print $9 }')
        printf '  Bridge:   %s  %s  (%s)\n' "$bridge" "$ip" "$state"
    else
        printf '  Bridge:   %s  (not up)\n' "$bridge"
        return
    fi

    # WiFi
    ssid=$(uci -q show wireless \
           | awk -F= '/\.network='"'$iface'"'/ { sec=$1; sub(/\.network/,"",sec); print sec }' \
           | head -1)
    if [ -n "$ssid" ]; then
        ssid_val=$(uci -q get wireless."$ssid".ssid 2>/dev/null || echo '?')
        enc=$(uci -q get wireless."$ssid".encryption 2>/dev/null || echo '?')
        printf '  WiFi:     %s  (%s, %s)\n' "$ssid_val" "$ssid" "$enc"
    fi

    # Connected clients
    clients=0
    for wif in $(iw dev 2>/dev/null | awk '/Interface/ { print $2 }'); do
        master=$(ip link show "$wif" 2>/dev/null | awk '/master/ { print $NF; exit }')
        [ "$master" = "$bridge" ] || continue
        while IFS= read -r line; do
            case "$line" in
                Station*) mac=$(echo "$line" | awk '{print $2}')
                          clients=$(( clients + 1 ))
                          printf '  Client:   %s (via %s)\n' "$mac" "$wif" ;;
            esac
        done < <(iw dev "$wif" station dump 2>/dev/null)
    done
    [ "$clients" -eq 0 ] && printf '  Clients:  none\n'

    # DHCP leases
    subnet_prefix=$(ip addr show "$bridge" 2>/dev/null \
                    | awk '/inet / { split($2,a,"."); print a[1]"."a[2]"."a[3] }')
    if [ -n "$subnet_prefix" ]; then
        leases=$(grep "^[0-9]" /tmp/dhcp.leases 2>/dev/null \
                 | awk -v p="$subnet_prefix" '$3 ~ p { print $3, $4, $2 }')
        if [ -n "$leases" ]; then
            echo "$leases" | while read -r lip lname lmac; do
                printf '  Lease:    %-16s %-20s %s\n' "$lip" "$lname" "$lmac"
            done
        else
            printf '  Leases:   none\n'
        fi
    fi

    # Rate limit
    rl=$(nft list chain inet fw4 "${iface}_ratelimit" 2>/dev/null \
         | awk '/limit rate over/ { print $5, $6, $7; exit }')
    [ -n "$rl" ] && printf '  Rate:     %s (aggregate)\n' "$rl"

    prl=$(nft list chain inet fw4 "${iface}_ratelimit" 2>/dev/null \
          | awk '/meter.*limit rate over/ { print $0 }' | grep -o 'limit rate over [0-9]* [a-z/]*' | head -1)
    [ -n "$prl" ] && printf '  Rate:     %s (per device)\n' "$prl"

    # Active port forwards
    fwds=$(uci show firewall 2>/dev/null \
           | awk -F= '/=redirect/ { sec=$1; sub(/=redirect/,"",sec); print sec }')
    for s in $fwds; do
        src=$(uci -q get firewall."$s".src 2>/dev/null)
        [ "$src" = "$zone" ] || continue
        name=$(uci -q get firewall."$s".name 2>/dev/null)
        port=$(uci -q get firewall."$s".src_dport 2>/dev/null)
        dest=$(uci -q get firewall."$s".dest_ip 2>/dev/null)
        printf '  Forward:  :%s → %s  (%s)\n' "$port" "$dest" "$name"
    done

    # LAN access
    lan_fwd=$(uci -q get firewall."lan_${iface}".src 2>/dev/null)
    if [ "$lan_fwd" = lan ]; then
        printf '  LAN:      can reach %s devices\n' "$zone"
    fi
}

# Find all non-lan/wan firewall zones that we manage (have a matching bridge).
zones=$(uci show firewall 2>/dev/null \
        | awk -F= '/\.name=/ && !/wan|lan/ {
            gsub(/firewall\.|\.name/,"",$1);
            gsub(/'"'"'/,"",$2);
            if ($2 != "wan" && $2 != "lan") print $2
          }')

if [ -z "$zones" ]; then
    echo "No isolated networks found."
    exit 0
fi

printf '\n  openwrt-extra-networks — network status\n'
for z in $zones; do
    _zone_info "$z"
done
printf '\n'
