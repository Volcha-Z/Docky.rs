#!/bin/sh
if ! awww query >/dev/null 2>&1; then
  awww-daemon &
  sleep 0.7
fi
path=$(python3 -c "import json; print(json.load(open('$HOME/.config/dockyrs/config.json'))['settings'].get('last_wallpaper',''))" 2>/dev/null)
if [ -n "$path" ] && [ -f "$path" ]; then
  awww img "$path" --transition-type grow --transition-duration 1.0
fi
