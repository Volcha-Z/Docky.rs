#!/bin/sh
dir="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
pkill -x dockyrs
pkill -x dockyrs-notifyd
sleep 0.3
setsid "$dir/target/release/dockyrs-notifyd" >/tmp/dockyrs-notifyd.log 2>&1 < /dev/null &
setsid "$dir/target/release/dockyrs" >/tmp/dockyrs.log 2>&1 < /dev/null &
