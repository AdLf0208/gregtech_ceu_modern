# gtceu — the content half

The mod ships **one flat SDK package** (`gtceu.hmod`, `api = "0.2"`): the code is
`src/`, and the content is `assets/gtceu/` and `data/` at the package root — no embedded
content archive and no `content_root` injection (the retired `.hnmod` used both).

## What the content declares

| Namespace:name | Kind | Template / shape |
|---|---|---|
| `gtceu:lp_steam_solid_boiler` | block + item | `minecraft:stone`, hardness 3, blast resistance 6, iron-block sounds; states `facing` (north/east/south/west) and `recipe_logic_status` (idle/working) |
| `gtceu:lv_steam_turbine` | block + item | the same, with the turbine's own model |
| `gtceu:lv_macerator` | block + item | the same, with the macerator's own model |
| `gtceu:iron_ore` | block + item | `minecraft:stone`, hardness 3, drops itself, in `minecraft:mineable/pickaxe`, `c:ores` and `c:ores/iron` |
| `gtceu:crushed_iron_ore` | item | `minecraft:gunpowder` stack shape, the dull material set's three layers |

Tags: `minecraft:tags/block/mineable/pickaxe` (the four blocks), `c:tags/block/ores` (the
family) and `c:tags/block/ores/iron` (`gtceu:iron_ore` and `minecraft:iron_ore` — what GT's
own `c:ores/iron` holds).

## Files

| Tree | Files |
|---|---|
| `assets/gtceu/blockstates/` | 4 (`iron_ore`, the three machines with 8 variants each) |
| `assets/gtceu/models/block/` | 7 (4 machine models: idle + active for the boiler, turbine and macerator; 1 ore model) |
| `assets/gtceu/models/item/` | 5 |
| `assets/gtceu/items/` | 5 (26.3 item definitions) |
| `assets/gtceu/textures/` | 23 carried from the jar (see [`SOURCE.md`](SOURCE.md)) |
| `assets/gtceu/lang/` | 1 (`en_us.json`) |
| `data/gtceu/horizon/block/` | 4 |
| `data/gtceu/horizon/item/` | 1 |
| `data/gtceu/loot_table/blocks/` | 4 (26.3 `"condition"` shape) |
| `data/minecraft/tags/block/mineable/` | 1 |
| `data/c/tags/block/` | 2 |
| `horizon-mod.toml` | the manifest (`api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pin) |

`tools/assemble_content.py` wrote all of it (it targeted the retired `content/` folder;
the files now live at the package root) and copies the 23 carried files;
`python3 tools/assemble_content.py --check` fails if the tree is not what the jar gives.

## What is not there

No GUI textures (the slice has no screen), no emissive overlays (26.3 has no
`forge_data` block light), no material tint (a content block cannot name a tint source),
and none of the mod's other 1,555 blockstates or 2,700 items. The reasons and the model
rewrites are in [`SOURCE.md`](SOURCE.md); the gap list is in
[`notes.md`](notes.md).
