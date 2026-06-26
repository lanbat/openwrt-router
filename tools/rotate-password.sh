#!/bin/sh
# Generate a new WiFi password for a network and apply it immediately.
#
# Usage: sh rotate-password.sh <config-file>
#
# Updates the config file in place and reloads wireless. The new password is
# shown as text and (if qrencode is installed) as a QR code.

set -eu

[ $# -eq 1 ] || { echo "Usage: sh rotate-password.sh <config-file>"; exit 1; }

CONFIG="$1"
[ -f "$CONFIG" ] || { echo "ERROR: config not found: $CONFIG"; exit 1; }
. "$CONFIG"

[ -z "${IFACE:-}" ] && { echo "ERROR: IFACE not set"; exit 1; }

WIFI_UCI="${WIFI_UCI:-$IFACE}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

NEW_KEY=$(cat /dev/urandom | tr -dc 'a-zA-Z0-9' | head -c 20)

# Update config file
if grep -q '^WIFI_KEY=' "$CONFIG"; then
    sed -i "s|^WIFI_KEY=.*|WIFI_KEY=$NEW_KEY|" "$CONFIG"
else
    printf 'WIFI_KEY=%s\n' "$NEW_KEY" >>"$CONFIG"
fi

# Update UCI for primary and extra radio sections
uci set wireless."$WIFI_UCI".key="$NEW_KEY"
[ -n "${RADIO_EXTRA:-}" ] && uci -q get wireless."${WIFI_UCI}_extra" >/dev/null 2>&1 \
    && uci set wireless."${WIFI_UCI}_extra".key="$NEW_KEY"
uci commit wireless
wifi reload

printf '\nPassword rotated: %s (%s)\n' "$IFACE" "${SSID:-$IFACE}"
printf 'New password:     %s\n\n' "$NEW_KEY"

# Print QR code using the updated config
WIFI_KEY="$NEW_KEY"
sh "$SCRIPT_DIR/qr.sh" "$CONFIG"
