# gtceu — findings (jar inventory + upstream behaviour)

**Mod:** GregTech CEu Modern, mod version **7.0.2**, the 1.21.1 NeoForge build.
**Jar:** `ports/_shared/licence-cache/gtceu-1.21.1-7.0.2.jar`
(29,257,797 B, **14,109 entries**, sha256 `c09a550523e931342a575193cbaad2e4b643e5de2cefe280cd1de540022c6388`).
**Upstream:** <https://github.com/GregTechCEu/GregTech-Modern>, tag **`v7.0.2-1.21.1`**
(commit `f6200e018582c3e26e8db06c289d9e01c8250ac8`), `gradle.properties` says
`mod_version = 7.0.2`, `mod_license = LGPL-3.0 license`.

## 1. The jar, measured

| Fact | Value |
|---|---|
| entries | 14,109 |
| blockstates | 1,556 |
| item models (`assets/gtceu/models/item/*.json`) | 2,712 |
| JSON recipes under `data/**/recipe*` | **0** — GTCEu builds every recipe in Java (`GTRecipeTypes`, `OreRecipeHandler`, `FuelRecipes`) |
| loot tables | 328, only for a handful of non-machine blocks (no machine, no ore: GT drops by code) |
| block/item names | generated from `material.gtceu.<m>` + `tagprefix.<prefix>` at runtime; the shipped `en_us.json` has **no** `block.gtceu.iron_ore` |

The slice's own entries:

| Want | In the jar |
|---|---|
| `gtceu:lp_steam_solid_boiler` | `assets/gtceu/blockstates/lp_steam_solid_boiler.json` (6 `facing=` variants), `models/block/machine/lp_steam_solid_boiler.json` (loader `gtceu:machine`, multipart on `recipe_logic_status`), `models/item/lp_steam_solid_boiler.json` |
| `gtceu:lv_steam_turbine` | same three, `…/lv_steam_turbine` |
| `gtceu:lv_macerator` | same three, `…/lv_macerator` (also `lp_steam_macerator`, `mv`…`uxv`, **not** carried) |
| `gtceu:iron_ore` | **no** blockstate and **no** model: ore blocks are registered from code and drawn by `models/block/ore.json` (loader `neoforge:composite`, `layer0` = `textures/block/material_sets/<set>/ore.png`, `layer1` = `ore_layer2.png`) |
| `gtceu:crushed_iron_ore` | **no** per-item model: id from `TagPrefix.crushed.idPattern("crushed_%s_ore")`, model from `models/item/material_sets/dull/crushed.json` (`layer0` crushed, `layer1` crushed_secondary, `layer2` crushed_overlay) |
| `gtceu:iron_dust` | `TagPrefix.dust` (`%s_dust`), model `models/item/material_sets/dull/dust.json` |

Textures needed (carried verbatim, see `CONTENT.md`):
`block/casings/steam/bricked_bronze/{bottom,side,top}.png`,
`block/casings/voltage/lv/{bottom,side,top}.png`,
`block/generators/boiler/coal/overlay_front.png` (+ `.mcmeta`),
`block/generators/steam_turbine/overlay_side.png` (+ `.mcmeta`),
`block/machines/macerator/overlay_{front,top}.png` (+ `.mcmeta`),
`block/material_sets/dull/{ore,ore_layer2}.png`,
`item/material_sets/dull/{crushed,crushed_secondary,crushed_overlay,dust,dust_secondary}.png`.

Lang keys read: `block.gtceu.lp_steam_solid_boiler` = "Low Pressure Steam Solid Boiler",
`block.gtceu.lv_steam_turbine` = "Basic Steam Turbine Generator §r",
`block.gtceu.lv_macerator` = "Basic Macerator §r", `material.gtceu.iron` = "Iron",
`tagprefix.dust` = "%s Dust".

## 2. The three machines, from the source

Every number below is the mod's; the file:line is in the upstream checkout at the tag above.

### `lp_steam_solid_boiler` — fuel becomes steam

- Registered at `data/machine/GTMachines.java:75-88` (`registerSteamMachines("steam_solid_boiler", …)`,
  recipe type `STEAM_BOILER_RECIPES`, model `block/generators/boiler/coal`).
- Tanks (`api/machine/steam/SteamBoilerMachine.java:97-103`):
  **steam out = 16 × 1000 = 16,000 mB**, **water in = 16,000 mB**.
- Base steam output at max temperature (`config/ConfigHolder.java:577`): **120 mB/s** for the
  low-pressure solid boiler (300 for the HP one, `:582`).
- Max temperature (`SteamBoilerMachine.java:245-247`): **500** LP.
- Heat-up (`:172-190`): while working, **+1 °C every 24 ticks** at LP (`:174-182`); idle, it cools
  **−1 °C every 45 ticks** (`:236-238`, rate `:240-243`).
- Steam (`:192-206`), every **10 ticks**, only when `temperature >= 100` (`:193`):
  `fill = base * (temperature / maxTemperature) / 2` (`:194`) — at 500 °C that is 60 mB per 10
  ticks = **6 mB/tick**; it drains **1 mB of water** per fill (`:195`), and the dry flag
  (`hasNoWater`) explodes the boiler at 2.0 when water *returns* after a dry spell (`:202-206`).
  A full steam tank vents 4 buckets (`:227`).
- Fuel (`api/machine/trait/customlogic/SteamBoilerLogic.java:86-96` +
  `common/machine/trait/customlogic/SmallBoilerLogic.java:19-21`): the fuel is any item with a
  **smelting burn time** (`input.getBurnTime(RecipeType.SMELTING)`), and the small boiler's recipe
  duration is **burnTime × 12** (a coal's 1600 ticks becomes **19,200 ticks** of burning).
- Auto-output (`SteamBoilerMachine.java:139-157`): steam is pushed to **adjacent fluid handlers**
  every **5 ticks**, excluding the front face and the down face.

### `lv_steam_turbine` — steam becomes EU

- `GTMachines.java:286-288`: `registerSimpleGenerator("steam_turbine", STEAM_TURBINE_FUELS,
  steamGeneratorTankSizeFunction, 0.0f, LV, MV, HV)` — the **LV** member is the slice's block.
- Steam tank at LV (`data/machine/GTMachineUtils.java:116-117`): `min(16 × 2^0, 64)` = **16,000 mB**.
- Energy buffer (`api/machine/WorkableTieredMachine.java:88-99`): `V[tier] × 64` = 32 × 64 =
  **2,048 EU** at LV; one amp; `V[LV] = 32` (`api/GTValues.java:45-46`).
- Fuel recipe (`data/recipe/misc/FuelRecipes.java:154-159`):
  **640 mB steam → 4 mB distilled water, duration 10 ticks, EUt −32** (a generator's negative EUt
  is its output); `SimpleGeneratorMachine.java:66-84` parallelises up to
  `overclockVoltage / EUt = 32 / 32 = 1`, so one operation is **320 EU from 640 mB steam**
  (0.5 EU per mB), 32 EU/t for 10 ticks.

### `lv_macerator` — EU macerates an ore

- `GTMachines.java:231-247`: `registerTieredMachines("macerator", … SimpleTieredMachine …)`,
  recipe type `MACERATOR_RECIPES`, **one output slot at LV** (`addOutputLimit(ItemRecipeCapability.CAP, 1)`
  for `case 1, 2`), `defaultTankSizeFunction` = 8,000 mB (unused by the ore recipe).
- Energy buffer: the same `V[LV] × 64` = **2,048 EU**.
- The recipe (`data/recipe/generated/OreRecipeHandler.java:118-135`), for the stone ore prefix
  (ore multiplier 1, `api/material/material/properties/OreProperty.java:30-40`):
  **1 ore block → 2 × crushed ore** (`:122`, `1 × 2 × 1`), **EUt 2** (`:124`), **duration 400**
  (`:126`). The chanced 14 % byproduct at `:123` is empty for iron: no material declares
  `setOreByProducts` in 7.0.2 (`OreProperty.java:156-165` returns `GTMaterials.NULL`), and the
  stone prefix has no secondary material — so the recipe is exactly 1 ore → 2 crushed.
- Recipe defaults when unspecified (`data/recipe/GTRecipeTypes.java:137-141`): duration 150,
  EUt 2.
- Progress (`api/machine/trait/RecipeLogic.java:188-205, 243-258, 429-455`): `progress++` per tick
  while the machine can pay the recipe's EUt; outputs are produced at `progress >= duration`.

### The chain, computed

One coal (1600 vanilla burn ticks × 12 = 19,200 burning ticks) heats the boiler 0 → 500 °C in
12,000 ticks and makes 6 mB/tick at the top. 640 mB of steam is 320 EU; the macerator's ore
operation is 800 EU (2 EU/t × 400 ticks), so the ore needs 1,600 mB of steam ≈ 267 ticks at
500 °C, plus the 400 ticks of grinding. The first steam appears at 100 °C, i.e. after 2,400 ticks
of burning.

## 3. What the javadoc/GUI says, and what the slice does not carry

- Both machines' GUIs (`createUI`, `SteamBoilerMachine.java:306-327`) and the boiler's
  `overlay_front_active` are **not** carried as a screen: this slice has no menu and no screen
  (a gap), and the model keeps the idle overlay.
- Ores: 1,556 blockstates exist; the slice carries **one** (`gtceu:iron_ore`).
- Pipes/cables: the jar has **no** cable or pipe blockstate in the slice's reach — the turbine and
  the macerator are connected by **direct adjacency** (the boiler pushes steam, the turbine pushes
  EU), which is how GT behaves with a machine against a machine.

## 4. Files read upstream

- `api/machine/steam/SteamBoilerMachine.java`, `api/machine/steam/SteamWorkableMachine.java`
- `common/machine/steam/SteamSolidBoilerMachine.java`
- `api/machine/trait/customlogic/SteamBoilerLogic.java`,
  `common/machine/trait/customlogic/SmallBoilerLogic.java`,
  `common/machine/trait/customlogic/MaceratorLogic.java`
- `api/machine/SimpleGeneratorMachine.java`, `api/machine/SimpleTieredMachine.java`,
  `api/machine/WorkableTieredMachine.java`, `api/machine/TieredEnergyMachine.java`,
  `api/machine/trait/RecipeLogic.java`
- `data/machine/GTMachines.java`, `data/machine/GTMachineUtils.java`
- `data/recipe/GTRecipeTypes.java`, `data/recipe/misc/FuelRecipes.java`,
  `data/recipe/generated/OreRecipeHandler.java`
- `api/tag/TagPrefix.java`, `api/material/material/properties/OreProperty.java`,
  `data/material/GTMaterials.java`, `data/material/ElementMaterials.java`, `api/GTValues.java`
- `config/ConfigHolder.java`, `api/machine/feature/IExplosionMachine.java`

The clone was deleted after these numbers were recorded (`ports/_shared/upstream/gtceu`); the jar
stays as licence and provenance evidence.
