# Content source — where every carried file came from

The content half of this port (`assets/` and `data/` at the package root, packed into
the flat `gtceu.hmod`) carries files out of one jar and generates the rest:

- **Jar:** `ports/_shared/licence-cache/gtceu-1.21.1-7.0.2.jar`
  (29,257,797 B, **14,109 entries**, SHA-256
  `c09a550523e931342a575193cbaad2e4b643e5de2cefe280cd1de540022c6388`).
- **Upstream:** `GregTechCEu/GregTech-Modern`, tag **`v7.0.2-1.21.1`**
  (commit `f6200e018582c3e26e8db06c289d9e01c8250ac8`), mod version `7.0.2`.
- **Licence of the jar:** LGPL-3.0 (`META-INF/neoforge.mods.toml`,
  `license = "LGPL-3.0 license"`). See [`LICENSE-NOTICE.md`](LICENSE-NOTICE.md).

`tools/assemble_content.py` is the only writer: it copies `CARRY` byte for byte and
writes every JSON (it wrote the retired `content/` folder; the same files are now at the
package root). `python3 tools/assemble_content.py --check` re-reads the jar and
fails if the tree differs from what the jar gives.

## Carried byte for byte (23 files, `CARRY` in `tools/assemble_content.py`)

| Jar path (`assets/gtceu/textures/`) | Why |
|---|---|
| `block/casings/steam/bricked_bronze/{bottom,side,top}.png` | the boiler's casing |
| `block/casings/voltage/lv/{bottom,side,top}.png` | the turbine's and the macerator's LV casing |
| `block/generators/boiler/coal/overlay_front.png` | the boiler's idle front |
| `block/generators/boiler/coal/overlay_front_active.png` + `.mcmeta` | its working front (animated) |
| `block/generators/steam_turbine/overlay_side.png` | the turbine's idle side |
| `block/generators/steam_turbine/overlay_side_active.png` + `.mcmeta` | its working side |
| `block/machines/macerator/overlay_front.png`, `overlay_top.png` | the macerator's idle front and top |
| `block/machines/macerator/overlay_front_active.png` + `.mcmeta`, `overlay_top_active.png` + `.mcmeta` | its working front and top |
| `block/material_sets/dull/ore.png`, `ore_layer2.png` | the iron ore block's two layers |
| `item/material_sets/dull/crushed.png`, `crushed_secondary.png`, `crushed_overlay.png` | crushed iron ore's three layers |

No `en_us.json`, model or blockstate is copied: GT generates those from code, so the
ones below are written by the script (and their wording is GT's own, from
`assets/gtceu/lang/en_us.json`, which was read for the names).

## Written by `tools/assemble_content.py`

| File | Rewritten from | Note |
|---|---|---|
| `assets/gtceu/blockstates/{lp_steam_solid_boiler,lv_steam_turbine,lv_macerator}.json` | the jar's own six variants | the four horizontal `facing` values with the jar's rotations (`east` 90, `south` 180, `west` 270), times `recipe_logic_status=idle|working` |
| `assets/gtceu/models/block/machine/<name>.json`, `<name>_active.json` | the jar's `gtceu:machine` multipart + `block/machine/template/sided/sided` | GT's model **loader** does not exist in a 26.3 content mod, so the geometry is written out: the base cube (`#bottom/#side/#top`) plus the same overlay planes the jar passes, with the idle or the active overlay per status |
| `assets/gtceu/models/block/iron_ore.json` | the jar's `models/block/ore.json` (`neoforge:composite`) | flattened to a stone cube plus the two ore layers it composites |
| `assets/gtceu/models/item/*.json`, `assets/gtceu/items/*.json` | the jar's `models/item/<name>.json` | 26.3 item definitions (`{"model":{"type":"minecraft:model",…}}`) beside the item models |
| `data/gtceu/horizon/block/*.json`, `horizon/item/crushed_iron_ore.json` | — | template `minecraft:stone` (machines: hardness 3, blast resistance 6, iron-block sounds, `facing` + `recipe_logic_status` states) / `minecraft:gunpowder` for the item |
| `data/gtceu/loot_table/blocks/*.json` | — | GT drops machines and ores from code; the slice ships the 26.3 singular typed condition (`"condition": {"type": "minecraft:survives_explosion"}`) |
| `data/minecraft/tags/block/mineable/pickaxe.json` | — | the four blocks |
| `data/c/tags/block/ores.json`, `ores/iron.json` | GT's own tag (`c:ores/iron` under `c:ores`) | GT's block and vanilla's `minecraft:iron_ore` |
| `assets/gtceu/lang/en_us.json` | the jar's names | five block/item names plus this port's status/command strings |

## What is not carried (and why)

- The **GUI screens** (`createUI`): the slice has no menu (a gap), so no GUI textures
  travel.
- The **emissive** overlays (`*_emissive.png`): 26.3 has no `forge_data` block-light
  element; the model is drawn with the overlay and no glow (a deviation).
- The **material tint**: GT tints its casing and item textures through `tintindex`;
  a content block in Horizon cannot name a tint source, so the textures render as they
  ship (a deviation, visible as slightly flatter colours).
- The **item models of the other 2,700 items**, the **1,555 other blockstates**, the
  multiblocks, covers, cables/pipes and tools: gaps (see `../notes.md`).
