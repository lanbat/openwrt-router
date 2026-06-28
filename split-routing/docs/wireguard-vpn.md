# Inbound WireGuard VPN

This sets up the router as a WireGuard VPN server so you can connect remotely and be treated as a LAN host — same DNS, same access to LAN devices, and the same Mullvad policy routing for torrent and porn sites.

## Step 1 — Create the WireGuard interface

**Network → Interfaces → Add new interface**

- Name: `vpn`
- Protocol: **WireGuard VPN**
- Click **Create interface**

In the interface settings:

- **Private Key**: click Generate Key (copy the public key shown — you'll need it for clients)
- **Listen Port**: `51821` (51820 is used by the Mullvad client interface)
- **IP Addresses**: `10.0.0.1/24`

**Save** (don't Save & Apply yet)

## Step 2 — Add peers

In the vpn interface, go to the **Peers** tab. For each client device:

- Click **Add peer**
- **Public Key**: paste the client's public key
- **Allowed IPs**: `10.0.0.x/32` — assign a unique IP per client (e.g. `10.0.0.2/32`, `10.0.0.3/32`)
- **Route Allowed IPs**: checked
- **Persistent Keepalive**: `25`

## Step 3 — Put the interface in the LAN firewall zone

**Network → Interfaces → Edit vpn → Firewall Settings tab**

- Zone: **lan**

**Save & Apply**

This makes VPN clients behave identically to wired/WiFi LAN clients, including Mullvad policy routing.

## Step 4 — Open the WireGuard port on WAN

**Network → Firewall → Traffic Rules → Add**

- Name: `Allow-WireGuard-Server`
- Protocol: `UDP`
- Source zone: `wan`
- Destination zone: **Device (input)** — not LAN, the router itself
- Destination port: `51821`
- Action: `Accept`

**Save & Apply**

## Step 5 — Let dnsmasq serve VPN clients

Replace `wg` with whatever you named your WireGuard interface in UCI:

```sh
uci add_list dhcp.@dnsmasq[0].interface='wg'
uci commit dhcp
/etc/init.d/dnsmasq reload
```

## Client config

```ini
[Interface]
PrivateKey = <client_private_key>
Address = 10.0.0.2/24
DNS = 10.0.0.1

[Peer]
PublicKey = <router_public_key>
Endpoint = <your_public_ip>:51821
AllowedIPs = 0.0.0.0/0, ::/0
PersistentKeepalive = 25
```

`AllowedIPs = 0.0.0.0/0, ::/0` routes all client traffic through the router. The Mullvad policy routing for torrent and porn sites applies automatically to VPN clients as a result.

Generate the client keypair on the client device:

```sh
wg genkey | tee privatekey | wg pubkey > publickey
```

Paste the public key into the peer entry in Step 2, and the private key into the client config above.
