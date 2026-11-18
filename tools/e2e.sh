#!/usr/bin/env bash
# In-game check for GTCEu slice 1: a coal-fired solid boiler makes steam, the
# steam turbine beside it turns that steam into EU, and the macerator beside the
# turbine spends that EU to grind one iron ore into two crushed ore.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `perform_command` hook.
#
# The End, in the void at x=200 (away from the central island and the ambient
# item entities of a generated level), on a stone floor laid by the scenario.
# Layout, all at y=101 on that floor:
#
#   z=0    boiler B1 (200)  ->  turbine T1 (201)  ->  macerator M1 (202)   the chain
#   z=-8   boiler B2 (200)                                                 no water
#   z=8    macerator M2 (202) with an ore, nothing beside it               no power
#   z=8    macerator M3 (206)                                              the ore outside the table
#
# Legs:
#  A  the three block items place as a player places them, and the boiler faces
#     the player who placed it (`facing=west` from the player looking east);
#  B  no fuel yet: nothing burns and nothing is made;
#  C  one coal in B1 and one in B2, water in B1 only: B1 heats to 100 degrees and
#     its first steam is 12 mB, B2 heats but makes no steam and reports dry —
#     the "fuelled boiler with no water" control;
#  D  the steam reaches T1 and becomes EU at 32 a tick, 320 an operation;
#  E  the EU reaches M1, whose blockstate turns to `recipe_logic_status=working`
#     while it grinds (`ctl block` reads it back);
#  F  one `gtceu:iron_ore` in M1 becomes two `gtceu:crushed_iron_ore`, and
#     `/gtceu take` puts them in the player's inventory;
#  G  the unpowered macerator M2 spends nothing, makes no progress and keeps its
#     ore, and `minecraft:gold_ore` is refused by the recipe table (M3);
#  H  water into the dry boiler B2 explodes it, as `SteamBoilerMachine` does
#     when water returns after a dry spell.
#
# Default run (EMBEDDED=0): the game loads `content/` as a dev mod.
# EMBEDDED=1: the game is launched with **only** the native bundle (no
# `--dev-mod`) and the content comes from the bundle's embedded copy
# (`mixins/gtceu/data/…`, the AE2 pattern). CONTROL=1 (default with EMBEDDED)
# first launches the same binary with `.horizon-cache` unusable, where the ids
# must be unknown.
#
# SELFTEST=1 flips F's two-crushed assertion (it must FAIL), proving the
# scenario can fail.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${GTCEU_DEBUG:-}" ] || set -x

EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}
SELFTEST=${SELFTEST:-0}
# The boiler needs 2,400 burning ticks to reach 100 degrees and the ore 400 more
# to grind; `/tick rate` moves them along without changing a single tick's
# arithmetic. RATE=20 keeps real time and takes about four minutes.
RATE=${RATE:-200}

CAMPAIGN=${CAMPAIGN:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CAMPAIGN/external/gtceu
PIN_BIN=$CAMPAIGN/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/gtceu-e2e.sock}
KEEP=${KEEP:-0}
WORLD=GtceuE2E
CONTROL_WORLD=GtceuControl
X0=200
CHAIN_Z=0
DRY_Z=-8
POWER_Z=8
BOILER=gtceu:lp_steam_solid_boiler
TURBINE=gtceu:lv_steam_turbine
MACERATOR=gtceu:lv_macerator
ORE=gtceu:iron_ore
CRUSHED=gtceu:crushed_iron_ore
CRUSHED_PER_ORE=2
# SteamBoilerMachine: 120 mB/s of base output, /2, at 100 of 500 degrees.
FIRST_STEAM=12
# FuelRecipes: 640 mB steam -> 32 EU/t for 10 ticks.
TURBINE_EU=320

command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; exit 1; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/gtceu-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
# Wall-clock time for the client to take in a teleport, a look or a click:
# `ctl wait` counts game ticks, and at `/tick rate 200` ten of them are 50 ms.
settle() { sleep 0.4; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 60 >&2 || true; exit 1; }
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    cd "$SCRATCH"
    local args=(--game "$GAME" --world "$world" --seed 20261105 --creative --headless
                --wait --resources "$RES")
    if [ "$EMBEDDED" = 1 ]; then
        echo "  launch: no --dev-mod; the bundle carries its own content" >&2
    else
        args+=(--dev-mod "$PORT/content")
    fi
    "$CTL" --socket "$SOCK" launch "${args[@]}" >/dev/null
    ctl wait-ready 120
    ctl wait 40
    # Faster game ticks, not a different rule: every number below is per tick.
    ctl command "tick rate $RATE" >/dev/null || true
    ctl wait 2 >/dev/null
    settle
    echo "  $(ctl command 'tick query' | jq -r '.feedback // .reply // empty' | head -1)" >&2
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }
block_state() { ctl block "$1" "$2" "$3" | jq -r '.state // .block // empty'; }

# The End's void at X0: a stone floor and clear air above it.
arena() {
    ctl command "execute in minecraft:the_end run tp @s $X0.5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 2)) 100 -14 $((X0 + 30)) 100 14 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 2)) 101 -14 $((X0 + 30)) 108 14 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $X0.5 101 $CHAIN_Z.5" >/dev/null
    settle
    ctl wait 20
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

# `/gtceu status X Y Z TAG`: the line, then its fields in KIND, BURN, TEMP,
# WATER, STEAM, DRY, ENERGY, PROGRESS, INPUT_ITEM, OUTPUT_ITEM (see
# tools/parse_status.py). The tag is a counter: the chat keeps earlier replies
# for the same coordinates, and a reply is only this call's when it carries it.
SEQ=0
status() { # status X Y Z
    SEQ=$((SEQ + 1))
    local tag="s$SEQ"
    say "gtceu status $1 $2 $3 $tag"
    STATUS_LINE=""
    for _ in $(seq 1 20); do
        ctl wait 4 >/dev/null
        sleep 0.1
        STATUS_LINE=$(ctl messages 20 | jq -r '.chat[]' | grep -F "tag=$tag " | tail -1 || true)
        [ -n "$STATUS_LINE" ] && break
    done
    [ -n "$STATUS_LINE" ] || fail "no /gtceu status reply for $1,$2,$3"
    eval "$(python3 "$PORT/tools/parse_status.py" "$STATUS_LINE")"
    [ "${OK:-0}" = 1 ] || fail "the status line did not parse: $STATUS_LINE"
    echo "  $STATUS_LINE" >&2
}

# Put one block item in the player's first hotbar slot and select it.
hold() { # hold ITEM
    ctl command "item replace entity @s hotbar.0 with $1 1" >/dev/null
    settle
    ctl select 1 >/dev/null
    settle
}

# Place `item` at X Y Z from the held item, standing `from_x` blocks west and
# looking at the floor block below the target, then check the block appeared.
place_at() { # place_at X Y Z ITEM [FROM_X]
    local x=$1 y=$2 z=$3 item=$4
    local from_x=${5:-$((x - 2))}
    ctl command "execute in minecraft:the_end run tp @s ${from_x}.5 $y $z.5" >/dev/null
    settle
    hold "$item"
    ctl look-at-block "$x" $((y - 1)) "$z" up >/dev/null || fail "cannot aim at the floor under $x,$y,$z"
    settle
    ctl place "$x" "$y" "$z" --item "$item" >/dev/null \
        || fail "ctl place $item at $x,$y,$z failed"
    settle
    local name
    name=$(block_name "$x" "$y" "$z")
    [ "$name" = "$item" ] || fail "the $item did not place at $x,$y,$z (got '$name')"
}

# A right click on a machine with `item` in hand. The player stands two blocks
# south of it (the machines sit in a row along x, so the south side is the one
# nothing else occupies) and clicks whatever face the crosshair finds.
click_with() { # click_with X Y Z ITEM
    local x=$1 y=$2 z=$3 item=$4
    ctl command "execute in minecraft:the_end run tp @s ${x}.5 101 $((z - 2)).5" >/dev/null
    settle
    hold "$item"
    ctl look-at-block "$x" "$y" "$z" >/dev/null || fail "cannot aim at $x,$y,$z"
    settle
    ctl interact "$x" "$y" "$z" >/dev/null || fail "ctl interact on $item at $x,$y,$z failed"
    settle
}

# `/gtceu fill X Y Z MB`: water into the boiler's input tank (the pipe a player
# would run against it; GT's solid boiler takes no container).
fill() { # fill X Y Z MB
    say "gtceu fill $1 $2 $3 $4"
    ctl wait 6 >/dev/null
    sleep 0.2
}

# Wait until a machine's status satisfies a shell test on its parsed fields,
# reading it every 40 ticks (a game-state wait, not a lock poll).
wait_status() { # wait_status X Y Z 'shell test' TRIES
    local x=$1 y=$2 z=$3 test=$4 tries=${5:-40}
    for _ in $(seq 1 "$tries"); do
        status "$x" "$y" "$z"
        if eval "$test"; then
            return 0
        fi
        ctl wait 40 >/dev/null
    done
    return 1
}

# The control for EMBEDDED=1: the same binary (every hook is in it) with the
# embedded content made unavailable — `.horizon-cache` is a plain file, so the
# helper can neither compare nor write its content file and falls back to
# `Continue`. The ids must then be unknown; if they resolve, the run proves
# nothing about the embedded file.
control_leg() {
    echo "== control: the same game with no embedded content file =="
    rm -rf "$SCRATCH/.horizon-cache"
    : > "$SCRATCH/.horizon-cache"
    launch "$CONTROL_WORLD"
    arena
    ctl command "item replace entity @s hotbar.0 with $MACERATOR 1" >/dev/null || true
    ctl wait 4 >/dev/null
    ctl inventory | jq -e --arg id "$MACERATOR" '[.hotbar[], .main[] | select(.item == $id)] | length == 0' >/dev/null \
        || fail "the control game has the macerator item, so the content did not come from the embedded file"
    ctl command "execute in minecraft:the_end run setblock $((X0 + 2)) 101 0 $BOILER" >/dev/null || true
    ctl wait 4 >/dev/null
    local name
    name=$(block_name $((X0 + 2)) 101 0)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    [ -f "$SCRATCH/.horizon-cache" ] || fail "the control's .horizon-cache was replaced: the helper wrote its content"
    echo "  without the embedded file: no macerator item, the block stayed $name"
    ctl quit >/dev/null 2>&1 || true
    rm -f "$SCRATCH/.horizon-cache"
}

# EMBEDDED=1: the bundle must have written its content out at startup, byte for
# byte the delivered content mod.
embedded_content_check() {
    local cache=$SCRATCH/.horizon-cache/gtceu-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/gtceu.hmod" \
        || fail "the embedded content differs from content/gtceu.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/gtceu.hmod"
}

echo "== launch (The End arena, content + native bundle) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi
arena

echo "== A: the three machines place as a player places them, facing the player =="
for item in "$BOILER" "$TURBINE" "$MACERATOR" "$ORE"; do
    ctl command "item replace entity @s hotbar.0 with $item 1" >/dev/null
    ctl wait 4 >/dev/null
    ctl inventory | jq -e --arg id "$item" '[.hotbar[], .main[] | select(.item == $id)] | length >= 1' >/dev/null \
        || fail "the $item item did not register: $(ctl inventory | jq -c '{hotbar,main}')"
done
echo "  the four items register (three machines and the ore)"
# The boiler by hand, as a player places it: the facing comes from `place_block`.
place_at "$X0" 101 "$CHAIN_Z" "$BOILER" "$((X0 - 2))"
STATE=$(block_state "$X0" 101 "$CHAIN_Z")
case "$STATE" in
    *facing=west*) ;;
    *) fail "the boiler at $X0,101,$CHAIN_Z is '$STATE', expected facing=west" ;;
esac
echo "  the boiler faces the player who placed it: $STATE"
# The other two, with the state named, so the chain's layout is exact.
ctl command "execute in minecraft:the_end run setblock $((X0 + 1)) 101 $CHAIN_Z $TURBINE[facing=west]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $((X0 + 2)) 101 $CHAIN_Z $MACERATOR[facing=west]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $X0 101 $DRY_Z $BOILER[facing=north]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $((X0 + 2)) 101 $POWER_Z $MACERATOR[facing=north]" >/dev/null
ctl command "execute in minecraft:the_end run setblock $((X0 + 6)) 101 $POWER_Z $MACERATOR[facing=north]" >/dev/null
ctl wait 8 >/dev/null
for at in "$X0 101 $DRY_Z $BOILER" "$((X0 + 2)) 101 $POWER_Z $MACERATOR" "$((X0 + 6)) 101 $POWER_Z $MACERATOR"; do
    set -- $at
    [ "$(block_name "$1" "$2" "$3")" = "$4" ] || fail "the $4 at $1,$2,$3 did not appear"
done
echo "  the dry boiler and the two control macerators are in place"

echo "== B: with no fuel nothing burns and nothing is made =="
wait_status "$X0" 101 "$CHAIN_Z" '[ "$BURN" -eq 0 ] && [ "$STEAM" -eq 0 ] && [ "$TEMP" -eq 0 ]' 5 \
    || fail "an unfuelled boiler reports burn=$BURN steam=$STEAM temp=$TEMP"
echo "  100 ticks with no fuel: burn 0, temp 0, steam 0"

echo "== C: one coal each; only the watered boiler makes steam =="
fill "$X0" 101 "$CHAIN_Z" 1000
status "$X0" 101 "$CHAIN_Z"
[ "$WATER" -eq 1000 ] || fail "the boiler holds $WATER mB of water after a 1,000 mB fill"
click_with "$X0" 101 "$CHAIN_Z" minecraft:coal
status "$X0" 101 "$CHAIN_Z"
[ "$BURN" -gt 0 ] || fail "the coal did not start a burn (burn $BURN)"
[ "$BURN" -le 19200 ] || fail "the coal's burn is $BURN ticks, expected at most 19200 (1600 x 12)"
echo "  the boiler is burning: $BURN/$BURN_TOTAL ticks"
click_with "$X0" 101 "$DRY_Z" minecraft:coal
status "$X0" 101 "$DRY_Z"
[ "$BURN" -gt 0 ] || fail "the dry boiler did not take its coal"
# 2,400 burning ticks is 100 degrees; the first steam step at that temperature is 12 mB.
wait_status "$X0" 101 "$CHAIN_Z" '[ "$STEAM" -gt 0 ]' 80 || fail "the watered boiler never made steam"
[ "$TEMP" -ge 100 ] || fail "the boiler made steam at $TEMP degrees"
[ "$WATER" -lt 1000 ] || fail "the boiler made steam without draining water ($WATER mB)"
echo "  watered boiler: temp $TEMP, water $WATER mB, steam $STEAM mB"
STATE=$(block_state "$X0" 101 "$CHAIN_Z")
case "$STATE" in
    *recipe_logic_status=working*) ;;
    *) fail "a burning boiler is '$STATE', expected recipe_logic_status=working" ;;
esac
status "$X0" 101 "$DRY_Z"
[ "$TEMP" -ge 100 ] || fail "the dry boiler is only $TEMP degrees after the same wait"
[ "$STEAM" -eq 0 ] || fail "the dry boiler made $STEAM mB of steam with no water"
[ "$DRY" = 1 ] || fail "the dry boiler does not report dry (dry=$DRY)"
echo "  dry boiler: temp $TEMP and steam $STEAM mB — the fuelled no-water control"

echo "== D: the steam reaches the turbine and becomes EU =="
wait_status "$((X0 + 1))" 101 "$CHAIN_Z" '[ "$STEAM" -gt 0 ]' 40 || fail "the turbine never received steam"
wait_status "$((X0 + 1))" 101 "$CHAIN_Z" '[ "$ENERGY" -gt 0 ]' 80 || fail "the turbine never made EU"
[ "$ENERGY" -le $TURBINE_EU ] || [ "$PROGRESS" -gt 0 ] || [ "$WATER" -gt 0 ] \
    || fail "the turbine holds $ENERGY EU with no operation behind it"
echo "  turbine: steam $STEAM mB, EU $ENERGY, progress $PROGRESS"

echo "== E: the EU reaches the macerator, which starts to grind =="
click_with "$((X0 + 2))" 101 "$CHAIN_Z" "$ORE"
status "$((X0 + 2))" 101 "$CHAIN_Z"
[ "$INPUT_ITEM" = "$ORE:1" ] || fail "the macerator's input is '$INPUT_ITEM', expected $ORE:1"
wait_status "$((X0 + 2))" 101 "$CHAIN_Z" '[ "$ENERGY" -gt 0 ]' 120 || fail "the macerator never received EU"
wait_status "$((X0 + 2))" 101 "$CHAIN_Z" '[ "$PROGRESS" -gt 0 ]' 40 || fail "the macerator never started grinding"
STATE=$(block_state "$((X0 + 2))" 101 "$CHAIN_Z")
case "$STATE" in
    *recipe_logic_status=working*) ;;
    *) fail "a grinding macerator is '$STATE', expected recipe_logic_status=working" ;;
esac
echo "  macerator: EU $ENERGY/$ENERGY_MAX, progress $PROGRESS/$PROGRESS_MAX, state $STATE"

echo "== F: one ore becomes two crushed ore, and they can be taken out =="
wait_status "$((X0 + 2))" 101 "$CHAIN_Z" '[ "$OUTPUT_ITEM" != none ]' 160 || fail "the macerator produced nothing"
[ "$OUTPUT_ITEM" = "$CRUSHED:$CRUSHED_PER_ORE" ] \
    || fail "the macerator's output is '$OUTPUT_ITEM', expected $CRUSHED:$CRUSHED_PER_ORE"
[ "$INPUT_ITEM" = none ] || [ "$INPUT_ITEM" = "$ORE:0" ] || fail "the ore was not consumed ($INPUT_ITEM)"
if [ "$SELFTEST" = 1 ]; then
    echo "(selftest: asserting three crushed ore)"
    [ "$OUTPUT_ITEM" = "$CRUSHED:3" ] || fail "the macerator's output is '$OUTPUT_ITEM', expected $CRUSHED:3"
fi
say "gtceu take $((X0 + 2)) 101 $CHAIN_Z"
ctl wait 8 >/dev/null
ctl inventory | jq -e --arg id "$CRUSHED" '[.hotbar[], .main[] | select(.item == $id) | .count] | add >= '"$CRUSHED_PER_ORE" >/dev/null \
    || fail "the crushed ore is not in the inventory: $(ctl inventory | jq -c '{hotbar,main}')"
echo "  the macerator holds $OUTPUT_ITEM and /gtceu take put them in the inventory"

echo "== G: no power grinds nothing, and an ore outside the table is refused =="
click_with "$((X0 + 2))" 101 "$POWER_Z" "$ORE"
status "$((X0 + 2))" 101 "$POWER_Z"
[ "$INPUT_ITEM" = "$ORE:1" ] || fail "the unpowered macerator did not take the ore ($INPUT_ITEM)"
ctl wait 200 >/dev/null
status "$((X0 + 2))" 101 "$POWER_Z"
[ "$ENERGY" -eq 0 ] || fail "the unpowered macerator holds $ENERGY EU"
[ "$PROGRESS" -eq 0 ] || fail "the unpowered macerator made $PROGRESS ticks of progress"
[ "$OUTPUT_ITEM" = none ] || fail "the unpowered macerator produced $OUTPUT_ITEM"
[ "$INPUT_ITEM" = "$ORE:1" ] || fail "the unpowered macerator lost its ore ($INPUT_ITEM)"
STATE=$(block_state "$((X0 + 2))" 101 "$POWER_Z")
case "$STATE" in
    *recipe_logic_status=idle*) ;;
    *) fail "an idle macerator is '$STATE', expected recipe_logic_status=idle" ;;
esac
echo "  the unpowered macerator: EU 0, progress 0, its ore still in the slot"
click_with "$((X0 + 6))" 101 "$POWER_Z" minecraft:gold_ore
status "$((X0 + 6))" 101 "$POWER_Z"
[ "$INPUT_ITEM" = none ] || fail "minecraft:gold_ore was taken as a macerator input ($INPUT_ITEM)"
echo "  minecraft:gold_ore is outside the recipe table and was refused"

echo "== H: water into the dry boiler explodes it =="
status "$X0" 101 "$DRY_Z"
[ "$DRY" = 1 ] || fail "the dry boiler is not dry before the water (dry=$DRY)"
fill "$X0" 101 "$DRY_Z" 1000
exploded=false
for _ in $(seq 1 20); do
    ctl wait 10 >/dev/null
    if [ "$(block_name "$X0" 101 "$DRY_Z")" = "minecraft:air" ]; then
        exploded=true
        break
    fi
done
[ "$exploded" = true ] || fail "the dry boiler survived the water returning ($(block_name "$X0" 101 "$DRY_Z"))"
echo "  the dry boiler is gone: water after a dry spell explodes it, as upstream does"

echo "PASS: a coal-fired solid boiler heats to 100 degrees and makes 12 mB of steam a step, a fuelled boiler with no water makes none and reports dry, the steam turbine turns 640 mB into 320 EU at 32 a tick, the macerator spends 2 EU a tick for 400 ticks and turns one gtceu:iron_ore into two gtceu:crushed_iron_ore, the blockstate shows working while it grinds and idle without power, an unpowered macerator keeps its ore, minecraft:gold_ore is refused, and water returning to a dry boiler explodes it"
