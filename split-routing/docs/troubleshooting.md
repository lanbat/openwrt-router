# Troubleshooting

## Check overall state

Run this on the router to see everything at once:

```sh
echo "=== mark rules ===" && nft list chain inet fw4 mangle_prerouting
echo "=== ip rule ===" && ip rule show | grep fwmark
echo "=== routing table ===" && ip route show table 100
echo "=== set sizes ===" && for s in $(nft list sets inet fw4 | grep -o 'set [a-z_]*4' | awk '{print $2}'); do
  echo "$s: $(nft list set inet fw4 $s | grep -c expires || true) dynamic / $(nft list set inet fw4 $s | grep -c '\.' || true) interval"
done
```

## Sets stay empty after DNS queries

The most common cause is dnsmasq running without `CAP_NET_ADMIN`. Without it, dnsmasq resolves DNS normally but silently drops every nftset write.

Test it:

```sh
nft flush set inet fw4 dns_torrentsites4
nslookup thepiratebay.org 127.0.0.1 > /dev/null
sleep 1
nft list set inet fw4 dns_torrentsites4
```

If the set stays empty, check:

```sh
# Should exist and contain CAP_NET_ADMIN
cat /etc/capabilities/dnsmasq.json

# dnsmasq-full is required — standard dnsmasq has no nftset support
dnsmasq --version 2>&1 | grep -o nftset

# dnsmasq must be loading the nftset config files
ls /etc/dnsmasq.d/
uci get dhcp.@dnsmasq[0].confdir
```

If the capabilities file is missing, run `sh install.sh` — it creates the file and restarts dnsmasq. See [mullvad-routing.md](mullvad-routing.md) for the full dnsmasq setup.

## Routing stops working after reboot or fw4 reload

`fw4 reload` (triggered by WAN/PPPoE reconnects) wipes `mangle_prerouting`. The `99-mullvad-routing` hotplug script re-adds the mark rules automatically on every interface up event, as long as the VPN interface is `LOWER_UP`.

Check if the chain is empty:

```sh
nft list chain inet fw4 mangle_prerouting
```

If it is, trigger the hotplug script manually:

```sh
ACTION=ifup sh /etc/hotplug.d/iface/99-mullvad-routing
```

If the chain is populated but routing still doesn't work, check the ip rule and routing table:

```sh
ip rule show | grep fwmark   # should show: fwmark 0x1 lookup 100
ip route show table 100      # should show: default dev <vpn-interface>
```

## Testing routing from a client device

The router's own traffic bypasses `mangle_prerouting` — that hook only marks forwarded packets from LAN clients. Always test from a LAN device, not directly on the router:

```sh
curl -4 ifconfig.co   # returns your VPN IP if the domain is in a routing set
curl -4 icanhazip.com # returns your home WAN IP (not in any set)
```

## dns category reports "No domains — skipping"

The local file for that category (`/etc/split-routing/local-dns-<name>.txt`) is empty or contains only comments, and no remote URL is configured for it in `update-routing-sets`. Add at least one domain to the local file and re-run `update-routing-sets`.

The `dns sites` category is local-only by default — it relies entirely on your `local-dns-sites.txt`.

## update-routing-sets produces no output for several minutes

The `resolve` categories run `nslookup` on every hostname in the tracker list sequentially — this can take several minutes for 600+ hostnames. Output only appears once all lookups complete.

Check if it's still running:

```sh
ps | grep nslookup
```

## Check logs

Each category writes its own log to `/tmp/`:

```sh
cat /tmp/dns-torrentsites.log
cat /tmp/dns-pornsites.log
cat /tmp/dns-sites.log
cat /tmp/resolve-torrenttrackers.log
cat /tmp/resolve-sites.log
```

The cron job also writes a combined log:

```sh
cat /tmp/routing-sets.log
```

## Check set contents

```sh
nft list set inet fw4 dns_torrentsites4
nft list set inet fw4 dns_pornsites4
nft list set inet fw4 resolve_torrenttrackers4
```
