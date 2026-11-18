# gtceu — validation

## SDK format (2026-11-18)

Flat `.hmod` at SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`**, mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, game 26.3. Every build step ran inside a
`ports/tools/build-slot.sh` lease with `CARGO_INCREMENTAL=0` and `BUILD_LOCAL=1` (the
port lives at `external/gtceu`, which the remote sync does not mirror). Logs:
`ports/_shared/evidence/gtceu/sdk-*`.

| Step | Command | Result |
|---|---|---|
| Runtime tests (before glue) | `cargo test` | **29 passed, 0 failed** — the byte-identical `runtime.rs` |
| Runtime tests (after glue) | `cargo test` | **29 passed, 0 failed** (`sdk-runtime-test.log`) |
| Formatting | `cargo fmt --check` | clean |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| Package | `cargo horizon-mod build` | `target/horizon-mod/gtceu.hmod`; two builds, same bytes |
| Deliverable | copied beside the sources | `gtceu.hmod` **84,804 B**, 67 entries, SHA-256 `6c0b06f360d564069a6dbe2696425d06bce604ef4fe7803949487a84fffe9778` (`sdk-package.txt`) |
| Format check | `cargo horizon-mod check gtceu.hmod` | `native-mod, format native-1/0b14f8a…/e59da90…, for 26.3`, exit 0 (`sdk-package-check.log`) |
| Fit | `cargo horizon-mod fits gtceu.hmod` | `0b14f8a  made for`, exit 0 |
| In game | — | **not run**: no game or shared binary built at `0b14f8a` and no `horizon-launcher` on this machine; the e2e below is the retired bundle's evidence |

The runtime tests include the rules the migration had to keep: the dry-spell explosion
(`water_returning_after_a_dry_spell_explodes_the_boiler`), the full-buffer stall
(`the_turbine_stalls_while_its_buffer_cannot_take_a_tick`), the 640 mB / 320 EU operation,
the 800 EU ore and the two adjacency moves. The glue compiles against the exact SDK
(`cargo check --all-targets`); its in-game re-measurement is the open item in `notes.md`.

## Retired `.hnmod` record (2026-11-13)

The sections below are the previous bundle's evidence, kept as the behaviour contract the
migration preserves. Its paths (`mixins/`, `content/`, `gtceu.hnmod`) were moved out of
the package on 2026-11-18.

Pin: Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6). Every command ran inside a
`ports/tools/build-slot.sh` lease; `CARGO_INCREMENTAL=0` everywhere; no bare `cargo` in
a port except through the runner, no `--target-dir` of our own.

## 1. The pure runtime (standalone crate)

```
cd external/gtceu && cargo fmt --check && cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline
```

- `cargo fmt --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: **0 warnings, 0 errors**.
- `cargo test`: **29 passed; 0 failed; 0 ignored** (`mixins/gtceu/runtime.rs`,
  `#[cfg(test)] mod tests`).

The tests were written first and watched fail: the first run had **26 failures** with
`not implemented`, then three assertions were corrected against the real tick arithmetic
(the boiler starts at 1 °C on its first working tick, so 100 °C lands on tick 2,380, not
2,400), and the run went green at 29. Every test names the upstream number it pins
(`FINDINGS.md` has the `file:line`).

## 2. The ladder at the pin

`python3 tools/horizon.py stage|check|test|build` (through the slot runner):

| Step | Result |
|---|---|
| `stage` | ok (`stage_changed: true`, the pinned `git archive` + mixin plan/apply) |
| `check` | `cargo_exit 0`, **0 warnings** (`target/horizon-check.log`) |
| `test` | `cargo_exit 0`, **29 passed**, `game::gtceu::tests`, 1240 filtered out |
| `build` | `cargo_exit 0`, `.../fast/horizon-client` |

## 3. Packages

`tools/package.sh` (in a slot):

| Artifact | Size | Entries | SHA-256 |
|---|---:|---:|---|
| `content/gtceu.hmod` | 33,575 B | 58 | `f0ba59127c06e6ebc83d31b711b0e6baaf7155a544e277f0bb94c129ec3076ad` |
| `gtceu.hnmod` | 62,890 B | 20 | `397f5b1cf8b0342ff1b1b08ed61aa95f5a02c47c1e64a071244505d6bbf86b9b` |

- `cargo horizon-mod build` prints **"no code: content only, nothing to run"** for the
  content half, and `horizon-mod check gtceu.hmod` passes; `horizon-mod check gtceu.hnmod
  --kind native-mod` passes.
- The `.hnmod` is **reproducible**: `python3 tools/package.py` run twice gives the same
  bytes (`cmp` clean).
- The archive carries **`mixins/gtceu/LICENSE`** (LGPL-3.0), **`LICENSE-GPL-3.0`**
  (GPL-3.0) and **`LICENSE-NOTICE.md`**, plus the embedded
  `mixins/gtceu/data/gtceu-content.hmod` (33,575 B, byte-identical to
  `content/gtceu.hmod`).

## 4. The e2e (fail-capable, in game)

`ports/tools/build-slot.sh e2e --job … --port external/gtceu -- tools/e2e.sh`, logs in
`ports/_shared/evidence/gtceu/`. The scenario runs in the End's void at x=200 with
`/tick rate 200` (the tick arithmetic is unchanged; only the wall time is), drives the
real game through `horizon-ctl`, and every mod command goes through `ctl chat` so
`perform_command` hears it.

| Log | Verdict | What it shows |
|---|---|---|
| `e2e.log` | **PASS + EXIT:0** | the whole chain and every control |
| `e2e-selftest.log` | **FAIL + EXIT:1** | the same run with F's assertion flipped (`:3` instead of `:2`) — the scenario really fails when the behaviour is wrong |
| `e2e-embedded.log` | **PASS + EXIT:0** | the same run with **no `--dev-mod`**, content from the bundle, and a control leg first |

All three legs were re-run **after the final `cargo fmt`**, i.e. on exactly the sources
the delivered `.hnmod` carries (`cmp` of every `.rs`/`.toml` in the archive against the
tree: no difference).

Quoted from `e2e.log` (the numbers are the mod's):

```
the boiler is burning: 19090/19200 ticks            # one coal = 1600 × 12
watered boiler: temp 428, water 211 mB, steam 280 mB
dry boiler: temp 409 and steam 0 mB — the fuelled no-water control
turbine: steam 16000 mB, EU 2048, progress 8
macerator: EU 2048/2048, progress 290/400, state gtceu:lv_macerator[facing=west,recipe_logic_status=working]
the macerator holds gtceu:crushed_iron_ore:2 and /gtceu take put them in the inventory
the unpowered macerator: EU 0, progress 0, its ore still in the slot
minecraft:gold_ore is outside the recipe table and was refused
the dry boiler is gone: water after a dry spell explodes it, as upstream does
```

The legs, in the script's own words: **A** the three items place and the boiler's
`facing` comes from the placer; **B** no fuel → nothing burns, nothing is made;
**C** one coal each, water in one boiler only → the watered one heats past 100 °C and
makes steam (12 mB at 100 °C, rising with temperature), the dry one heats but makes
none and reports `dry=1` (the control); **D** the steam reaches the turbine and becomes
EU; **E** the EU reaches the macerator and it starts to grind, its blockstate reading
`recipe_logic_status=working`; **F** one `gtceu:iron_ore` becomes exactly two
`gtceu:crushed_iron_ore` and `/gtceu take` puts them in the inventory; **G** an
unpowered macerator spends nothing, makes no progress, keeps its ore and reads `idle`,
and `minecraft:gold_ore` is refused by the recipe table; **H** water into the dry boiler
explodes it (the block is gone).

The **control for the embedded leg** (inside `e2e-embedded.log`) launches the same binary
with `.horizon-cache` unusable: no macerator item and the block id unknown — so the
success is not independent of the embedded file. The run then checks the written file is
byte-identical to `content/gtceu.hmod`.

## 5. Gates

| Gate | Result |
|---|---|
| `python3 ports/tools/check-e2e-logs.py gtceu --strict` | **exit 0** — `logs=3 ok=3 missing-exit=0 bad-exit=0` (`PASS + EXIT:0`, `FAIL + EXIT:1`, `PASS + EXIT:0`) |
| `python3 ports/tools/check-skills.py --notes external/gtceu` | **PASS**, exit 0 |
| `python3 ports/tools/upstream_ready.py gtceu --copyleft` | **cannot pass yet**: the tool answers `{"blockers":["not in ledger"],"ready":false}` because `gtceu` has no `ports/ledger.csv` row and no `ports/_shared/licence-audit.csv` row — both are the orchestrator's to add (this port may not edit them). Everything else it checks is green: the artifact is in the port root, `notes.md` has the `Skills:` line, an e2e log is `PASS + EXIT:0`, and `compatibility.json` is pinned to `1d63e0a`. The licence for the row, read from the jar: **LGPL-3.0** (`META-INF/neoforge.mods.toml`, `license = "LGPL-3.0 license"`), class `content`, tier `deep`, status `ported-partial`, directory `external/gtceu`. |

## 6. Content

`python3 tools/assemble_content.py --check` — the tree is exactly what the jar gives
(23 carried files byte for byte, 35 generated JSONs). `cargo horizon-mod check` reports
the content mod as registering blocks and items and holding **no code**.

## 7. Disk

`target/` holds only the slot caches' symlinks and the ladder logs; the staged trees
live under `ports/_shared/build-slots/slot-N/`. The delivered `content/gtceu.hmod` and
`gtceu.hnmod` are in the port, not in `target/`. The upstream clone was deleted after
`FINDINGS.md` was written; the jar stays in `ports/_shared/licence-cache/` as licence
evidence.
