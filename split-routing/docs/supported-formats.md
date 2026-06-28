# Supported formats

The core updater supports these formats:

```text
auto
domain
hosts
adblock
url
dnsmasq
rpz
unbound
ipset
clash
ip
```

## domain

```text
example.com
another-example.org # comment
```

## hosts

```text
0.0.0.0 example.com
127.0.0.1 ads.example.org
```

## adblock

```text
||example.com^
||ads.example.org^$third-party
```

## url / tracker

```text
udp://tracker.example.org:1337/announce
https://tracker.example.net/announce
```

## dnsmasq

```text
address=/example.com/0.0.0.0
server=/example.org/
ipset=/example.net/some-set
```

## rpz (Response Policy Zone)

BIND/DNS firewall format used by OISD, Hagezi, and Pi-hole Pro feeds.
Extracts the blocked domain name from each record.

```text
$TTL 300
@ SOA rpz.example.com. admin.example.com. ( 2024010101 3600 900 604800 300 )
@ NS ns1.example.com.
example.com CNAME .
*.example.com CNAME .
bad.com A 0.0.0.0
```

Infrastructure records (SOA, NS, MX, TXT, PTR) are skipped automatically.

## unbound

Unbound config format. Extracts the domain from `local-zone` and `local-data` directives.

```text
local-zone: "example.com" always_nxdomain
local-zone: "ads.example.org" static
local-data: "tracker.example.net A 0.0.0.0"
```

## ipset

ipset save/restore format. Extracts IP/CIDR values from `add` lines.

```text
create blocklist hash:ip family inet hashsize 1024 maxelem 65536
add blocklist 1.2.3.4
add blocklist 5.6.7.0/24
add blocklist 2001:db8::/32
```

## clash / surge

Clash, Mihomo, and Surge proxy rule format. Handles `DOMAIN`, `DOMAIN-SUFFIX`,
`DOMAIN-KEYWORD`, `IP-CIDR`, and `IP-CIDR6` rule types. YAML list prefix (`- `)
is stripped automatically.

```text
DOMAIN,example.com
DOMAIN-SUFFIX,ads.example.org
IP-CIDR,1.2.3.0/24
IP-CIDR6,2001:db8::/32
- DOMAIN,tracker.example.net
```

## ip

```text
1.2.3.4
5.6.7.0/24
2001:db8::/32
```
