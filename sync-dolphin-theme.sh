#!/usr/bin/env bash

set -euo pipefail

scheme="$(python3 -c "import json; print(json.load(open('$HOME/.config/dockyrs/config.json'))['settings'].get('matugen_scheme', 'scheme-tonal-spot'))")"
from_wallpaper="$(python3 -c "import json; print(json.load(open('$HOME/.config/dockyrs/config.json'))['settings'].get('accent_from_wallpaper', True))")"

if [ "$from_wallpaper" = "True" ]; then
    wallpaper="$(python3 -c "import json; print(json.load(open('$HOME/.config/dockyrs/config.json'))['settings'].get('last_wallpaper', ''))")"
    if [ -z "$wallpaper" ] || [ ! -f "$wallpaper" ]; then
        echo "sync-dolphin-theme: no wallpaper set in dockyrs config, nothing to do" >&2
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

mkdir -p "$HOME/.config"

echo "$roles_json" | python3 -c '
import json, sys

c = json.load(sys.stdin)

def rgb(hexstr):
    hexstr = hexstr.lstrip("#")
    return ",".join(str(int(hexstr[i:i+2], 16)) for i in (0, 2, 4))

def section(name, bg, bg_alt, fg, fg_inactive, decoration, hover):
    return f"""[Colors:{name}]
BackgroundAlternate={rgb(bg_alt)}
BackgroundNormal={rgb(bg)}
DecorationFocus={rgb(decoration)}
DecorationHover={rgb(hover)}
ForegroundActive={rgb(decoration)}
ForegroundInactive={rgb(fg_inactive)}
ForegroundLink={rgb(decoration)}
ForegroundNegative={rgb(c["error"])}
ForegroundNeutral={rgb(c["tertiary"])}
ForegroundNormal={rgb(fg)}
ForegroundPositive={rgb(c["tertiary"])}
ForegroundVisited={rgb(hover)}
"""

out = []
out.append(section("View", c["surface"], c["surface_container"], c["on_surface"], c["outline"], c["primary"], c["secondary"]))
out.append(section("Window", c["surface_container"], c["surface_container_high"], c["on_surface"], c["outline"], c["primary"], c["secondary"]))
out.append(section("Button", c["secondary_container"], c["surface_container_high"], c["on_secondary_container"], c["outline"], c["primary"], c["secondary"]))
out.append(section("Selection", c["primary"], c["primary_container"], c["on_primary"], c["on_primary_container"], c["primary"], c["secondary"]))
out.append(section("Tooltip", c["inverse_surface"], c["surface_container_high"], c["inverse_on_surface"], c["outline"], c["primary"], c["secondary"]))
out.append(section("Header", c["surface_container_high"], c["surface_container_highest"], c["on_surface"], c["outline"], c["primary"], c["secondary"]))
out.append(section("Complementary", c["surface_container"], c["surface_container_high"], c["on_surface"], c["outline"], c["primary"], c["secondary"]))

out.append(f"""[General]
Name=Matugen Dock
shadeSortColumn=true

[KDE]
contrast=4

[WM]
activeBackground={rgb(c["surface_container"])}
activeBlend={rgb(c["primary"])}
activeForeground={rgb(c["on_surface"])}
inactiveBackground={rgb(c["surface"])}
inactiveBlend={rgb(c["outline"])}
inactiveForeground={rgb(c["outline"])}
""")

print("\n".join(out))
' > "$HOME/.config/kdeglobals"

echo "Dolphin/Qt colors synced from $src_desc"
