//! GregTech CEu Modern for Horizon: the opening steam chain.
//!
//! The behaviour and every number live in [`runtime`], a plain module with no
//! `mod_api` types. This file is the glue written for the flat SDK package:
//!
//! - **`register`** declares one block entity type per machine, the boiler's
//!   fuel slot and the macerator's two, the boiler's and turbine's tanks, and
//!   the turbine's emitter and the macerator's receiver buffers (the mod API's
//!   default `block_container` / `block_fluids` / `block_energy` declarations).
//! - **[`MachinePart`]** answers the moments that belong to those blocks: the
//!   facing a placed machine takes (`state_for_placement`), that its block
//!   entity ticks, and one server tick of it.
//! - **`right_click_block`** inserts fuel or ore, or reads the machine.
//! - **`level_tick_post`** moves the boiler's steam into the turbine and the
//!   turbine's EU into the macerator, after every block entity is back in the
//!   world (a block entity is out of the world during its own tick).
//! - **`register_commands`** / **`run_mod_command`** carry `/gtceu status`,
//!   `/gtceu take` and `/gtceu fill`.
//!
//! The saved compound keeps exactly the keys the retired bundle used — the
//! machine state, the engine's `fluids` list and `energy` — so a save taken by
//! the old package reads back here and the pin's default capability providers
//! find the tanks and the buffer where they expect them.
pub mod runtime;

use std::sync::Mutex;

use mod_api::{
    self, BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent, BlockPart, BlockState,
    BlockUpdateFlags, Command, CommandArgumentValue, Compound, Direction, ExplosionInteraction,
    ExplosionSettings, Flow, Hand, Hooks, HostSide, HostTransaction, Identifier, InteractionResult,
    ItemKey, ItemPlace, ItemResource, ItemStack, LevelRef, ModContainer, ModEnergyBuffer,
    ModFluidTanks, PlayerRef, Registrar, RegistrationContext, RightClickBlockEvent,
    RunModCommandEvent, ServerHost, StateForPlacementEvent, Tag, Text,
};

use crate::runtime::{
    Boiler, MACERATOR_CAPACITY, MACERATOR_CRUSHED, MACERATOR_DURATION, MACERATOR_EUT,
    MAX_TEMPERATURE, Macerator, MaceratorStep, STEAM_TANK_MB, TURBINE_CAPACITY, TURBINE_DURATION,
    TURBINE_MAX_TRANSFER, TURBINE_OUTPUT_TANK_MB, TURBINE_STEAM_TANK_MB, Turbine, WATER_TANK_MB,
    burn_ticks_for, macerator_output,
};

const BOILER: &str = "gtceu:lp_steam_solid_boiler";
const TURBINE: &str = "gtceu:lv_steam_turbine";
const MACERATOR: &str = "gtceu:lv_macerator";
const STEAM: &str = "gtceu:steam";
const DISTILLED_WATER: &str = "gtceu:distilled_water";
const WATER: &str = "minecraft:water";
/// The blockstate property GT switches on its machines while they work.
const STATUS: &str = "recipe_logic_status";
const FACING: &str = "facing";
// The saved keys: everything but the item slots (the engine saves those) and
// the tank amounts (the engine's `fluids` list).
const KEY_TEMPERATURE: &str = "temperature";
const KEY_BURN: &str = "burn";
const KEY_BURN_TOTAL: &str = "burn_total";
const KEY_TICKS: &str = "ticks";
const KEY_COOLDOWN: &str = "cooldown";
const KEY_DRY: &str = "dry";
const KEY_PROGRESS: &str = "progress";
const KEY_ENERGY: &str = "energy";
const KEY_FLUIDS: &str = "fluids";

// Tank indices, in the order the `fluids` list holds them.
const BOILER_WATER: usize = 0;
const BOILER_STEAM: usize = 1;
const TURBINE_STEAM: usize = 0;
const TURBINE_WATER: usize = 1;

// Slot indices.
const BOILER_FUEL: usize = 0;
const MACERATOR_INPUT: usize = 0;
const MACERATOR_OUTPUT: usize = 1;

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

fn read_bool(nbt: &Compound, key: &str) -> bool {
    read_int(nbt, key) != 0
}

/// One tank of the engine's `fluids` list: its fluid and its amount.
type Tank = (Option<String>, i32);

fn read_tanks(nbt: &Compound, count: usize) -> Vec<Tank> {
    let list = match nbt.get(&KEY_FLUIDS.into()) {
        Some(Tag::List(list)) => list,
        _ => return vec![(None, 0); count],
    };
    let mut tanks: Vec<Tank> = list
        .iter()
        .take(count)
        .map(|tag| match tag {
            Tag::Compound(map) => (
                match map.get(&"Fluid".into()) {
                    Some(Tag::String(fluid)) if fluid.to_string_lossy() != "minecraft:empty" => {
                        Some(fluid.to_string_lossy())
                    }
                    _ => None,
                },
                match map.get(&"Amount".into()) {
                    Some(Tag::Int(amount)) => *amount,
                    _ => 0,
                },
            ),
            _ => (None, 0),
        })
        .collect();
    tanks.resize(count, (None, 0));
    tanks
}

fn write_tanks(nbt: &mut Compound, tanks: &[Tank]) {
    let list = tanks
        .iter()
        .map(|(fluid, amount)| {
            let mut map = Compound::new();
            map.insert(
                "Fluid".into(),
                Tag::String(fluid.as_deref().unwrap_or("minecraft:empty").into()),
            );
            map.insert("Amount".into(), Tag::Int(*amount));
            Tag::Compound(map)
        })
        .collect();
    nbt.insert(KEY_FLUIDS.into(), Tag::List(list));
}

/// The amount in one tank, and nothing where it holds another fluid.
fn tank_amount(tanks: &[Tank], index: usize, fluid: &str) -> i32 {
    match tanks.get(index) {
        Some((Some(name), amount)) if name == fluid => *amount,
        _ => 0,
    }
}

/// Put `amount` of `fluid` in tank `index`, keeping the fluid it already holds.
fn set_tank(tanks: &mut [Tank], index: usize, fluid: &str, amount: i32) {
    if let Some(tank) = tanks.get_mut(index) {
        if amount <= 0 {
            // An emptied tank holds nothing, as `store_boiler` writes it.
            *tank = (None, 0);
            return;
        }
        let holds = tank.0.as_deref() == Some(fluid) || tank.0.is_none();
        if holds {
            *tank = (Some(fluid.to_owned()), amount);
        }
    }
}

/// The level being ticked now, for the writes a block entity's own tick needs.
fn ticking_level(host: &mut dyn ServerHost) -> Option<LevelRef> {
    let levels = host.levels().list_levels();
    levels.into_iter().find(|level| level.in_place(host))
}

/// The block id at `pos`, if the position is loaded.
fn block_at(host: &dyn mod_api::Host, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    level.block(host, pos).map(|block| block.id(host))
}

/// The saved compound of the block entity at `pos`.
fn entity_data(host: &dyn mod_api::Host, level: LevelRef, pos: [i32; 3]) -> Option<Compound> {
    host.block_entity(level, pos)?.data(host)
}

/// Merge `data` into the block entity at `pos`, as `/data merge block` does.
fn merge_data(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], data: Compound) {
    if let Some(entity) = host.block_entity(level, pos) {
        let _ = entity.merge_data(host, data);
    }
}

fn is_machine(block: &str) -> bool {
    matches!(block, BOILER | TURBINE | MACERATOR)
}

// ---------------------------------------------------------------------------
// Registration: the block entity types and the default capabilities
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the three machines' block entities, slots, tanks and buffers.
    /// The blocks themselves come from `data/gtceu/horizon/block/`; a type may
    /// name a block declared later, and the game creates the entity for it.
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let boiler = r
            .block_entity_type("lp_steam_solid_boiler")
            .blocks(&[BOILER])
            .build();
        let turbine = r
            .block_entity_type("lv_steam_turbine")
            .blocks(&[TURBINE])
            .build();
        let macerator = r
            .block_entity_type("lv_macerator")
            .blocks(&[MACERATOR])
            .build();

        let _ = r.block_entity_part(&boiler, MachinePart);
        let _ = r.block_entity_part(&turbine, MachinePart);
        let _ = r.block_entity_part(&macerator, MachinePart);

        let boiler_block = r.block_key(BOILER);
        let turbine_block = r.block_key(TURBINE);
        let macerator_block = r.block_key(MACERATOR);
        let all = |count: usize| -> Vec<usize> { (0..count).collect() };

        // `native_block_entity_container`: the boiler's fuel slot, the
        // macerator's input and output. The turbine holds no items.
        r.block_container(
            &boiler_block,
            ModContainer {
                slots: 1,
                faces: std::array::from_fn(|_| vec![BOILER_FUEL]),
                inputs: vec![BOILER_FUEL],
                outputs: vec![],
                menu: None,
            },
        );
        r.block_container(
            &macerator_block,
            ModContainer {
                slots: 2,
                faces: std::array::from_fn(|_| all(2)),
                inputs: vec![MACERATOR_INPUT],
                outputs: vec![MACERATOR_OUTPUT],
                menu: None,
            },
        );

        // `native_block_entity_fluids`: the boiler's water + steam tanks and
        // the turbine's steam + distilled water, as `IO.IN`/`IO.OUT`.
        let mut boiler_tanks = ModFluidTanks::default();
        boiler_tanks.tanks = vec![WATER_TANK_MB, STEAM_TANK_MB];
        boiler_tanks.faces = std::array::from_fn(|_| all(2));
        boiler_tanks.inputs = vec![BOILER_WATER];
        boiler_tanks.outputs = vec![BOILER_STEAM];
        r.block_fluids(&boiler_block, boiler_tanks);
        let mut turbine_tanks = ModFluidTanks::default();
        turbine_tanks.tanks = vec![TURBINE_STEAM_TANK_MB, TURBINE_OUTPUT_TANK_MB];
        turbine_tanks.faces = std::array::from_fn(|_| all(2));
        turbine_tanks.inputs = vec![TURBINE_STEAM];
        turbine_tanks.outputs = vec![TURBINE_WATER];
        r.block_fluids(&turbine_block, turbine_tanks);

        // `native_block_entity_energy` (API 142): the turbine emits one LV amp
        // and the macerator receives one; both are `V[LV] * 64` = 2,048 EU.
        r.block_energy(
            &turbine_block,
            ModEnergyBuffer {
                capacity: TURBINE_CAPACITY,
                max_insert: 0,
                max_extract: TURBINE_MAX_TRANSFER,
                insert_faces: [false; 6],
                extract_faces: [true; 6],
            },
        );
        r.block_energy(
            &macerator_block,
            ModEnergyBuffer {
                capacity: MACERATOR_CAPACITY,
                max_insert: TURBINE_MAX_TRANSFER,
                max_extract: 0,
                insert_faces: [true; 6],
                extract_faces: [false; 6],
            },
        );
    }

    /// `/gtceu status|take|fill`: the readout, the output pickup and the
    /// harness fill, typed instead of the retired string parser.
    fn register_commands(
        &self,
        _: &mut dyn ServerHost,
        event: &mut mod_api::RegisterCommandsEvent<'_>,
    ) {
        event
            .commands
            .push(Command::new("gtceu status <pos:block_pos> [tag:word]"));
        event
            .commands
            .push(Command::new("gtceu take <pos:block_pos>"));
        event
            .commands
            .push(Command::new("gtceu fill <pos:block_pos> <amount:integer>"));
    }

    /// A right click on a machine: insert fuel or ore, or read the machine.
    fn right_click_block(&self, host: HostSide, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        if event.cancelled {
            return Flow::Continue;
        }
        let pos = event.click.pos;
        let Some(dimension) = dimension_id(event.click.dimension) else {
            return Flow::Continue;
        };
        let Some(level) = host.level(&dimension) else {
            return Flow::Continue;
        };
        let Some(name) = block_at(host, level, pos) else {
            return Flow::Continue;
        };
        if !is_machine(&name) {
            return Flow::Continue;
        }
        // Claim the click: the item is not placed against the machine and no
        // block use runs, as the retired helper cancelling the hook did.
        event.cancelled = true;
        *event.cancellation_result = InteractionResult::SuccessServer;

        let Some(player) = host.player(event.click.player) else {
            return Flow::Cancel;
        };
        let Some(held) = event.click.stack.cloned() else {
            let line = status_text(host, level, pos, "");
            let _ = player.action_bar(host, Text::literal(line));
            return Flow::Cancel;
        };
        match name.as_str() {
            BOILER => {
                let burn = fuel_burn_time(host, &held);
                if burn == 0 {
                    let id = item_id(host, held.item());
                    say(host, player, format!("{id} is not a fuel"));
                    return Flow::Cancel;
                }
                let moved = insert(
                    host,
                    level,
                    pos,
                    BOILER_FUEL,
                    player,
                    event.click.hand,
                    &held,
                );
                if moved > 0 {
                    let id = item_id(host, held.item());
                    say(
                        host,
                        player,
                        format!("Inserted {id} x{moved} into the {name}"),
                    );
                }
            }
            MACERATOR => {
                let id = item_id(host, held.item());
                if macerator_output(&id).is_none() {
                    say(
                        host,
                        player,
                        format!("{id} has no macerator recipe in this slice"),
                    );
                    return Flow::Cancel;
                }
                let moved = insert(
                    host,
                    level,
                    pos,
                    MACERATOR_INPUT,
                    player,
                    event.click.hand,
                    &held,
                );
                if moved > 0 {
                    say(
                        host,
                        player,
                        format!("Inserted {id} x{moved} into the {name}"),
                    );
                }
            }
            _ => {
                let line = status_text(host, level, pos, "");
                say(host, player, line);
            }
        }
        Flow::Cancel
    }

    /// Run one registered `/gtceu` usage after the dispatcher parsed it.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let Some(dimension) = event.command.dimension.id(host) else {
            return Flow::Continue;
        };
        let Some(level) = host.level(&dimension) else {
            return Flow::Continue;
        };
        let source_player = match event.source {
            mod_api::CommandSource::Player { player, .. } => Some(*player),
            _ => None,
        };
        match event.command.literals.as_slice() {
            [root, action] if root == "gtceu" && action == "status" => {
                let Some(pos) = block_pos(event.command.argument("pos")) else {
                    return Flow::Continue;
                };
                let tag = match event.command.argument("tag") {
                    Some(CommandArgumentValue::String(tag)) => tag.clone(),
                    _ => String::new(),
                };
                let line = status_text(host, level, pos, &tag);
                feedback(host, line);
                event.cancelled = true;
                Flow::Cancel
            }
            [root, action] if root == "gtceu" && action == "take" => {
                let Some(pos) = block_pos(event.command.argument("pos")) else {
                    return Flow::Continue;
                };
                let line = take_output(host, level, pos, source_player);
                feedback(host, line);
                event.cancelled = true;
                Flow::Cancel
            }
            [root, action] if root == "gtceu" && action == "fill" => {
                let Some(pos) = block_pos(event.command.argument("pos")) else {
                    return Flow::Continue;
                };
                let Some(CommandArgumentValue::Integer(amount)) = event.command.argument("amount")
                else {
                    return Flow::Continue;
                };
                let line = fill_tank(host, level, pos, *amount);
                feedback(host, line);
                event.cancelled = true;
                Flow::Cancel
            }
            _ => Flow::Continue,
        }
    }

    /// After every level tick: push steam and EU into the neighbouring machine.
    /// A block entity is out of the world during its own tick, so this is the
    /// only point where both ends of the transfer can be read and written.
    fn level_tick_post(&self, host: HostSide, event: &mut mod_api::LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        let level = event.level;
        let dimension = level.dimension_id().to_owned();
        let queued = {
            let mut queue = PUSH.lock().unwrap_or_else(|poison| poison.into_inner());
            std::mem::take(&mut *queue)
        };
        if queued.is_empty() {
            return;
        }
        let mut later = Vec::new();
        for (dim, pos, push) in queued {
            if dim != dimension {
                later.push((dim, pos, push));
                continue;
            }
            match push {
                Push::Steam => push_steam(host, level, pos),
                Push::Energy => push_energy(host, level, pos),
            }
        }
        if !later.is_empty() {
            PUSH.lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .extend(later);
        }
    }
}

fn dimension_id(dimension: mod_api::LevelDimension) -> Option<String> {
    use mod_api::LevelDimension as L;
    match dimension {
        L::Overworld => Some("minecraft:overworld".to_owned()),
        L::Nether => Some("minecraft:the_nether".to_owned()),
        L::End => Some("minecraft:the_end".to_owned()),
        L::Custom(_) => None,
    }
}

fn item_id(host: &dyn mod_api::Host, item: ItemKey) -> String {
    item.id(host)
}

fn block_pos(argument: Option<&CommandArgumentValue>) -> Option<[i32; 3]> {
    match argument {
        Some(CommandArgumentValue::BlockPos(pos)) => Some(*pos),
        _ => None,
    }
}

fn feedback(host: &mut dyn ServerHost, line: String) {
    let _ = host.commands().feedback(Text::literal(line));
}

/// The engine's fuel value (`minecraft:cooking_fuel.burn_time`), 0 if none.
fn fuel_burn_time(host: &dyn mod_api::Host, stack: &ItemStack) -> u32 {
    host.item_burn_time(stack.item())
        .unwrap_or(0)
        .max(0)
        .try_into()
        .unwrap_or(0)
}

fn say(host: &mut dyn ServerHost, player: PlayerRef, line: String) {
    let _ = player.action_bar(host, Text::literal(line));
}

// ---------------------------------------------------------------------------
// The machine part: placement, ticking and the two block-entity capabilities
// ---------------------------------------------------------------------------

/// The part every machine block and block entity shares. A Part is attached to
/// one block entity type, so the block and its entity ask the same object.
#[derive(Clone, Copy)]
struct MachinePart;

/// Which machine a block entity is, copied out of its id before the tick
/// borrows the entity mutably.
#[derive(Clone, Copy)]
enum Kind {
    Boiler,
    Turbine,
    Macerator,
}

#[mod_api::horizon_part]
impl BlockPart for MachinePart {
    /// A placed machine faces the player, as GT's `setPlacedBy` gives it:
    /// `Direction.fromYRot(yaw).getOpposite()`.
    fn state_for_placement(
        &self,
        level: &dyn mod_api::LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let facing = facing_for_yaw(event.context.yaw);
        level
            .state_with_property(event.state, FACING, facing)
            .map(Some)
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for MachinePart {
    /// All three machines tick on the server (`EntityBlock.getTicker`).
    fn block_entity_ticks(&self, _: &dyn mod_api::Host, event: &mut BlockEntityTicksEvent<'_>) {
        if is_machine(event.block) {
            event.ticks = true;
        }
    }

    /// One server tick of a machine.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let pos = event.pos;
        let level = ticking_level(host);
        let kind = match event.entity.id() {
            BOILER => Some(Kind::Boiler),
            TURBINE => Some(Kind::Turbine),
            MACERATOR => Some(Kind::Macerator),
            _ => None,
        };
        match kind {
            Some(Kind::Boiler) => tick_boiler(host, level, pos, &mut event.entity),
            Some(Kind::Turbine) => tick_turbine(host, level, pos, &mut event.entity),
            Some(Kind::Macerator) => tick_macerator(host, level, pos, &mut event.entity),
            None => {}
        }
    }
}

fn facing_for_yaw(yaw: f32) -> &'static str {
    // Vanilla: 0 is south, 90 west, 180 north, 270 east; the front is the opposite.
    let quarter = ((f64::from(yaw) / 90.0).round() as i32).rem_euclid(4);
    match quarter {
        0 => "north",
        1 => "east",
        2 => "south",
        _ => "west",
    }
}

// ---------------------------------------------------------------------------
// The boiler
// ---------------------------------------------------------------------------

fn boiler_from(nbt: &Compound) -> Boiler {
    let tanks = read_tanks(nbt, 2);
    Boiler {
        temperature: read_int(nbt, KEY_TEMPERATURE).max(0) as u32,
        burn: read_int(nbt, KEY_BURN).max(0) as u32,
        burn_total: read_int(nbt, KEY_BURN_TOTAL).max(0) as u32,
        water_mb: tank_amount(&tanks, BOILER_WATER, WATER),
        steam_mb: tank_amount(&tanks, BOILER_STEAM, STEAM),
        ticks: read_int(nbt, KEY_TICKS).max(0) as u32,
        cooldown: read_int(nbt, KEY_COOLDOWN).max(0) as u32,
        dry: read_bool(nbt, KEY_DRY),
    }
    .clamped()
}

fn store_boiler(nbt: &mut Compound, boiler: &Boiler) {
    write_int(nbt, KEY_TEMPERATURE, boiler.temperature as i32);
    write_int(nbt, KEY_BURN, boiler.burn as i32);
    write_int(nbt, KEY_BURN_TOTAL, boiler.burn_total as i32);
    write_int(nbt, KEY_TICKS, boiler.ticks as i32);
    write_int(nbt, KEY_COOLDOWN, boiler.cooldown as i32);
    write_int(nbt, KEY_DRY, i32::from(boiler.dry));
    let water = if boiler.water_mb > 0 {
        Some(WATER.to_owned())
    } else {
        None
    };
    let steam = if boiler.steam_mb > 0 {
        Some(STEAM.to_owned())
    } else {
        None
    };
    write_tanks(
        nbt,
        &[
            (water, boiler.water_mb.clamp(0, WATER_TANK_MB)),
            (steam, boiler.steam_mb.clamp(0, STEAM_TANK_MB)),
        ],
    );
}

/// One tick of the boiler: load the fuel slot when idle, heat, make steam.
fn tick_boiler(
    host: &mut dyn ServerHost,
    level: Option<LevelRef>,
    pos: [i32; 3],
    entity: &mut mod_api::ModBlockEntityView<'_>,
) {
    let mut boiler = boiler_from(&entity.nbt);
    if !boiler.burning()
        && let Some(stack) = entity.items.get(BOILER_FUEL).and_then(Option::clone)
    {
        let burn = burn_ticks_for(fuel_burn_time(host, &stack));
        if boiler.ignite(burn) {
            consume_fuel(host, entity, &stack);
        }
    }
    let before = boiler;
    let step = boiler.tick();
    store_boiler(&mut entity.nbt, &boiler);
    if let Some(level) = level {
        switch_status(host, level, pos, boiler.burning());
    }
    if step.exploded {
        if let Some(level) = level {
            explode(host, level, pos);
        }
        return;
    }
    if before.steam_mb != boiler.steam_mb
        && let Some(level) = level
    {
        queue_push(level, pos, Push::Steam);
    }
}

/// `StackUtil.shrinkForContainer`: one item is consumed and a container item
/// (a lava bucket's bucket) takes its place when the stack runs out.
fn consume_fuel(
    host: &dyn mod_api::Host,
    entity: &mut mod_api::ModBlockEntityView<'_>,
    stack: &ItemStack,
) {
    let remainder = host.item_crafting_remainder(stack);
    let left = stack.count() - 1;
    entity.items[BOILER_FUEL] = if left > 0 {
        let mut kept = stack.clone();
        kept.set_count(left).ok().map(|()| kept)
    } else {
        remainder
    };
}

/// `IExplosionMachine.doExplosion`: the block goes first, then the blast.
fn explode(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    if let Some(air) = host.block_state_id("minecraft:air") {
        let _ = level.set_block(
            host,
            pos,
            air,
            BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
        );
    }
    let center = [
        f64::from(pos[0]) + 0.5,
        f64::from(pos[1]) + 0.5,
        f64::from(pos[2]) + 0.5,
    ];
    let _ = level.explode(
        host,
        None,
        center,
        runtime::EXPLOSION_POWER,
        false,
        ExplosionInteraction::Block,
        ExplosionSettings::default(),
    );
}

// ---------------------------------------------------------------------------
// The steam turbine
// ---------------------------------------------------------------------------

fn turbine_from(nbt: &Compound) -> Turbine {
    let tanks = read_tanks(nbt, 2);
    Turbine {
        steam_mb: tank_amount(&tanks, TURBINE_STEAM, STEAM),
        energy: read_int(nbt, KEY_ENERGY),
        progress: read_int(nbt, KEY_PROGRESS).max(0) as u32,
        water_mb: tank_amount(&tanks, TURBINE_WATER, DISTILLED_WATER),
    }
    .clamped()
}

fn store_turbine(nbt: &mut Compound, turbine: &Turbine) {
    write_int(nbt, KEY_ENERGY, turbine.energy);
    write_int(nbt, KEY_PROGRESS, turbine.progress as i32);
    let steam = if turbine.steam_mb > 0 {
        Some(STEAM.to_owned())
    } else {
        None
    };
    let water = if turbine.water_mb > 0 {
        Some(DISTILLED_WATER.to_owned())
    } else {
        None
    };
    write_tanks(
        nbt,
        &[
            (steam, turbine.steam_mb.clamp(0, TURBINE_STEAM_TANK_MB)),
            (water, turbine.water_mb.clamp(0, TURBINE_OUTPUT_TANK_MB)),
        ],
    );
}

/// One tick of the turbine: burn steam into energy, 32 EU a tick.
fn tick_turbine(
    host: &mut dyn ServerHost,
    level: Option<LevelRef>,
    pos: [i32; 3],
    entity: &mut mod_api::ModBlockEntityView<'_>,
) {
    let mut turbine = turbine_from(&entity.nbt);
    let _ = turbine.tick();
    store_turbine(&mut entity.nbt, &turbine);
    if let Some(level) = level {
        switch_status(host, level, pos, turbine.running());
        if turbine.energy > 0 {
            queue_push(level, pos, Push::Energy);
        }
    }
}

// ---------------------------------------------------------------------------
// The macerator
// ---------------------------------------------------------------------------

fn macerator_from(nbt: &Compound) -> Macerator {
    Macerator {
        energy: read_int(nbt, KEY_ENERGY),
        progress: read_int(nbt, KEY_PROGRESS).max(0) as u32,
    }
    .clamped()
}

fn store_macerator(nbt: &mut Compound, macerator: &Macerator) {
    write_int(nbt, KEY_ENERGY, macerator.energy);
    write_int(nbt, KEY_PROGRESS, macerator.progress as i32);
}

/// The input item the recipe table knows, and whether its output fits.
fn macerator_input(
    host: &dyn mod_api::Host,
    entity: &mod_api::ModBlockEntityView<'_>,
) -> Option<ItemKey> {
    let stack = entity.items.get(MACERATOR_INPUT).and_then(Option::as_ref)?;
    let result = macerator_output(&stack.item().id(host))?;
    let result_key = host.item_key(&Identifier::parse_namespaced(result)?)?;
    let limit = host.max_stack_size(result_key).unwrap_or(64);
    let fits = match entity.items.get(MACERATOR_OUTPUT).and_then(Option::as_ref) {
        None => true,
        Some(out) => out.item() == result_key && out.count() + MACERATOR_CRUSHED <= limit,
    };
    fits.then_some(result_key)
}

/// One tick of the macerator: spend 2 EU a tick for 400 ticks, then two crushed
/// ore out of one ore in.
fn tick_macerator(
    host: &mut dyn ServerHost,
    level: Option<LevelRef>,
    pos: [i32; 3],
    entity: &mut mod_api::ModBlockEntityView<'_>,
) {
    let result = macerator_input(host, entity);
    let mut macerator = macerator_from(&entity.nbt);
    let step = macerator.tick(result.is_some());
    if step == MaceratorStep::Finished
        && let Some(result) = result
    {
        finish_maceration(host, entity, result);
    }
    store_macerator(&mut entity.nbt, &macerator);
    if let Some(level) = level {
        switch_status(host, level, pos, macerator.progress > 0);
    }
}

fn finish_maceration(
    host: &dyn mod_api::Host,
    entity: &mut mod_api::ModBlockEntityView<'_>,
    result: ItemKey,
) {
    let limit = host.max_stack_size(result).unwrap_or(64);
    let merged = match entity.items.get(MACERATOR_OUTPUT).and_then(Option::clone) {
        Some(mut stack) => {
            let count = stack.count().saturating_add(MACERATOR_CRUSHED).min(limit);
            stack.set_count(count).ok();
            Some(stack)
        }
        None => ItemStack::new(result, MACERATOR_CRUSHED.min(limit)),
    };
    entity.items[MACERATOR_OUTPUT] = merged;
    if let Some(input) = entity.items.get_mut(MACERATOR_INPUT).and_then(Option::take) {
        let left = input.count().saturating_sub(1);
        entity.items[MACERATOR_INPUT] = ItemStack::new(input.item(), left);
    }
}

// ---------------------------------------------------------------------------
// The status property and the push queue
// ---------------------------------------------------------------------------

/// `recipe_logic_status`, GT's own "is it running" property, which switches the
/// model between the idle and the active overlay.
fn switch_status(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], working: bool) {
    let wanted = if working { "working" } else { "idle" };
    if level.block_property(host, pos, STATUS).as_deref() == Some(wanted) {
        return;
    }
    host.set_block_property(pos, STATUS, wanted);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Push {
    Steam,
    Energy,
}

static PUSH: Mutex<Vec<(String, [i32; 3], Push)>> = Mutex::new(Vec::new());

fn queue_push(level: LevelRef, pos: [i32; 3], push: Push) {
    let dimension = level.dimension_id().to_owned();
    let mut queue = PUSH.lock().unwrap_or_else(|poison| poison.into_inner());
    if !queue
        .iter()
        .any(|(dim, at, kind)| dim == &dimension && at == &pos && kind == &push)
    {
        queue.push((dimension, pos, push));
    }
}

/// The face a machine's `facing` names, for the boiler's front-face exclusion.
fn front_face(host: &dyn mod_api::Host, level: LevelRef, pos: [i32; 3]) -> Option<Direction> {
    match level.block_property(host, pos, FACING).as_deref() {
        Some("north") => Some(Direction::North),
        Some("south") => Some(Direction::South),
        Some("east") => Some(Direction::East),
        Some("west") => Some(Direction::West),
        _ => None,
    }
}

/// `SteamBoilerMachine.autoOutput`: every side but the front and the down one.
fn push_steam(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    if block_at(host, level, pos).as_deref() != Some(BOILER) {
        return;
    }
    let front = front_face(host, level, pos);
    let Some(from) = entity_data(host, level, pos) else {
        return;
    };
    let mut from_tanks = read_tanks(&from, 2);
    let have = tank_amount(&from_tanks, BOILER_STEAM, STEAM);
    if have <= 0 {
        return;
    }
    for face in Direction::ALL {
        if face == Direction::Down || Some(face) == front {
            continue;
        }
        let normal = face.normal();
        let target = [pos[0] + normal[0], pos[1] + normal[1], pos[2] + normal[2]];
        if block_at(host, level, target).as_deref() != Some(TURBINE) {
            continue;
        }
        let Some(to) = entity_data(host, level, target) else {
            continue;
        };
        let mut to_tanks = read_tanks(&to, 2);
        let room = (TURBINE_STEAM_TANK_MB - tank_amount(&to_tanks, TURBINE_STEAM, STEAM)).max(0);
        let moved = have.min(room);
        if moved <= 0 {
            continue;
        }
        set_tank(&mut from_tanks, BOILER_STEAM, STEAM, have - moved);
        let now = tank_amount(&to_tanks, TURBINE_STEAM, STEAM) + moved;
        set_tank(&mut to_tanks, TURBINE_STEAM, STEAM, now);
        let mut from_data = from.clone();
        write_tanks(&mut from_data, &from_tanks);
        let mut to_data = to.clone();
        write_tanks(&mut to_data, &to_tanks);
        merge_data(host, level, pos, from_data);
        merge_data(host, level, target, to_data);
        return;
    }
}

/// `NotifiableEnergyContainer`'s push: one amp of LV into each neighbour.
fn push_energy(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    if block_at(host, level, pos).as_deref() != Some(TURBINE) {
        return;
    }
    let Some(from) = entity_data(host, level, pos) else {
        return;
    };
    let have = read_int(&from, KEY_ENERGY);
    if have <= 0 {
        return;
    }
    for face in Direction::ALL {
        let normal = face.normal();
        let target = [pos[0] + normal[0], pos[1] + normal[1], pos[2] + normal[2]];
        if block_at(host, level, target).as_deref() != Some(MACERATOR) {
            continue;
        }
        let Some(to) = entity_data(host, level, target) else {
            continue;
        };
        let room = (MACERATOR_CAPACITY - read_int(&to, KEY_ENERGY)).max(0);
        let moved = have.min(TURBINE_MAX_TRANSFER).min(room);
        if moved <= 0 {
            continue;
        }
        let mut from_data = from.clone();
        write_int(&mut from_data, KEY_ENERGY, have - moved);
        let mut to_data = to.clone();
        write_int(&mut to_data, KEY_ENERGY, read_int(&to, KEY_ENERGY) + moved);
        merge_data(host, level, pos, from_data);
        merge_data(host, level, target, to_data);
        return;
    }
}

// ---------------------------------------------------------------------------
// Clicks: inserting fuel or ore
// ---------------------------------------------------------------------------

/// Put up to a stack of `held` into `slot`, taking what went in from the
/// player's hand (none in creative); what went in.
fn insert(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    slot: usize,
    player: PlayerRef,
    hand: Hand,
    held: &ItemStack,
) -> u32 {
    let resource = ItemResource::of(held);
    let target = ItemPlace::BlockEntity {
        level,
        pos,
        side: None,
        slot,
    };
    let source = ItemPlace::PlayerHand {
        player: player.uuid(),
        hand,
    };
    let creative = player.is_creative(host).unwrap_or(false);
    host.transaction(|host, tx| {
        let moved = target.insert(host, tx, &resource, held.count());
        if moved > 0 && !creative {
            let _ = source.extract(host, tx, &resource, moved);
        }
        Ok::<u32, ()>(moved)
    })
    .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// `/gtceu`
// ---------------------------------------------------------------------------

/// The one-line state the e2e parses, per machine. `tag` is echoed so a script
/// can tell this reply from an earlier one for the same block.
fn status_text(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], tag: &str) -> String {
    let tag = if tag.is_empty() {
        String::new()
    } else {
        format!(" tag={tag}")
    };
    let at = format!("{tag} x={} y={} z={}", pos[0], pos[1], pos[2]);
    let Some(name) = block_at(host, level, pos) else {
        return format!("gtceu nothing{at}");
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return format!("gtceu nothing{at}");
    };
    let Some(data) = entity.data(host) else {
        return format!("gtceu nothing{at}");
    };
    match name.as_str() {
        BOILER => {
            let boiler = boiler_from(&data);
            format!(
                "gtceu boiler {at} burn={}/{} temp={}/{} water={}/{} steam={}/{} dry={}",
                boiler.burn,
                boiler.burn_total,
                boiler.temperature,
                MAX_TEMPERATURE,
                boiler.water_mb,
                WATER_TANK_MB,
                boiler.steam_mb,
                STEAM_TANK_MB,
                i32::from(boiler.dry),
            )
        }
        TURBINE => {
            let turbine = turbine_from(&data);
            format!(
                "gtceu turbine {at} steam={}/{} energy={}/{} progress={}/{} water={}/{}",
                turbine.steam_mb,
                TURBINE_STEAM_TANK_MB,
                turbine.energy,
                TURBINE_CAPACITY,
                turbine.progress,
                TURBINE_DURATION,
                turbine.water_mb,
                TURBINE_OUTPUT_TANK_MB,
            )
        }
        MACERATOR => {
            let macerator = macerator_from(&data);
            let input = slot_text(host, &entity, MACERATOR_INPUT);
            let output = slot_text(host, &entity, MACERATOR_OUTPUT);
            format!(
                "gtceu macerator {at} energy={}/{} progress={}/{} eu/t={} input={} output={}",
                macerator.energy,
                MACERATOR_CAPACITY,
                macerator.progress,
                MACERATOR_DURATION,
                MACERATOR_EUT,
                input,
                output,
            )
        }
        _ => format!("gtceu nothing{at}"),
    }
}

fn slot_text(host: &mut dyn ServerHost, entity: &mod_api::BlockEntityRef, slot: usize) -> String {
    match entity.item_slot(host, slot as u32) {
        Ok(Some(stack)) => format!("{}:{}", item_id(host, stack.item()), stack.count()),
        _ => "none".to_owned(),
    }
}

/// A machine's output slot into the player's inventory (`/gtceu take`).
fn take_output(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    player: Option<PlayerRef>,
) -> String {
    let nothing = format!("gtceu nothing x={} y={} z={}", pos[0], pos[1], pos[2]);
    if block_at(host, level, pos).as_deref() != Some(MACERATOR) {
        return nothing;
    }
    let Some(handler) = level.capability(host, &mod_api::ITEMS, pos, None) else {
        return "gtceu took none".to_owned();
    };
    let Some(resource) = handler.resource(host, MACERATOR_OUTPUT) else {
        return "gtceu took none".to_owned();
    };
    let amount = handler.amount(host, MACERATOR_OUTPUT);
    let moved = host
        .transaction(|host, tx| {
            Ok::<u32, ()>(handler.extract(host, tx, MACERATOR_OUTPUT, &resource, amount))
        })
        .unwrap_or(0);
    if moved == 0 {
        return "gtceu took none".to_owned();
    }
    let id = item_id(host, resource.item());
    if let Some(player) = player
        && let Some(stack) = resource.to_stack(moved)
    {
        let _ = player.give_stack(host, stack);
    }
    format!("gtceu took {id} x{moved}")
}

/// Fill a machine's input tank from outside (`/gtceu fill`): the pipe a player
/// would put there, not a bucket GT would accept (the solid boiler takes no
/// container, only a fluid handler against it).
fn fill_tank(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], amount: i32) -> String {
    let nothing = format!("gtceu nothing x={} y={} z={}", pos[0], pos[1], pos[2]);
    let Some(name) = block_at(host, level, pos) else {
        return nothing;
    };
    let (fluid, index, capacity) = match name.as_str() {
        BOILER => (WATER, BOILER_WATER, WATER_TANK_MB),
        TURBINE => (STEAM, TURBINE_STEAM, TURBINE_STEAM_TANK_MB),
        _ => return nothing,
    };
    let Some(mut data) = entity_data(host, level, pos) else {
        return nothing;
    };
    let mut tanks = read_tanks(&data, 2);
    let room = (capacity - tank_amount(&tanks, index, fluid)).max(0);
    let moved = amount.max(0).min(room);
    if moved > 0 {
        let now = tank_amount(&tanks, index, fluid) + moved;
        set_tank(&mut tanks, index, fluid, now);
        write_tanks(&mut data, &tanks);
        merge_data(host, level, pos, data);
    }
    format!("gtceu filled {fluid} x{moved}")
}
