#!/bin/sh
# Top-level installer — runs each sub-project's install.sh in order.
# Each sub-script is self-contained and can also be run directly.
set -eu

REPO="$(cd "$(dirname "$0")" && pwd)"

[ $# -gt 0 ] && case "$1" in
    extra-networks) sh "$REPO/extra-networks/install.sh"; exit ;;
    split-routing)  sh "$REPO/split-routing/install.sh";  exit ;;
esac

sh "$REPO/extra-networks/install.sh"
sh "$REPO/split-routing/install.sh"
