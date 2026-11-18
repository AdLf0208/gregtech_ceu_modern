# gtceu — notes

**Class:** `content` · **Tier:** `deep` (machines with a fluid tank, an energy buffer and a
progress counter) · **Role:** core · **Port:** **external** (GregTech CEu Modern is not in
ATM10).
**Approach from `horizon/docs/modpacks.md`:** `deep` — each machine is a block entity with
its own tanks, buffer and operation. This slice is one **fuel → steam → EU → ore**
chain with the mod's own numbers; the rest of the mod is the gap list below.
**Pin (SDK format, 2026-11-18):** Horizon nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4`
(API `0.2`, launcher ≥ 0.2.70) + horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`. The previous slice was built at
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` + mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`
(`.hnmod`); the migration is recorded below and supersedes it.

## Licence

- **LGPL-3.0** (the jar's `META-INF/neoforge.mods.toml` says
  `license = "LGPL-3.0 license"`; GitHub reports `LGPL-3.0`). **Rank 1 (copyleft)** —
  publishable under the *same* licence with the complete corresponding source, which is
  this repository.
- The four obligations, all met and stated in [`LICENSE-NOTICE.md`](LICENSE-NOTICE.md):
  1. ship **both texts** — [`LICENSE`](LICENSE) (LGPL-3.0, 165 lines) and
     [`LICENSE-GPL-3.0`](LICENSE-GPL-3.0) (GPL-3.0, 674 lines) — and both travel **inside
     the archive** at its root, beside `LICENSE-NOTICE.md` (entry list in
     `ports/_shared/evidence/gtceu/sdk-package.txt`);
  2. a [`LICENSE-NOTICE.md`](LICENSE-NOTICE.md) naming the jar's authors (**KilaBash,
     screret, Mikerooni, serenibyss, Tech22, YoungOnion, Ghostipedia**) and its credits,
     the upstream URL and tag, the exact jar and its SHA-256, and that this port is a
     **derivative under the same licence with its complete source published** (it also
     travels inside the archive);
  3. state the modifications — this is a **rewrite on Horizon**, not a copy: the
     behaviour is re-implemented from the mod's own source read as documentation, the
     block models are rewritten for 26.3, the textures/lang/rotations are the jar's own;
  4. add **no further restriction** (LGPL-3.0 §3/§7): no relicensing, no extra terms.
  The SDK package carries the complete source of the port in the archive itself
  (`src/runtime.rs`, `src/lib.rs`, `Cargo.toml`, `horizon-mod.toml`, `assets/` and
  `data/`); `tools/` and `notes.md` stay in the repository.
- The jar stays at `ports/_shared/licence-cache/gtceu-1.21.1-7.0.2.jar` as the licence
  and provenance evidence; the upstream clone was deleted after `FINDINGS.md` was written.

## Slice boundary (what is in, and what is deliberately out)

**Why this slice.** A port is judged by a behaviour a player can perform, and GregTech's
smallest complete loop is its opening one: a fuel makes steam, steam makes EU, EU makes a
machine work. Boiler → turbine → macerator is exactly one of each kind of machine GT has
(steam producer, steam consumer, EU consumer), it needs no cable or pipe (the three blocks
touch), and every number is small enough to state and check: 12 mB of steam at 100 °C,
320 EU from 640 mB, 800 EU for an ore. It also proves the three shapes a later slice
reuses — a tank, an energy buffer, a progress counter — and both directions of the
energy/fluid hand-off.

**In** — one complete power chain, each number the mod's own:

| Block | What it does here |
|---|---|
| `gtceu:lp_steam_solid_boiler` | one fuel item with a smelting burn time burns for **burnTime × 12** ticks; while burning it heats **+1 °C every 24 ticks** to **500**; from **100 °C** it makes `120 × temp / 500 / 2` mB every **10 ticks** (6 mB/t at the top), draining **1 mB of water** a step; it cools **−1 °C every 45 ticks** when idle; a full steam tank vents 4 buckets; a **dry** boiler that gets water back **explodes** (radius 2, block removed). Tanks: water 16,000 mB in, steam 16,000 mB out. Steam is pushed into adjacent fluid handlers on every side but the front and the down one. |
| `gtceu:lv_steam_turbine` | **640 mB of steam → 4 mB of distilled water, 10 ticks, 32 EU/t** (320 EU an operation); a 16,000 mB tank and a 2,048 EU buffer; a full buffer holds the operation; it pushes **one amp (32 EU)** into an adjacent machine. |
| `gtceu:lv_macerator` | one iron ore → **two crushed iron ore**, **2 EU/t for 400 ticks** (800 EU), a 2,048 EU buffer, one input and one output slot. |
| `gtceu:iron_ore` | the one ore the slice carries; the recipe table also accepts vanilla `minecraft:iron_ore` (both are in `c:ores/iron` upstream). |
| `gtceu:crushed_iron_ore` | the output item. |

The three blocks carry their jar blockstates' rotations, their jar textures (idle and
active overlays), GT's own names, loot tables and the `c:ores/iron` tag. The connections
are **direct adjacency** (a machine against a machine): the boiler pushes steam into the
turbine, the turbine pushes EU into the macerator — which is how GT behaves with no
conduit, and this jar has no cable blockstate in the slice's reach.

**Out (gaps, stated as gaps)**

- **Everything else GT has**: 1,555 of the 1,556 blockstates, the other 2,700 items, the
  multiblocks (large boilers, the large turbine, EBF, distillation tower…), cables and
  pipes, covers, tools, circuits, the material system (dusts, ingots, plates, fluids),
  worldgen and ore veins, the recycler/rotor-decomposition recipes (`MaceratorLogic`),
  the steam machines (`lp_steam_macerator`, extractor, furnace…), the LV/MV/HV tiers above
  the slice, the battery buffers, transformers.
- **GUIs and screens** for all three machines: there is no `createUI` half here, so
  inserting is a right click and reading is `/gtceu status` (or the action bar). The
  machines keep their GT slot/tank shape so a later slice can add the screen.
- **The boiler's ash output**: `SteamSolidBoilerMachine.afterWorking` drops ash/dark ash
  30–50 % of the time; there is no ash item in the slice, so the fuel slot is the only one.
- **Redstone, muffling, wrenches, the config UI, explosion damage from weather**, the
  boiler's `recipe_logic_status` for the *turbine's* steam-liquid variant, the boiler's
  `hasNoWater` state for anything but the explosion, and the machine sounds.
- **Emissive overlays** and **material tint**: 26.3 content has no `forge_data` block
  light and no block tint source, so the models draw the plain overlay (see
  [`CONTENT.md`](CONTENT.md)).
- **The active/idle model switch works** (`recipe_logic_status`), and the e2e reads it
  back from `ctl block` — but nothing draws it in a headless run, so the *look* is
  unverified (the state is not).

## Deviations (deliberate, not defects)

1. **The input is consumed when the operation finishes, not when it starts.** GT's
   `RecipeLogic.setupRecipe` consumes the input at the start, so breaking a machine
   mid-recipe loses it. The port keeps the ore in the slot until the two crushed ore are
   made: a player sees the same result and cannot lose the input. Recorded here, not as a
   defect.
2. **The steam/water transfer is the pin's own fluid-handler move** (`native_move_fluids`,
   `native_move_energy`) instead of GT's `NotifiableFluidTank.exportToNearby`/energy
   network. GT pushes to adjacent handlers too; the difference is that only blocks with a
   Horizon fluid/energy table are seen, i.e. this port's machines and any other port that
   declares one — a GT pipe or cable is a gap, not a silent no-op.
3. **A boiler with no water at all** never explodes here (upstream only explodes when
   water *returns*; that path is implemented and proven in the e2e). The weather/terrain
   explosion (`checkWeatherOrTerrainExplosion`) is not carried.
4. **The turbine's output tank** (distilled water) is a real 16,000 mB tank that nothing
   drains in this slice; upstream's would go into a pipe.
5. **`/gtceu fill <x> <y> <z> <mb>`** is a **harness surface**: GT's solid boiler takes
   no bucket (only a fluid handler against it), so the e2e fills the water tank the way a
   pipe would. It is not a gameplay mechanic and is documented as a command.
6. **`/gtceu take` gives the output with `PlayerRef::give_stack`** (the `/give` path):
   what does not fit is dropped at the player's feet instead of staying in the output
   slot, as the retired `native_give_items` fallback did. The player still receives the
   output; the difference is where an over-full inventory puts the excess.

## Upstream defects fixed

None found. The one inconsistency worth noting is not a defect: `OreRecipeHandler` calls
`chancedOutput(byproductStack, 1400, 0)` for every ore, but no material in 7.0.2 sets
`setOreByProducts`, so `getOreByProduct` answers `GTMaterials.NULL` and the chanced output
is empty — the port therefore has **no** 14 % byproduct, which is what the artefact
actually does. `gtceu:iron_ore` → 2 crushed is the whole recipe (two files cited in
`FINDINGS.md`).

## The hooks the retired bundle used (historical)

The table and helper list below describe the **retired `.hnmod`** at `1d63e0a`; they are
kept to read the old evidence. The flat SDK package's helper→hook mapping is the next
section's table.

| Hook | Version | What it did here |
|---|---|---|
| `block_entity` | 1 | a block entity for the three machines |
| `block_entity_ticks` | 1 | all three tick on the server |
| `block_entity_container` | 1 | the boiler's fuel slot, the macerator's input and output |
| `block_entity_fluids` | 1 | the boiler's water + steam tanks, the turbine's steam + distilled water (API 54) |
| `block_entity_energy` | 1 | the turbine's emitter and the macerator's receiver (API 142, `V[LV] × 64`) |
| `block_entity_tick` | 1/17 | one server tick of a machine |
| `level_tick_post` | 1 | the boiler→turbine steam and turbine→macerator energy pushes, where both blocks are in the world |
| `place_block` | 1/14 | a placed machine faces the player, as `setPlacedBy` sets it, and its `recipe_logic_status` starts `idle` |
| `right_click_block` | 1/21 | insert fuel or ore, or read the machine |
| `register_commands` / `perform_command` | 1/8 | `/gtceu status\|take\|fill` |
| `content_root` (raw target) | — | the embedded `.hmod` is written out and appended to the dev list (AE2 pattern) |

Helpers used: `native_move_fluids` (API 54), `native_move_energy` and the energy buffer
(API 142), `native_fluid_insert` (API 54), `native_explode` (API 66), `native_set_block`
(API 24) and `native_set_block_property` (API 18) for `recipe_logic_status` and `facing`,
`native_block`, `native_block_property`, `native_give_items`, `native_take_items`,
`native_feedback`/`native_action_bar`, `res.providers().burn_time` (the smelting fuel
table).

## Upstream files read

Repository `GregTechCEu/GregTech-Modern`, tag `v7.0.2-1.21.1`
(commit `f6200e018582c3e26e8db06c289d9e01c8250ac8`): `api/machine/steam/SteamBoilerMachine`,
`api/machine/steam/SteamWorkableMachine`,
`common/machine/steam/SteamSolidBoilerMachine`,
`api/machine/trait/customlogic/SteamBoilerLogic`,
`common/machine/trait/customlogic/SmallBoilerLogic`,
`common/machine/trait/customlogic/MaceratorLogic`, `api/machine/SimpleGeneratorMachine`,
`api/machine/SimpleTieredMachine`, `api/machine/WorkableTieredMachine`,
`api/machine/TieredEnergyMachine`, `api/machine/trait/RecipeLogic`,
`data/machine/GTMachines`, `data/machine/GTMachineUtils`, `data/recipe/GTRecipeTypes`,
`data/recipe/misc/FuelRecipes`, `data/recipe/generated/OreRecipeHandler`,
`api/tag/TagPrefix`, `api/material/material/properties/OreProperty`,
`data/material/GTMaterials`, `data/material/ElementMaterials`, `api/GTValues`,
`config/ConfigHolder`, `api/machine/feature/IExplosionMachine`. The numbers with their
`file:line` are in [`FINDINGS.md`](FINDINGS.md); the clone was deleted afterwards.

## Migration to the SDK format (2026-11-18)

The retired `.hnmod` container (`horizon-mixin.toml`, `mixins/gtceu/*.rs`,
`horizon-native.json`, the plan/injection schema and the embedded content half) is
replaced by **one flat `.hmod`** — `horizon-mod.toml` (`api = "0.2"`) + `Cargo.toml` +
`src/lib.rs` (one `impl mod_api::Hooks`) + `src/runtime.rs` + `assets/` + `data/` — built
by the launcher against the SDK nightly `0b14f8a`, mixins `e59da90`, game 26.3.
`runtime.rs` is the old file **byte-identical** (SHA-256
`f277f8f5e6f623873169166ddfd75267cfd5e1534be90fbad24d49aa2354722b`); its 29 tests were
green **before** any glue was written, and are still green.

### File map

| Retired | Flat SDK package |
|---|---|
| `mixins/gtceu/runtime.rs` | `src/runtime.rs` (byte-identical) |
| `mixins/gtceu/support.rs` | `src/lib.rs` (the game glue; no separate support module) |
| `mixins/gtceu/<hook>.rs` + `horizon-mixin.toml` `[[injection]]` | methods of the one `impl Hooks` / `impl BlockPart` / `impl BlockEntityPart` |
| `horizon-native.json` (`game`/`horizon`/`mixins`, schema 1) | `horizon-mod.toml` `[native]` (`api = "0.2"`, exact 40-hex commits) |
| `mixins/gtceu/data/gtceu-content.hmod` + `embedded_content.rs` (`content_root`) | `assets/gtceu/` and `data/` at the package root; **no embedding, no `content_root`** |
| `tools/package.py` / `tools/package.sh` | `cargo horizon-mod build` (the package lands in `target/horizon-mod/`) |
| `compatibility.json`, `rust-toolchain.toml` | dropped: `[native]` is the pin, the launcher selects the toolchain (`1.99.1`) |
| `gtceu.hnmod` (62,890 B) | `gtceu.hmod` (84,804 B, 67 entries, sha256 `6c0b06f3…9778`) |

### Helper → hook / host table

Every retired helper mapped to what the SDK calls now. `kept` means the behaviour is
carried; `gap` means the pin's SDK surface cannot express it as the old bundle did.

| Old helper | Old hook | New place | Verdict |
|---|---|---|---|
| `on_block_entity` | `block_entity` v1 | `register`: `r.block_entity_type(<name>).blocks(&[<block>]).build()` + `r.block_entity_part(&kind, MachinePart)`; the declared type starts with the empty compound (the old helper's `Compound::new()`) | kept |
| `on_block_entity_ticks` | `block_entity_ticks` v1 | `BlockEntityPart::block_entity_ticks` sets `event.ticks = true` | kept |
| `on_block_entity_container` | `block_entity_container` v1 | `r.block_container(&block_key, ModContainer {…})` — boiler 1 fuel slot, macerator input 0 / output 1 | kept |
| `on_block_entity_fluids` | `block_entity_fluids` v1 | `r.block_fluids(&block_key, ModFluidTanks {…})` — same two tanks, same `fluids` NBT list, `IO.IN`/`IO.OUT` as `inputs`/`outputs` | kept |
| `on_block_entity_energy` | `block_entity_energy` v1/142 | `r.block_energy(&block_key, ModEnergyBuffer {…})` — capacity 2,048, one 32 EU amp, same `energy` NBT key | kept |
| `on_tick` | `block_entity_tick` v1/17 | `BlockEntityPart::block_entity_tick(host, event)`; reads `event.entity.nbt`/`.items`, calls `runtime`, writes back | kept |
| `on_level_tick_post` | `level_tick_post` v1 | `Hooks::level_tick_post(host, event)`; the queue keys on `level.dimension_id()` and moves steam/EU by `merge_data` | kept, transfer rewritten (see gap 2) |
| `on_place_block` | `place_block` v1/14 | `BlockPart::state_for_placement` from `context.yaw`, `Direction::fromYRot(yaw).getOpposite()` | kept (the vanilla moment, not a post-place patch) |
| `on_right_click_block` | `right_click_block` v1/21 | `Hooks::right_click_block`; `event.click.stack` replaces `game.held()`; `ItemPlace::{BlockEntity, PlayerHand}` in one transaction replaces `native_give/take_items`; `*event.cancellation_result = SuccessServer` + `Flow::Cancel` replaces `canceled = true` | kept |
| `on_register_commands` | `register_commands` v1/8 | `Hooks::register_commands` pushes three typed `Command`s (`<pos:block_pos>`, `<amount:integer>`, `[tag:word]`) | kept (typed parser) |
| `on_perform_command` | `perform_command` v1/8 | `Hooks::run_mod_command` reads `CommandInvocation.literals`/`argument(name)`; `host.commands().feedback` replaces `native_feedback` | kept (typed parser) |
| `embedded_content` | `content_root` (raw target) | none: the package root **is** the content root | dropped by the format |
| `switch_status` | `native_set_block_property` | `host.set_block_property(pos, "recipe_logic_status", …)` | kept |
| `explode` | `native_set_block` + `native_explode` | `level.set_block(air)` + `level.explode(host, None, …, 2.0, ExplosionInteraction::Block, ExplosionSettings::default())` | kept |
| `fuel_burn_time` | `res.providers().burn_time` | `host.item_burn_time(item)` (`cooking_fuel` component) | kept |
| `consume_fuel` remainder | `items().get(...).remainder` | `host.item_crafting_remainder(stack)` | kept |
| `max_stack` | `items().get(...).max_stack` | `host.max_stack_size(item)` | kept |
| `insert`, `take_output` | `native_take_items` / `native_give_items` | `ItemPlace::PlayerHand::extract` / `PlayerRef::give_stack` (one transaction each) | kept |
| `mark_modified` | `world.modified.insert` | the tick's `DerefMut` write moves the entity revision and the engine applies the payload | replaced by design |
| `acting_uuid` / `game.held` | `native_acting_player` / `held` | `event.click.player` (global hook) and `CommandSource::Player` (command) | kept |

Host calls used: `Hooks::register`, `Hooks::register_commands`,
`Hooks::run_mod_command`, `Hooks::right_click_block`, `Hooks::level_tick_post`;
`BlockPart::state_for_placement`; `BlockEntityPart::block_entity_ticks` /
`block_entity_tick`; `host.block_entity`, `BlockEntityRef::{data, merge_data,
item_slot}`, `host.block_state_id`, `host.set_block_property`, `level.{block,
block_property, set_block, explode, capability}`, `host.item_burn_time`,
`host.item_crafting_remainder`, `host.max_stack_size`, `host.item_key`,
`host.commands().feedback`, `PlayerRef::{action_bar, give_stack, is_creative}`,
`ItemPlace::{insert, extract}`, `host.transaction`.

### Manifest mapping

| `horizon-mixin.toml` | `horizon-mod.toml` |
|---|---|
| `[mod] id = "gtceu"`, `version`, `priority = 100` | `id`, `version = "0.2.0"` (`priority` has no SDK equivalent) |
| `[compatibility] horizon_revision` | `[native] horizon` (exact 40-hex) and `mixins` |
| `[[module]] runtime` | `src/runtime.rs` (`pub mod runtime`) |
| `[[module]] gtceu_state` (support) | `src/lib.rs` |
| `[[injection]]` × 12 | the `Hooks`/Part methods above |
| — | `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native] game = "26.3"` |

### Build and check

```
cargo horizon-mod build                  # target/horizon-mod/gtceu.hmod (copy beside the sources)
cargo horizon-mod check gtceu.hmod       # format + manifest + sha256, exit 0
cargo horizon-mod fits gtceu.hmod        # "0b14f8a  made for"
cargo test                               # 29 runtime tests, exit 0
cargo clippy --all-targets -- -D warnings# exit 0, 0 warnings
```

### Resubmission changelog

> Supersedes #927, closed because Horizon retired `.hnmod` (#17992). Changes: flat SDK
> package with `api = "0.2"` and `[native]` pinned to `0b14f8a`; hooks rewritten as
> `impl mod_api::Hooks` methods against typed hosts/events (12 hooks mapped, 0 recorded as
> gaps); `runtime.rs` and its tests unchanged.

The four non-hook gaps the migration records are the missing `0b14f8a` in-game run, the
name-only steam/distilled-water transfer, custom-dimension clicks, and the `in_place`
level derivation (below).

### Traps

- **The packer refuses every retired page in the port tree**: a surviving `mixins/` made
  `cargo horizon-mod build` answer `error: unexpected package path:
  mixins/gtceu/runtime.rs`. Move `mixins/`, `content/` and the `.hnmod` out before the
  first build; the flat package has no place for them.
- **`ModFluidTanks` is `#[non_exhaustive]`**: `Default::default()` + field writes, not a
  struct literal (the other two capability records accept literals).
- **A Part is registered per block entity type**, and `block_entity_tick` is dispatched
  by the **type** id (here the same as the block id, because the type is named after the
  block). `state_for_placement` is dispatched by the block state.
- **`build-slot.sh run` starts in the campaign root** and tries to forward to the remote
  box; for this port path use `BUILD_LOCAL=1` and pass `--manifest-path external/gtceu/Cargo.toml`
  (or `cd external/gtceu` inside the leased command).
- **The build output is not the deliverable path**: `cargo horizon-mod build` writes
  `target/horizon-mod/<id>.hmod`; copy it to the port root.
- **`cargo test`/`check` need `.horizon/sdk/`**: any `cargo horizon-mod` command fetches
  it (749 surface items at `0b14f8a`); it is git-ignored and never packed.
- The manifest's `license`/`homepage` are packed as written; after editing it, rebuild
  and confirm with `unzip -p gtceu.hmod horizon-mod.toml` (a package built before the edit
  does not carry it).
- The two old `native_move_*` helpers have **no SDK method with that name**: the SDK moves
  resources through capability handlers (`FLUIDS`/`ENERGY`, `move_resources`/`move_energy`).
  This port moves its two tanks and its buffer directly (next gap).

### Gaps from the migration

1. **No in-game re-measurement at this pin.** There is no game binary or shared binary
   built at `0b14f8a` and no `horizon-launcher` on this machine, so the old
   `tools/e2e.sh` (which drives a `1d63e0a` `.hnmod`) cannot run. The evidence is the
   byte-identical runtime's 29 tests (the dry-spell explosion and the turbine's EU
   stall are among them), `cargo check/clippy`, `cargo horizon-mod check` and `fits`.
   Re-run the chain in game once a `0b14f8a` build exists; the assertions to repeat are
   the old VALIDATION phases.
2. **Steam and distilled water are name-only fluids.** The content registers blocks and
   items, not fluids, so `gtceu:steam` has no `FluidKey` to hand `move_resources`. The
   boiler→turbine move therefore rewrites the two `fluids` lists directly (the same
   arithmetic as the pure `move_steam`/`move_energy`, which the runtime tests cover),
   instead of going through the engine's `FLUIDS` handler. Water (a vanilla fluid) and
   the tanks themselves are still visible to other mods through the declared
   `block_fluids`. A registered-fluid declaration is the faithful fix.
3. **EU transfer is the same direct write** for the same reason; `ENERGY` is declared and
   other mods can read/drain the buffer, but the port's own push does not call
   `move_energy`.
4. **Custom dimensions are not mapped** by the click path: `LevelDimension::Custom` has
   no stable name to hand `host.level`; right clicks on machines in a runtime level fall
   through. The command path does not have this limit (the invocation carries a
   `DimensionKey`).
5. **The tick derives its level with `LevelRef::in_place`** among `host.levels()`; if a
   tick ever fired while no level is in place, the level-dependent writes (status,
   explosion, push) are skipped while the machine's own state still ticks. Documented,
   not observed.
6. The old `.hnmod` and its `mixins/`/`content/` trees are no longer part of the port
   (they were moved outside the package tree so the flat packer accepts it; the content
   itself is carried in `assets/` and `data/`, and the retired bundle's evidence stays in
   `ports/_shared/evidence/gtceu/e2e*.log` and `VALIDATION.md`).

## Verification

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

- `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` exit 0, **0
  warnings**; `cargo test` **29 passed, 0 failed** (the pure runtime, byte-identical to
  the retired bundle). Logs: `ports/_shared/evidence/gtceu/sdk-{runtime-test,clippy}.log`.
- `cargo horizon-mod build` → `target/horizon-mod/gtceu.hmod`, then copied to
  [`gtceu.hmod`](gtceu.hmod): **84,804 B, 67 entries, SHA-256
  `6c0b06f360d564069a6dbe2696425d06bce604ef4fe7803949487a84fffe9778`**, reproducible
  (two builds, same bytes); `LICENSE`, `LICENSE-GPL-3.0` and `LICENSE-NOTICE.md` are
  inside the archive.
- `cargo horizon-mod check gtceu.hmod` → `gtceu 0.2.0 (native-mod, format
  native-1/0b14f8a…/e59da90…, for 26.3)`, 84,501 bytes, the SHA-256 above, **exit 0**;
  `cargo horizon-mod fits gtceu.hmod` → `0b14f8a  made for`, **exit 0**. Log:
  `ports/_shared/evidence/gtceu/sdk-package-check.log`; entry list and hash in
  `sdk-package.txt`.
- In-game chain: **not re-run at this pin** — no game or shared binary built at
  `0b14f8a` exists here and `horizon-launcher` is not installed; the earlier e2e evidence
  (`e2e.log`, `e2e-selftest.log`, `e2e-embedded.log`) belongs to the retired `.hnmod` at
  `1d63e0a` and is kept as the behaviour contract the migration must preserve.
- The obsolete `.hnmod`/content build commands (`tools/package.py`, `tools/horizon.py`,
  `tools/e2e.sh`, `tools/assemble_content.py`) are retained unedited in `tools/` for the
  record; they describe the retired format and are not the SDK build.
