# GregTech CEu Modern for Horizon — slice 1

A first slice of **GregTech CEu Modern 7.0.2** (the 1.21.1 NeoForge build) on
[Horizon](https://github.com/horizon-rust/horizon), a Rust rewrite of Minecraft: Java
Edition: a coal-fired steam boiler, a steam turbine and an LV macerator, connected by
adjacent faces, with the mod's own numbers.

```
 coal ──► gtceu:lp_steam_solid_boiler ──steam──► gtceu:lv_steam_turbine ──EU──► gtceu:lv_macerator
 (any smelting fuel)      120 mB/s at 500 °C        640 mB → 320 EU             2 EU/t for 400 ticks
                          +1 °C every 24 ticks      32 EU/t for 10 ticks        one iron ore → two crushed
```

| Block | Numbers (all upstream's) |
|---|---|
| `gtceu:lp_steam_solid_boiler` | burns a fuel item for `burnTime × 12` ticks; heats to 500 °C at +1 / 24 ticks; from 100 °C makes `120 × temp / 500 / 2` mB every 10 ticks and drains 1 mB of water a step; cools −1 / 45 ticks; 16,000 mB water and steam tanks; dry-boil explosion |
| `gtceu:lv_steam_turbine` | 640 mB steam → 4 mB distilled water, 10 ticks, 32 EU/t (320 EU); 16,000 mB tank, 2,048 EU buffer |
| `gtceu:lv_macerator` | one `gtceu:iron_ore` → two `gtceu:crushed_iron_ore`, 2 EU/t for 400 ticks (800 EU); 2,048 EU buffer |

One flat SDK package installs everything: **`gtceu.hmod`** (`api = "0.2"`, pinned to
Horizon `0b14f8a` and mixins `e59da90`) carries the code (`src/`), the content
(`assets/gtceu/`, `data/`) and the LGPL-3.0 text, the GPL-3.0 text and this port's
licence notice. There is no `.hnmod` and no embedded content archive: the package root
is the content root.

## What this slice does not have

Most of GregTech. The multiblocks, cables and pipes, the material system, the other
1,555 blockstates and 2,700 items, tools, covers, circuits and worldgen are **gaps**, and
so are the machine GUIs: inserting is a right click, reading is `/gtceu status`. The
machines connect by **direct adjacency**, which is how GT behaves with no conduit. The
full list, with reasons, is in [`notes.md`](notes.md).

## Commands

```
/gtceu status <x> <y> <z> [tag]   one machine's state (fuel, temperature, tanks, EU, progress)
/gtceu take <x> <y> <z>          a macerator's output into the player's inventory
/gtceu fill <x> <y> <z> <mb>     water into a boiler / steam into a turbine (the pipe a
                                 player would run; GT's boiler takes no bucket)
```

## Licence

This port is a derivative of **GregTech CEu Modern** by KilaBash, screret, Mikerooni,
serenibyss, Tech22, YoungOnion and Ghostipedia, distributed under the **LGPL-3.0** and
published under the **same licence** with its complete source in this repository. The
LGPL-3.0 text is [`LICENSE`](LICENSE), the GPL-3.0 text it refers to is
[`LICENSE-GPL-3.0`](LICENSE-GPL-3.0), and [`LICENSE-NOTICE.md`](LICENSE-NOTICE.md)
states the attribution, the modifications and the source. It is not endorsed by the
GregTech CEu developers.

## Building it

```
cargo horizon-mod build           # target/horizon-mod/gtceu.hmod (copy it beside the sources)
cargo horizon-mod check gtceu.hmod
cargo horizon-mod fits gtceu.hmod # "0b14f8a  made for"
cargo test                        # the pure runtime, 29 tests
cargo clippy --all-targets -- -D warnings
```

The retired `.hnmod` tooling (`tools/package.py`, `tools/horizon.py`, `tools/e2e.sh`,
`tools/assemble_content.py`) is kept for the record only. There is no in-game run at the
SDK pin yet: no game binary was built at `0b14f8a` on this machine (see `notes.md`).

Numbers and their `file:line`: [`FINDINGS.md`](FINDINGS.md). Verification:
[`VALIDATION.md`](VALIDATION.md) and `ports/_shared/evidence/gtceu/` (`sdk-*` logs).
