#!/usr/bin/env bash
set -euo pipefail

scheme="$(python3 -c "import json; print(json.load(open('$HOME/.config/dockyrs/config.json'))['settings'].get('matugen_scheme', 'scheme-tonal-spot'))")"
from_wallpaper="$(python3 -c "import json; print(json.load(open('$HOME/.config/dockyrs/config.json'))['settings'].get('accent_from_wallpaper', True))")"

if [ "$from_wallpaper" = "True" ]; then
    wallpaper="$(python3 -c "import json; print(json.load(open('$HOME/.config/dockyrs/config.json'))['settings'].get('last_wallpaper', ''))")"
    if [ -z "$wallpaper" ] || [ ! -f "$wallpaper" ]; then
        echo "sync-p10k-theme: no wallpaper set in dockyrs config, nothing to do" >&2
        exit 1
    fi
    roles_json="$(matugen --type "$scheme" image "$wallpaper" --source-color-index 0 --json hex --mode dark 2>/dev/null | python3 -c "
import json, sys
d = json.load(sys.stdin)
print(json.dumps({k: v['dark']['color'] for k, v in d['colors'].items()}))
")"
    src_desc="$wallpaper"
else
    roles_json="$(python3 -c "
import json

def clamp(v): return max(0, min(255, round(v)))
def to_hex(rgb): return '#%02x%02x%02x' % tuple(clamp(v) for v in rgb)
def mix(a, b, t): return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))

s = json.load(open('$HOME/.config/dockyrs/config.json'))['settings']
panel = (s['panel_r'], s['panel_g'], s['panel_b'])
text = (s['text_r'], s['text_g'], s['text_b'])
text_dim = (s['text_dim_r'], s['text_dim_g'], s['text_dim_b'])
accent = (s['accent_r'], s['accent_g'], s['accent_b'])
accent2 = (s['accent2_r'], s['accent2_g'], s['accent2_b'])
on_accent = (s['on_accent_r'], s['on_accent_g'], s['on_accent_b'])

c = {
    'surface': to_hex(panel),
    'surface_container': to_hex(mix(panel, (255, 255, 255), 0.06)),
    'surface_container_high': to_hex(mix(panel, (255, 255, 255), 0.12)),
    'surface_container_highest': to_hex(mix(panel, (255, 255, 255), 0.18)),
    'on_surface': to_hex(text),
    'outline': to_hex(text_dim),
    'outline_variant': to_hex(mix(text_dim, panel, 0.4)),
    'primary': to_hex(accent),
    'on_primary': to_hex(on_accent),
    'primary_container': to_hex(mix(panel, accent, 0.3)),
    'on_primary_container': to_hex(text),
    'secondary': to_hex(accent2),
    'on_secondary': to_hex(on_accent),
    'secondary_container': to_hex(mix(panel, accent2, 0.3)),
    'on_secondary_container': to_hex(text),
    'tertiary': to_hex(accent2),
    'error': '#f38ba8',
    'inverse_surface': to_hex(text),
    'inverse_on_surface': to_hex(panel),
}
print(json.dumps(c))
")"
    src_desc="panel"
fi

out="$HOME/.p10k-colors.zsh"

py="$(mktemp)"
trap 'rm -f "$py"' EXIT
cat > "$py" <<'PYEOF'
import json, sys, colorsys

c = json.load(sys.stdin)

def to_rgb(hexstr):
    hexstr = hexstr.lstrip("#")
    return tuple(int(hexstr[i:i+2], 16) / 255 for i in (0, 2, 4))

def to_hex(rgb):
    return "#%02x%02x%02x" % tuple(max(0, min(255, round(v * 255))) for v in rgb)

sats, lits = [], []
for role in ("primary", "secondary", "tertiary"):
    r, g, b = to_rgb(c[role])
    h, l, s = colorsys.rgb_to_hls(r, g, b)
    sats.append(s)
    lits.append(l)
sat = sum(sats) / len(sats)
lit = sum(lits) / len(lits)

def hue(deg, bright=False):
    l = lit + (1.0 - lit) * 0.4 if bright else lit * 0.72
    return to_hex(colorsys.hls_to_rgb(deg / 360, min(1.0, l), sat))

green = hue(120, bright=True)
yellow = hue(50, bright=True)
red = hue(0, bright=True)
cyan = hue(180, bright=True)

print("typeset -g POWERLEVEL9K_OS_ICON_BACKGROUND=\"" + c["primary"] + "\"")
print("typeset -g POWERLEVEL9K_OS_ICON_FOREGROUND=\"" + c["on_primary"] + "\"")

print("typeset -g POWERLEVEL9K_DIR_BACKGROUND=\"" + c["secondary"] + "\"")
print("typeset -g POWERLEVEL9K_DIR_FOREGROUND=\"" + c["on_secondary"] + "\"")

print("typeset -g POWERLEVEL9K_VCS_CLEAN_BACKGROUND=\"" + green + "\"")
print("typeset -g POWERLEVEL9K_VCS_UNTRACKED_BACKGROUND=\"" + green + "\"")
print("typeset -g POWERLEVEL9K_VCS_MODIFIED_BACKGROUND=\"" + yellow + "\"")
print("typeset -g POWERLEVEL9K_VCS_CONFLICTED_BACKGROUND=\"" + yellow + "\"")
print("typeset -g POWERLEVEL9K_VCS_FOREGROUND=\"" + c["surface"] + "\"")

print("typeset -g POWERLEVEL9K_STATUS_OK_FOREGROUND=\"" + green + "\"")
print("typeset -g POWERLEVEL9K_STATUS_OK_BACKGROUND=\"" + c["surface_container"] + "\"")
print("typeset -g POWERLEVEL9K_STATUS_OK_PIPE_FOREGROUND=\"" + green + "\"")
print("typeset -g POWERLEVEL9K_STATUS_OK_PIPE_BACKGROUND=\"" + c["surface_container"] + "\"")
print("typeset -g POWERLEVEL9K_STATUS_ERROR_FOREGROUND=\"" + c["surface"] + "\"")
print("typeset -g POWERLEVEL9K_STATUS_ERROR_BACKGROUND=\"" + red + "\"")
print("typeset -g POWERLEVEL9K_STATUS_ERROR_SIGNAL_FOREGROUND=\"" + c["surface"] + "\"")
print("typeset -g POWERLEVEL9K_STATUS_ERROR_SIGNAL_BACKGROUND=\"" + red + "\"")
print("typeset -g POWERLEVEL9K_STATUS_ERROR_PIPE_FOREGROUND=\"" + c["surface"] + "\"")
print("typeset -g POWERLEVEL9K_STATUS_ERROR_PIPE_BACKGROUND=\"" + red + "\"")

print("typeset -g POWERLEVEL9K_COMMAND_EXECUTION_TIME_FOREGROUND=\"" + c["surface"] + "\"")
print("typeset -g POWERLEVEL9K_COMMAND_EXECUTION_TIME_BACKGROUND=\"" + yellow + "\"")

print("typeset -g POWERLEVEL9K_BACKGROUND_JOBS_FOREGROUND=\"" + cyan + "\"")
print("typeset -g POWERLEVEL9K_BACKGROUND_JOBS_BACKGROUND=\"" + c["surface_container"] + "\"")

print("typeset -g POWERLEVEL9K_CONTEXT_FOREGROUND=\"" + yellow + "\"")
print("typeset -g POWERLEVEL9K_CONTEXT_BACKGROUND=\"" + c["surface_container"] + "\"")
print("typeset -g POWERLEVEL9K_CONTEXT_ROOT_FOREGROUND=\"" + c["surface"] + "\"")
print("typeset -g POWERLEVEL9K_CONTEXT_ROOT_BACKGROUND=\"" + red + "\"")
PYEOF

{
    echo "# Auto-generated by sync-p10k-theme.sh, do not edit by hand."
    echo "$roles_json" | python3 "$py"
} > "$out"

pkill -USR1 -u "$(id -u)" -x zsh >/dev/null 2>&1 || true

echo "p10k colors synced from $src_desc"
