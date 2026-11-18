#!/usr/bin/env python3
"""Turn one `/gtceu status` line into shell assignments the e2e sources.

The port prints `gtceu <kind> x=.. y=.. z=.. key=value ...` (`gtceu.status.*` in
the content's lang). `boiler` carries burn/burn_total, temp/max, water/max,
steam/max and dry; `turbine` steam/max, energy/max, progress/max and water/max;
`macerator` energy/max, progress/max, eu/t, input and output. A field that is a
pair becomes `<NAME>` = the first half and `<NAME>_MAX` = the second; an item field
keeps its `id:count` text in `<NAME>_ITEM` (`eu/t` is `EU_T`). Every name is
upper case, so the e2e reads `$BURN`, `$STEAM`, `$OUTPUT_ITEM`.
"""
import re
import shlex
import sys

PAIRS = {
    "burn": ("burn", "burn_total"),
    "temp": ("temp", "temp_max"),
    "water": ("water", "water_max"),
    "steam": ("steam", "steam_max"),
    "energy": ("energy", "energy_max"),
    "progress": ("progress", "progress_max"),
}
ITEMS = {"input", "output"}


def main():
    if len(sys.argv) != 2:
        print("usage: parse_status.py '<status line>'", file=sys.stderr)
        return 2
    line = sys.argv[1].strip()
    words = line.split()
    if not words or words[0] != "gtceu" or len(words) < 2:
        print("ok=0")
        return 0
    out = {"ok": "1", "kind": words[1]}
    for word in words[2:]:
        if "=" not in word:
            continue
        key, _, value = word.partition("=")
        if key in PAIRS:
            first, second = PAIRS[key]
            left, _, right = value.partition("/")
            out[first] = left
            out[second] = right
        elif key in ITEMS:
            out[f"{key}_item"] = value
        else:
            out[key] = value
    for key, value in out.items():
        # A field name becomes a shell name: `eu/t` is `EU_T`.
        name = re.sub(r"[^A-Za-z0-9_]", "_", key).upper()
        print(f"{name}={shlex.quote(value)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
