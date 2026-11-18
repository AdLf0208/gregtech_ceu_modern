#!/usr/bin/env python3
"""Carry the GregTech CEu slice's assets from the exact jar, and write the derived JSON.

The jar's own files are copied byte for byte (textures, `.mcmeta`); everything that
names a GT block or item is written here, because GT generates it at runtime:

- blockstates: the jar's six `facing=` variants, cut to the four horizontal ones
  (the machines of this slice have no vertical front), with the jar's own rotations;
- models: GT's `gtceu:machine` model loader and its `recipe_logic_status` multipart do
  not exist in a 26.3 content mod, so the boiler/turbine/macerator models are written
  as plain element lists using GT's own `sided` template geometry and the jar's own
  textures (base cube + the overlay planes the jar's model passes);
- the ore block: GT's `neoforge:composite` ore model becomes a stone cube plus the two
  ore layers it composites;
- item definitions (`assets/gtceu/items/*.json`, 26.3), item models, the
  `horizon/block` and `horizon/item` declarations, loot tables, tags and lang.

Run it from the port: `python3 tools/assemble_content.py` (or `--check` to verify the
tree matches the jar without writing).
"""
import json
from pathlib import Path
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
JAR = ROOT.parents[1] / "ports/_shared/licence-cache/gtceu-1.21.1-7.0.2.jar"
CONTENT = ROOT / "content"
NS = "gtceu"

# The jar's files that travel byte for byte. Every one is referenced by a model below.
CARRY = [
    # the three machines' casings and overlays
    "assets/gtceu/textures/block/casings/steam/bricked_bronze/bottom.png",
    "assets/gtceu/textures/block/casings/steam/bricked_bronze/side.png",
    "assets/gtceu/textures/block/casings/steam/bricked_bronze/top.png",
    "assets/gtceu/textures/block/casings/voltage/lv/bottom.png",
    "assets/gtceu/textures/block/casings/voltage/lv/side.png",
    "assets/gtceu/textures/block/casings/voltage/lv/top.png",
    "assets/gtceu/textures/block/generators/boiler/coal/overlay_front.png",
    "assets/gtceu/textures/block/generators/boiler/coal/overlay_front_active.png",
    "assets/gtceu/textures/block/generators/boiler/coal/overlay_front_active.png.mcmeta",
    "assets/gtceu/textures/block/generators/steam_turbine/overlay_side.png",
    "assets/gtceu/textures/block/generators/steam_turbine/overlay_side_active.png",
    "assets/gtceu/textures/block/generators/steam_turbine/overlay_side_active.png.mcmeta",
    "assets/gtceu/textures/block/machines/macerator/overlay_front.png",
    "assets/gtceu/textures/block/machines/macerator/overlay_front_active.png",
    "assets/gtceu/textures/block/machines/macerator/overlay_front_active.png.mcmeta",
    "assets/gtceu/textures/block/machines/macerator/overlay_top.png",
    "assets/gtceu/textures/block/machines/macerator/overlay_top_active.png",
    "assets/gtceu/textures/block/machines/macerator/overlay_top_active.png.mcmeta",
    # the ore block's two layers, and the crushed ore item's three
    "assets/gtceu/textures/block/material_sets/dull/ore.png",
    "assets/gtceu/textures/block/material_sets/dull/ore_layer2.png",
    "assets/gtceu/textures/item/material_sets/dull/crushed.png",
    "assets/gtceu/textures/item/material_sets/dull/crushed_secondary.png",
    "assets/gtceu/textures/item/material_sets/dull/crushed_overlay.png",
]

BLOCKS = ["lp_steam_solid_boiler", "lv_steam_turbine", "lv_macerator"]

LANG = {
    "block.gtceu.lp_steam_solid_boiler": "Low Pressure Steam Solid Boiler",
    "block.gtceu.lv_steam_turbine": "Basic Steam Turbine Generator",
    "block.gtceu.lv_macerator": "Basic Macerator",
    "block.gtceu.iron_ore": "Iron Ore",
    "item.gtceu.crushed_iron_ore": "Crushed Iron Ore",
    "gtceu.status.boiler": "burn %s/%s ticks, temperature %s/%s, water %s/%s mB, steam %s/%s mB, dry %s",
    "gtceu.status.turbine": "steam %s/%s mB, EU %s/%s, progress %s/%s, distilled water %s/%s mB",
    "gtceu.status.macerator": "EU %s/%s, progress %s/%s, input %s, output %s",
    "gtceu.machine.not_a_machine": "No GregTech machine at %s,%s,%s.",
    "gtceu.machine.no_recipe": "%s has no macerator recipe in this slice.",
    "gtceu.machine.usage": "Usage: /gtceu status|take|fill <x> <y> <z> [mb]",
}

# The four horizontal facings, with the rotations GT's own blockstate gives them.
FACING = {"north": None, "east": 90, "south": 180, "west": 270}

# name -> (bottom, side, top, [(depth, [faces])]) — GT's own textures per machine: the
# boiler is a bricked-bronze casing with a coal-door overlay on the front; the turbine
# and macerator are LV casings with their overlay on the sides and the front/top.
MODELS = {
    "lp_steam_solid_boiler": (
        "steam/bricked_bronze/bottom",
        "steam/bricked_bronze/side",
        "steam/bricked_bronze/top",
        [(-0.01, ["north"])],
    ),
    "lv_steam_turbine": (
        "voltage/lv/bottom",
        "voltage/lv/side",
        "voltage/lv/top",
        [(-0.01, ["west", "east"])],
    ),
    "lv_macerator": (
        "voltage/lv/bottom",
        "voltage/lv/side",
        "voltage/lv/top",
        [(-0.01, ["north"]), (-0.02, ["up"])],
    ),
}

# The overlay each machine shows idle, and the one it shows while working: GT picks
# them from its `recipe_logic_status` blockstate property, which this slice declares.
OVERLAY_TEXTURE = {
    "lp_steam_solid_boiler": {
        "idle": {"north": f"{NS}:block/generators/boiler/coal/overlay_front"},
        "working": {"north": f"{NS}:block/generators/boiler/coal/overlay_front_active"},
    },
    "lv_steam_turbine": {
        "idle": {
            "west": f"{NS}:block/generators/steam_turbine/overlay_side",
            "east": f"{NS}:block/generators/steam_turbine/overlay_side",
        },
        "working": {
            "west": f"{NS}:block/generators/steam_turbine/overlay_side_active",
            "east": f"{NS}:block/generators/steam_turbine/overlay_side_active",
        },
    },
    "lv_macerator": {
        "idle": {
            "north": f"{NS}:block/machines/macerator/overlay_front",
            "up": f"{NS}:block/machines/macerator/overlay_top",
        },
        "working": {
            "north": f"{NS}:block/machines/macerator/overlay_front_active",
            "up": f"{NS}:block/machines/macerator/overlay_top_active",
        },
    },
}

# Every text file this script owns: relative path -> text.
FILES = {}


def put(relative, data):
    text = data if isinstance(data, str) else json.dumps(data, indent=2, sort_keys=True) + "\n"
    FILES[Path(relative)] = text


def blockstate(name, model, facings=True, statuses=None):
    """The block's variants: one per facing (no `y` means north), times one per status."""
    variants = {}
    if facings:
        for facing, rotation in FACING.items():
            for status, suffix in (statuses or {"": ""}).items():
                key = f"facing={facing}" if not status else f"facing={facing},recipe_logic_status={status}"
                variant = {"model": f"{model}{suffix}"}
                if rotation is not None:
                    variant["y"] = rotation
                variants[key] = variant
    else:
        variants[""] = {"model": model}
    put(f"assets/{NS}/blockstates/{name}.json", {"variants": variants})


def cube(bottom, side, top):
    """GT's `block/machine/template/sided/sided` base cube, with its cullfaces."""
    return {
        "from": [0, 0, 0],
        "to": [16, 16, 16],
        "faces": {
            "down": {"texture": "#bottom", "cullface": "down"},
            "up": {"texture": "#top", "cullface": "up"},
            "north": {"texture": "#side", "cullface": "north"},
            "south": {"texture": "#side", "cullface": "south"},
            "west": {"texture": "#side", "cullface": "west"},
            "east": {"texture": "#side", "cullface": "east"},
        },
    }


def overlay(faces, depth):
    """One of GT's overlay planes: a 0.01 larger cube showing only the faces named."""
    return {
        "from": [-depth, -depth, -depth],
        "to": [16 + depth, 16 + depth, 16 + depth],
        "faces": {
            face: {"uv": [0, 0, 16, 16], "texture": f"#{face}", "cullface": face}
            for face in faces
        },
    }


def machine_model(name, bottom, side, top, overlays, status):
    textures = {
        "particle": f"{NS}:block/casings/{side}",
        "bottom": f"{NS}:block/casings/{bottom}",
        "side": f"{NS}:block/casings/{side}",
        "top": f"{NS}:block/casings/{top}",
    }
    elements = [cube(bottom, side, top)]
    for depth, faces in overlays:
        elements.append(overlay(faces, depth))
        for face in faces:
            textures[face] = OVERLAY_TEXTURE[name][status][face]
    return {
        "parent": "minecraft:block/block",
        "render_type": "minecraft:cutout",
        "textures": textures,
        "elements": elements,
    }


def machine_blocks():
    for name in BLOCKS:
        bottom, side, top, overlays = MODELS[name]
        blockstate(
            name,
            f"{NS}:block/machine/{name}",
            statuses={"idle": "", "working": "_active"},
        )
        put(
            f"assets/{NS}/models/block/machine/{name}.json",
            machine_model(name, bottom, side, top, overlays, "idle"),
        )
        put(
            f"assets/{NS}/models/block/machine/{name}_active.json",
            machine_model(name, bottom, side, top, overlays, "working"),
        )
        put(
            f"assets/{NS}/models/item/{name}.json",
            {"parent": f"{NS}:block/machine/{name}"},
        )
        put(
            f"assets/{NS}/items/{name}.json",
            {"model": {"type": "minecraft:model", "model": f"{NS}:item/{name}"}},
        )
        put(
            f"data/{NS}/horizon/block/{name}.json",
            {
                "template": "minecraft:stone",
                "properties": {
                    "hardness": 3.0,
                    "explosion_resistance": 6.0,
                    "requires_tool": True,
                    "sounds": "minecraft:iron_block",
                },
                "state_properties": {
                    "facing": {"type": "enum", "values": list(FACING)},
                    "recipe_logic_status": {"type": "enum", "values": ["idle", "working"]},
                },
                "default_state": {"facing": "north", "recipe_logic_status": "idle"},
                "item": True,
            },
        )
        put(
            f"data/{NS}/loot_table/blocks/{name}.json",
            {
                "type": "minecraft:block",
                "pools": [
                    {
                        "bonus_rolls": 0.0,
                        "rolls": 1.0,
                        "entries": [{"type": "minecraft:item", "name": f"{NS}:{name}"}],
                        "condition": {"type": "minecraft:survives_explosion"},
                    }
                ],
                "random_sequence": f"{NS}:blocks/{name}",
            },
        )


def ore_block():
    """`gtceu:iron_ore`: GT's composite ore model flattened to a stone cube and its layers."""
    faces = ("down", "up", "north", "south", "west", "east")
    stone = {
        "from": [0, 0, 0],
        "to": [16, 16, 16],
        "faces": {face: {"texture": "#base", "cullface": face} for face in faces},
    }
    layers = []
    for index, texture in enumerate(("ore", "ore_layer2"), start=1):
        depth = -0.01 * index
        layers.append(
            {
                "from": [depth, depth, depth],
                "to": [16 + depth, 16 + depth, 16 + depth],
                "faces": {face: {"texture": f"#{texture}", "cullface": face} for face in faces},
            }
        )
    put(
        f"assets/{NS}/models/block/iron_ore.json",
        {
            "parent": "minecraft:block/block",
            "render_type": "minecraft:cutout",
            "textures": {
                "particle": "minecraft:block/stone",
                "base": "minecraft:block/stone",
                "ore": f"{NS}:block/material_sets/dull/ore",
                "ore_layer2": f"{NS}:block/material_sets/dull/ore_layer2",
            },
            "elements": [stone, *layers],
        },
    )
    put(f"assets/{NS}/models/item/iron_ore.json", {"parent": f"{NS}:block/iron_ore"})
    put(
        f"assets/{NS}/items/iron_ore.json",
        {"model": {"type": "minecraft:model", "model": f"{NS}:item/iron_ore"}},
    )
    blockstate("iron_ore", f"{NS}:block/iron_ore", facings=False)
    put(
        f"data/{NS}/horizon/block/iron_ore.json",
        {
            "template": "minecraft:stone",
            "properties": {
                "hardness": 3.0,
                "explosion_resistance": 3.0,
                "requires_tool": True,
                "sounds": "minecraft:stone",
            },
            "item": True,
        },
    )
    put(
        f"data/{NS}/loot_table/blocks/iron_ore.json",
        {
            "type": "minecraft:block",
            "pools": [
                {
                    "bonus_rolls": 0.0,
                    "rolls": 1.0,
                    "entries": [{"type": "minecraft:item", "name": f"{NS}:iron_ore"}],
                    "condition": {"type": "minecraft:survives_explosion"},
                }
            ],
            "random_sequence": f"{NS}:blocks/iron_ore",
        },
    )


def crushed_item():
    """`gtceu:crushed_iron_ore`: the dull material set's three layers, as the jar draws it."""
    put(
        f"assets/{NS}/models/item/crushed_iron_ore.json",
        {
            "parent": "minecraft:item/generated",
            "textures": {
                "layer0": f"{NS}:item/material_sets/dull/crushed",
                "layer1": f"{NS}:item/material_sets/dull/crushed_secondary",
                "layer2": f"{NS}:item/material_sets/dull/crushed_overlay",
            },
        },
    )
    put(
        f"assets/{NS}/items/crushed_iron_ore.json",
        {"model": {"type": "minecraft:model", "model": f"{NS}:item/crushed_iron_ore"}},
    )
    put(f"data/{NS}/horizon/item/crushed_iron_ore.json", {"template": "minecraft:gunpowder"})


def tags():
    put(
        "data/minecraft/tags/block/mineable/pickaxe.json",
        {"values": [f"{NS}:{name}" for name in (*BLOCKS, "iron_ore")]},
    )
    # The ore tag the macerator's recipe matches upstream (`c:ores/iron`, under
    # `c:ores`): GT's own ore block and the vanilla one of the same metal.
    put(
        "data/c/tags/block/ores/iron.json",
        {"values": [f"{NS}:iron_ore", "minecraft:iron_ore"]},
    )
    put("data/c/tags/block/ores.json", {"values": ["#c:ores/iron"]})


def manifest():
    put(
        "horizon-mod.toml",
        f"""# Port-owned content mod for the GregTech CEu Modern 7.0.2 slice: the low pressure
# steam solid boiler, the LV steam turbine and the LV macerator, with the iron ore
# they work on. Textures are the jar's own assets under LGPL-3.0 (see SOURCE.md).
id = "{NS}"
name = "GregTech CEu Modern"
version = "0.1.0"
api = "0.2"
side = "both"
capabilities = ["registries"]
""",
    )


def carry(jar, check):
    with zipfile.ZipFile(jar) as archive:
        for path in CARRY:
            data = archive.read(path)
            target = CONTENT / path
            if check:
                if not target.is_file() or target.read_bytes() != data:
                    raise SystemExit(f"{target} is not the jar's {path}")
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
    return len(CARRY)


def main():
    check = "--check" in sys.argv[1:]
    if not JAR.is_file():
        raise SystemExit(f"missing the jar: {JAR}")
    carried = carry(JAR, check)
    machine_blocks()
    ore_block()
    crushed_item()
    tags()
    put(f"assets/{NS}/lang/en_us.json", LANG)
    manifest()
    if check:
        stale = [
            str(relative)
            for relative, text in FILES.items()
            if not (CONTENT / relative).is_file()
            or (CONTENT / relative).read_text() != text
        ]
        if stale:
            raise SystemExit("content is not what the jar gives: " + ", ".join(sorted(stale)))
        print(json.dumps({"carried": carried, "generated": len(FILES), "check": True}, indent=2))
        return
    for relative, text in FILES.items():
        target = CONTENT / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
    print(json.dumps({"carried": carried, "generated": len(FILES), "check": False}, indent=2))


if __name__ == "__main__":
    main()
