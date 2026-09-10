#!/usr/bin/env bash

dock="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)/target/release/dockyrs"
state="$HOME/.cache/dockyrs-recording-path"

if pgrep -x wf-recorder >/dev/null; then
    pkill -INT -x wf-recorder
    while pgrep -x wf-recorder >/dev/null; do sleep 0.2; done
    out="$(cat "$state" 2>/dev/null)"
    rm -f "$state"
    "$dock" --notify "Recording saved" "$(basename "${out:-recording.mp4}")"
else
    mkdir -p "$HOME/Videos"
    out="$HOME/Videos/recording-$(date +%Y%m%d-%H%M%S).mp4"
    echo "$out" > "$state"
    sink="$(pactl get-default-sink 2>/dev/null)"
    hwenc=()
    [ -e /dev/dri/renderD128 ] && hwenc=(-c h264_vaapi -d /dev/dri/renderD128)
    if [ -n "$sink" ]; then
        wf-recorder "${hwenc[@]}" -f "$out" --audio="$sink.monitor" &
    else
        wf-recorder "${hwenc[@]}" -f "$out" -a &
    fi
    disown
    "$dock" --notify "Recording started" "$(basename "$out")"
fi
