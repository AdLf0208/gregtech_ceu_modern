// Pure GregTech CEu arithmetic (standalone crate lib, injected as
// `crate::game::gtceu`). No game access lives here: the solid boiler's heat,
// burn and steam, the steam turbine's operation and energy, the LV macerator's
// progress and cost, the one recipe the slice carries, and the two adjacent
// transfers.
//
// Every number below is upstream's own, read from GregTechCEu/GregTech-Modern
// at tag `v7.0.2-1.21.1` (commit f6200e0):
//
// - `SteamBoilerMachine`: steam tank 16 buckets (:98), water tank 16 buckets
//   (:102), max temperature 500 at low pressure (:245-247), +1 degree every 24
//   ticks while burning (:174-182), -1 every 45 ticks while cooling
//   (:236-243), steam every 10 ticks above 100 degrees
//   (fill = base * temperature / max / 2, :192-201), 1 mB of water a fill
//   (:195), an explosion at 2.0 when water returns after a dry spell
//   (:202-206), 4 buckets vented into a full steam tank (:227), and steam
//   pushed to adjacent fluid handlers every 5 ticks (:139-157).
// - `ConfigHolder`: `smallBoilers.solidBoilerBaseOutput` 120 (:577).
// - `SteamBoilerLogic` + `SmallBoilerLogic`: the fuel is any item with a
//   smelting burn time, and the small boiler's operation lasts burnTime * 12
//   ticks (:19-21).
// - `FuelRecipes`: the steam turbine's fuel is 640 mB steam -> 4 mB distilled
//   water, 10 ticks, -32 EU/t (:154-159).
// - `WorkableTieredMachine`: an LV machine's buffer is V[LV] * 64 = 2,048 EU
//   (:88-99), one amp, V[LV] = 32 (`GTValues`:45-46).
// - `GTMachineUtils`: the steam turbine's tank at LV is 16 buckets (:116-117).
// - `OreRecipeHandler`: the macerator's ore recipe is 1 ore -> 2 crushed ore,
//   EUt 2, duration 400 (:118-135), with no byproduct for iron
//   (`OreProperty.getOreByProduct` returns NULL; no material sets one).
//
// See `../../FINDINGS.md` for the file-and-line record and the arithmetic.

/// `SteamBoilerMachine.createWaterTank`: 16 buckets, in.
pub const WATER_TANK_MB: i32 = 16_000;
/// `SteamBoilerMachine.createSteamTank`: 16 buckets, out.
pub const STEAM_TANK_MB: i32 = 16_000;
/// `SteamBoilerMachine.getMaxTemperature()` at low pressure.
pub const MAX_TEMPERATURE: u32 = 500;
/// `ConfigHolder.machines.smallBoilers.solidBoilerBaseOutput`, mB a second.
pub const BASE_STEAM_OUTPUT: u32 = 120;
/// `SteamBoilerMachine.updateCurrentTemperature`: steam starts at 100 degrees.
pub const MIN_STEAM_TEMPERATURE: u32 = 100;
/// The `getOffsetTimer() % 10 == 0` gate on one steam step.
pub const STEAM_INTERVAL: u32 = 10;
/// The `getOffsetTimer() % 24 == 0` gate on one degree at low pressure.
pub const HEAT_INTERVAL: u32 = 24;
/// `SteamBoilerMachine.getCooldownInterval()` at low pressure.
pub const COOLDOWN_INTERVAL: u32 = 45;
/// `SteamBoilerMachine.getCoolDownRate()`: one degree a cooldown.
pub const COOLDOWN_RATE: u32 = 1;
/// `SmallBoilerLogic.modifyBurnTime`: a fuel's smelting burn time times twelve.
pub const BURN_MULTIPLIER: u32 = 12;
/// The 4 buckets `SteamBoilerMachine` vents into a full steam tank.
pub const VENT_MB: i32 = 4_000;
/// The dry-boil explosion's power (`SteamBoilerMachine` `doExplosion(2.0f)`).
pub const EXPLOSION_POWER: f32 = 2.0;

/// The steam turbine's steam tank at LV (`steamGeneratorTankSizeFunction`).
pub const TURBINE_STEAM_TANK_MB: i32 = 16_000;
/// Its distilled-water tank: the same scaling function, on the output side.
pub const TURBINE_OUTPUT_TANK_MB: i32 = 16_000;
/// An LV machine's energy buffer: `V[tier] * 64`.
pub const TURBINE_CAPACITY: i32 = 2_048;
/// `FuelRecipes`' steam entry: millibuckets of steam one operation burns.
pub const TURBINE_STEAM_PER_OPERATION: i32 = 640;
/// ... its duration, in ticks.
pub const TURBINE_DURATION: u32 = 10;
/// ... its output: `-EUt(-V[LV])`, 32 EU a tick.
pub const TURBINE_EU_PER_TICK: i32 = 32;
/// ... and its distilled water, in millibuckets.
pub const TURBINE_DISTILLED_WATER_MB: i32 = 4;
/// The most one adjacent transfer moves: one amp at LV.
pub const TURBINE_MAX_TRANSFER: i32 = 32;

/// The LV macerator's energy buffer: `V[tier] * 64`.
pub const MACERATOR_CAPACITY: i32 = 2_048;
/// `OreRecipeHandler`'s ore recipe: EU a tick.
pub const MACERATOR_EUT: i32 = 2;
/// ... its duration, in ticks.
pub const MACERATOR_DURATION: u32 = 400;
/// ... and its guaranteed output: one ore block gives two crushed ore.
pub const MACERATOR_CRUSHED: u32 = 2;

/// The one ore this slice carries, as the block the jar registers.
pub const IRON_ORE: &str = "gtceu:iron_ore";
/// The vanilla block of the same ore: `c:ores/iron` holds both upstream.
pub const VANILLA_IRON_ORE: &str = "minecraft:iron_ore";
/// `TagPrefix.crushed.idPattern("crushed_%s_ore")` for iron.
pub const CRUSHED_IRON_ORE: &str = "gtceu:crushed_iron_ore";

/// What one [`Boiler::tick`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BoilerStep {
    /// Steam that went into the tank this tick.
    pub steam_made: i32,
    /// Whether the water drain of a steam step happened.
    pub water_drained: bool,
    /// Steam vented because the tank was full.
    pub vented_mb: i32,
    /// The dry-boil explosion fired: the block is gone.
    pub exploded: bool,
    /// The temperature moved this tick.
    pub temperature_changed: bool,
}

/// The LP steam solid boiler's saved state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Boiler {
    /// `currentTemperature`, 0 to [`MAX_TEMPERATURE`].
    pub temperature: u32,
    /// Ticks of burning left in the loaded fuel.
    pub burn: u32,
    /// The whole burn this fuel gave, for a progress bar.
    pub burn_total: u32,
    /// Water in the input tank.
    pub water_mb: i32,
    /// Steam in the output tank.
    pub steam_mb: i32,
    /// The machine's own tick counter, upstream's `getOffsetTimer()`.
    pub ticks: u32,
    /// Ticks left before the next cooling step.
    pub cooldown: u32,
    /// Upstream's `hasNoWater`: a steam step found the tank dry.
    pub dry: bool,
}

impl Boiler {
    /// A freshly placed boiler: empty, cold.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the boiler is burning fuel now.
    #[must_use]
    pub fn burning(&self) -> bool {
        self.burn > 0
    }

    /// Load one fuel that burns for `burn_ticks`, as `RecipeLogic.setupRecipe`
    /// does: false while another is burning or the fuel is empty.
    ///
    /// [`burn_ticks_for`] turns a vanilla smelting time into the boiler's.
    pub fn ignite(&mut self, burn_ticks: u32) -> bool {
        if self.burn > 0 || burn_ticks == 0 {
            return false;
        }
        self.burn = burn_ticks;
        self.burn_total = burn_ticks;
        // SteamBoilerMachine.onWorking: a working boiler is at least 1 degree.
        self.temperature = self.temperature.max(1);
        self.cooldown = 0;
        true
    }

    /// One server tick.
    pub fn tick(&mut self) -> BoilerStep {
        let mut step = BoilerStep::default();
        self.ticks = self.ticks.wrapping_add(1);
        let burning = self.burning();
        let before = self.temperature;
        if burning {
            // `getOffsetTimer() % 24 == 0` at low pressure (:174-182).
            if self.ticks.is_multiple_of(HEAT_INTERVAL) && self.temperature < MAX_TEMPERATURE {
                self.temperature += 1;
            }
        } else if self.cooldown > 0 {
            self.cooldown -= 1;
        } else if self.temperature > 0 {
            self.temperature = self.temperature.saturating_sub(COOLDOWN_RATE);
            self.cooldown = COOLDOWN_INTERVAL;
        }
        step.temperature_changed = self.temperature != before;

        if self.ticks.is_multiple_of(STEAM_INTERVAL) {
            if self.temperature >= MIN_STEAM_TEMPERATURE {
                let fill = steam_per_step(self.temperature);
                let drained = self.water_mb >= 1;
                if drained {
                    self.water_mb -= 1;
                    step.water_drained = true;
                    let room = self.steam_room();
                    step.steam_made = fill.min(room);
                    self.steam_mb += step.steam_made;
                    if step.steam_made == 0 {
                        // A full tank is vented rather than filled (:227).
                        step.vented_mb = VENT_MB;
                    }
                }
                if self.dry && drained {
                    // `hasNoWater && hasDrainedWater`: the boiler explodes (:202-206)
                    // and `doExplosion` takes the block away first.
                    step.exploded = true;
                    *self = Self::new();
                    return step;
                }
                self.dry = !drained;
            } else {
                self.dry = false;
            }
        }

        if burning {
            self.burn -= 1;
            if self.burn == 0 {
                // `SteamBoilerMachine.afterWorking`: the cooldown starts with the end
                // of a burn.
                self.cooldown = COOLDOWN_INTERVAL;
            }
        }
        step
    }

    /// Take up to `amount` of steam out of the tank; what came out.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "tested pure API; the glue moves fluids and energy through the engine's own handlers"
        )
    )]
    pub fn take_steam(&mut self, amount: i32) -> i32 {
        let taken = self.steam_mb.min(amount.max(0));
        self.steam_mb -= taken;
        taken
    }

    /// Put up to `amount` of water in; what did not fit.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "tested pure API; the glue moves fluids and energy through the engine's own handlers"
        )
    )]
    pub fn add_water(&mut self, amount: i32) -> i32 {
        let room = (WATER_TANK_MB - self.water_mb).max(0);
        let taken = amount.max(0).min(room);
        self.water_mb += taken;
        amount.max(0) - taken
    }

    /// Free room in the steam tank.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "tested pure API; the glue moves fluids and energy through the engine's own handlers"
        )
    )]
    #[must_use]
    pub fn steam_room(&self) -> i32 {
        (STEAM_TANK_MB - self.steam_mb).max(0)
    }

    /// Bring saved fields into range, for a block entity read from a save.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.temperature = self.temperature.min(MAX_TEMPERATURE);
        self.water_mb = self.water_mb.clamp(0, WATER_TANK_MB);
        self.steam_mb = self.steam_mb.clamp(0, STEAM_TANK_MB);
        self.burn = self.burn.min(self.burn_total);
        self
    }
}

/// What one [`Turbine::tick`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TurbineStep {
    /// An operation started this tick.
    pub started: bool,
    /// EU added to the buffer this tick.
    pub energy_made: i32,
    /// The operation finished: its distilled water is out.
    pub finished: bool,
}

/// The LV steam turbine's saved state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Turbine {
    /// Steam in the input tank.
    pub steam_mb: i32,
    /// Energy in the buffer, 0 to [`TURBINE_CAPACITY`].
    pub energy: i32,
    /// Ticks of the running operation done; 0 when idle.
    pub progress: u32,
    /// Distilled water in the output tank.
    pub water_mb: i32,
}

impl Turbine {
    /// A freshly placed turbine: empty.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "tested pure API; the glue moves fluids and energy through the engine's own handlers"
        )
    )]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether an operation is running.
    #[must_use]
    pub fn running(&self) -> bool {
        self.progress > 0
    }

    /// One server tick.
    pub fn tick(&mut self) -> TurbineStep {
        let mut step = TurbineStep::default();
        if self.progress == 0 {
            // The fuel recipe needs its whole 640 mB and room for its water, and
            // the first tick's EU must fit before the operation starts at all.
            if self.steam_mb < TURBINE_STEAM_PER_OPERATION
                || self.water_mb + TURBINE_DISTILLED_WATER_MB > TURBINE_OUTPUT_TANK_MB
                || self.energy_room() < TURBINE_EU_PER_TICK
            {
                return step;
            }
            self.steam_mb -= TURBINE_STEAM_PER_OPERATION;
            self.progress = 1;
            step.started = true;
        } else if self.energy_room() < TURBINE_EU_PER_TICK {
            // A full buffer holds the operation where it is: its EU cannot be voided.
            return step;
        } else {
            self.progress += 1;
        }
        self.energy += TURBINE_EU_PER_TICK;
        step.energy_made = TURBINE_EU_PER_TICK;
        if self.progress >= TURBINE_DURATION {
            self.progress = 0;
            step.finished = true;
            let room = (TURBINE_OUTPUT_TANK_MB - self.water_mb).max(0);
            self.water_mb += TURBINE_DISTILLED_WATER_MB.min(room);
        }
        step
    }

    /// Take up to `amount` of energy out of the buffer; what came out.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "tested pure API; the glue moves fluids and energy through the engine's own handlers"
        )
    )]
    pub fn take_energy(&mut self, amount: i32) -> i32 {
        let taken = self.energy.min(amount.max(0));
        self.energy -= taken;
        taken
    }

    /// Put up to `amount` of steam in; what did not fit.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "tested pure API; the glue moves fluids and energy through the engine's own handlers"
        )
    )]
    pub fn add_steam(&mut self, amount: i32) -> i32 {
        let room = self.steam_room();
        let taken = amount.max(0).min(room);
        self.steam_mb += taken;
        amount.max(0) - taken
    }

    /// Free room in the steam tank.
    #[must_use]
    pub fn steam_room(&self) -> i32 {
        (TURBINE_STEAM_TANK_MB - self.steam_mb).max(0)
    }

    /// Free room in the energy buffer.
    #[must_use]
    pub fn energy_room(&self) -> i32 {
        (TURBINE_CAPACITY - self.energy).max(0)
    }

    /// Bring saved fields into range.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.steam_mb = self.steam_mb.clamp(0, TURBINE_STEAM_TANK_MB);
        self.energy = self.energy.clamp(0, TURBINE_CAPACITY);
        self.water_mb = self.water_mb.clamp(0, TURBINE_OUTPUT_TANK_MB);
        self.progress = self.progress.min(TURBINE_DURATION + 1);
        self
    }
}

/// What one [`Macerator::tick`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MaceratorStep {
    /// No recipe, or no energy: nothing moved.
    #[default]
    Idle,
    /// One tick of the operation done.
    Working,
    /// The operation finished this tick: consume the input, emit the output.
    Finished,
}

/// The LV macerator's saved state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Macerator {
    /// Energy in the buffer, 0 to [`MACERATOR_CAPACITY`].
    pub energy: i32,
    /// Ticks of the running operation done; 0 when idle.
    pub progress: u32,
}

impl Macerator {
    /// A freshly placed macerator: empty.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "tested pure API; the glue moves fluids and energy through the engine's own handlers"
        )
    )]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// One server tick, with whether the input slot holds a recipe's input.
    pub fn tick(&mut self, has_recipe: bool) -> MaceratorStep {
        if !has_recipe {
            // No input that matches: the operation is gone, not merely waiting.
            self.progress = 0;
            return MaceratorStep::Idle;
        }
        if self.energy < MACERATOR_EUT {
            return MaceratorStep::Idle;
        }
        self.energy -= MACERATOR_EUT;
        self.progress += 1;
        if self.progress >= MACERATOR_DURATION {
            self.progress = 0;
            return MaceratorStep::Finished;
        }
        MaceratorStep::Working
    }

    /// Put up to `amount` of energy in; what did not fit.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "tested pure API; the glue moves fluids and energy through the engine's own handlers"
        )
    )]
    pub fn add_energy(&mut self, amount: i32) -> i32 {
        let room = self.energy_room();
        let taken = amount.max(0).min(room);
        self.energy += taken;
        amount.max(0) - taken
    }

    /// Free room in the buffer.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "tested pure API; the glue moves fluids and energy through the engine's own handlers"
        )
    )]
    #[must_use]
    pub fn energy_room(&self) -> i32 {
        (MACERATOR_CAPACITY - self.energy).max(0)
    }

    /// Bring saved fields into range.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.energy = self.energy.clamp(0, MACERATOR_CAPACITY);
        self.progress = self.progress.min(MACERATOR_DURATION);
        self
    }
}

/// The small boiler's burn for a fuel with this vanilla smelting time.
#[must_use]
pub fn burn_ticks_for(smelting_ticks: u32) -> u32 {
    smelting_ticks.saturating_mul(BURN_MULTIPLIER)
}

/// The steam one steam step at `temperature` makes.
#[must_use]
pub fn steam_per_step(temperature: u32) -> i32 {
    // `(int) (base * ((float) temperature / max) / 2)` — the same single cast.
    (BASE_STEAM_OUTPUT as f32 * (temperature.min(MAX_TEMPERATURE) as f32 / MAX_TEMPERATURE as f32)
        / 2.0) as i32
}

/// The macerator's output for an input item id, from the recipe table this
/// slice carries: the iron ore family, and nothing else.
#[must_use]
pub fn macerator_output(input: &str) -> Option<&'static str> {
    match input {
        IRON_ORE | VANILLA_IRON_ORE => Some(CRUSHED_IRON_ORE),
        _ => None,
    }
}

/// Move up to `max` steam from a boiler's tank into a turbine's; what moved.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "tested pure API; the glue lets the engine move it"
    )
)]
pub fn move_steam(from: &mut Boiler, to: &mut Turbine, max: i32) -> i32 {
    let moved = from.steam_mb.min(max.max(0)).min(to.steam_room()).max(0);
    from.steam_mb -= moved;
    to.steam_mb += moved;
    moved
}

/// Move up to `max` energy from a turbine's buffer into a macerator's.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "tested pure API; the glue lets the engine move it"
    )
)]
pub fn move_energy(from: &mut Turbine, to: &mut Macerator, max: i32) -> i32 {
    let moved = from.energy.min(max.max(0)).min(to.energy_room()).max(0);
    from.energy -= moved;
    to.energy += moved;
    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A boiler with water and one coal's worth of burning.
    fn lit_boiler() -> Boiler {
        let mut boiler = Boiler {
            water_mb: 1_000,
            ..Boiler::new()
        };
        assert!(boiler.ignite(burn_ticks_for(1_600)));
        boiler
    }

    #[test]
    fn a_fresh_boiler_is_cold_and_empty_with_upstream_tanks() {
        let boiler = Boiler::new();
        assert_eq!(boiler.temperature, 0);
        assert_eq!(boiler.water_mb, 0);
        assert_eq!(boiler.steam_mb, 0);
        assert!(!boiler.burning());
        assert_eq!((WATER_TANK_MB, STEAM_TANK_MB), (16_000, 16_000));
        assert_eq!(MAX_TEMPERATURE, 500);
    }

    #[test]
    fn one_coal_burns_for_twelve_times_its_smelting_time() {
        // SmallBoilerLogic.modifyBurnTime: 1600 * 12.
        assert_eq!(burn_ticks_for(1_600), 19_200);
        assert_eq!(burn_ticks_for(0), 0);
        assert_eq!(burn_ticks_for(200), 2_400);
    }

    #[test]
    fn a_boiler_already_burning_takes_no_second_fuel() {
        let mut boiler = lit_boiler();
        assert!(!boiler.ignite(1_600));
        assert_eq!(boiler.burn, 19_200);
        assert_eq!(boiler.burn_total, 19_200);
    }

    #[test]
    fn the_first_burning_tick_lifts_the_temperature_to_one_degree() {
        // SteamBoilerMachine.onWorking: `Math.max(1, currentTemperature)`.
        let boiler = lit_boiler();
        assert_eq!(boiler.temperature, 1);
    }

    #[test]
    fn the_boiler_heats_one_degree_every_twenty_four_ticks() {
        let mut boiler = lit_boiler();
        for _ in 0..24 {
            boiler.tick();
        }
        assert_eq!(boiler.temperature, 2);
        for _ in 0..72 {
            boiler.tick();
        }
        assert_eq!(boiler.temperature, 5, "96 ticks of burning is 5 degrees");
    }

    #[test]
    fn the_boiler_cools_one_degree_every_forty_five_idle_ticks() {
        let mut boiler = Boiler {
            temperature: 5,
            ..Boiler::new()
        };
        for _ in 0..45 {
            boiler.tick();
        }
        assert_eq!(boiler.temperature, 4);
        for _ in 0..45 {
            boiler.tick();
        }
        assert_eq!(boiler.temperature, 3);
    }

    #[test]
    fn no_steam_below_a_hundred_degrees() {
        let mut boiler = lit_boiler();
        // A working boiler starts at 1 degree (`onWorking`), so 2,370 ticks of
        // burning is 99: no steam step has run at 100 yet.
        for _ in 0..2_370 {
            let step = boiler.tick();
            assert_eq!(step.steam_made, 0);
        }
        assert_eq!(boiler.temperature, 99);
        assert_eq!(boiler.steam_mb, 0);
        assert_eq!(boiler.water_mb, 1_000);
    }

    #[test]
    fn steam_starts_at_a_hundred_degrees_twelve_millibuckets_a_step() {
        let mut boiler = lit_boiler();
        for _ in 0..2_379 {
            boiler.tick();
        }
        assert_eq!(boiler.temperature, 100, "1 + 2379 / 24");
        assert_eq!(boiler.steam_mb, 0);
        let step = boiler.tick(); // tick 2380: 2380 % 10 == 0
        assert_eq!(step.steam_made, 12);
        assert!(step.water_drained);
        assert_eq!(boiler.steam_mb, 12);
        assert_eq!(boiler.water_mb, 999);
    }

    #[test]
    fn a_hot_boiler_makes_six_millibuckets_a_tick() {
        assert_eq!(steam_per_step(500), 60);
        assert_eq!(steam_per_step(100), 12);
        assert_eq!(steam_per_step(250), 30);
        assert_eq!(steam_per_step(0), 0);
    }

    #[test]
    fn the_boiler_drains_one_millibucket_of_water_a_steam_step() {
        let mut boiler = Boiler {
            temperature: 500,
            burn: 5_000,
            burn_total: 5_000,
            water_mb: 1_000,
            ticks: 9,
            ..Boiler::new()
        };
        for _ in 0..100 {
            boiler.tick();
        }
        assert_eq!(boiler.water_mb, 990, "100 ticks is ten steam steps");
        assert_eq!(boiler.steam_mb, 600);
        assert_eq!(boiler.temperature, 500, "burning holds the top temperature");
    }

    #[test]
    fn a_dry_boiler_makes_no_steam_and_says_so() {
        let mut boiler = Boiler {
            temperature: 500,
            ..Boiler::new()
        };
        assert!(boiler.ignite(1_600));
        for _ in 0..20 {
            boiler.tick();
        }
        assert_eq!(boiler.steam_mb, 0);
        assert!(boiler.dry);
    }

    #[test]
    fn water_returning_after_a_dry_spell_explodes_the_boiler() {
        let mut boiler = Boiler {
            temperature: 500,
            ticks: 9,
            ..Boiler::new()
        };
        assert!(boiler.ignite(1_600));
        boiler.tick(); // tick 10: a steam step with no water
        assert!(boiler.dry);
        assert_eq!(boiler.add_water(1_000), 0);
        let mut exploded = false;
        for _ in 0..10 {
            exploded |= boiler.tick().exploded; // tick 20: water is back
        }
        assert!(exploded);
        assert_eq!(boiler.temperature, 0, "the block is gone");
        assert!(!boiler.burning());
        assert_eq!(EXPLOSION_POWER, 2.0);
    }

    #[test]
    fn a_full_steam_tank_vents_four_buckets() {
        let mut boiler = Boiler {
            temperature: 500,
            water_mb: 1_000,
            steam_mb: STEAM_TANK_MB,
            ticks: 9,
            ..Boiler::new()
        };
        let step = boiler.tick();
        assert_eq!(step.steam_made, 0);
        assert_eq!(step.vented_mb, 4_000);
        assert_eq!(boiler.steam_mb, STEAM_TANK_MB);
    }

    #[test]
    fn water_goes_in_only_up_to_the_tank() {
        let mut boiler = Boiler::new();
        assert_eq!(boiler.add_water(20_000), 4_000);
        assert_eq!(boiler.water_mb, WATER_TANK_MB);
        assert_eq!(boiler.steam_room(), STEAM_TANK_MB);
    }

    #[test]
    fn a_turbine_operation_is_ten_ticks_of_thirty_two_eu_from_640_steam() {
        let mut turbine = Turbine {
            steam_mb: TURBINE_STEAM_PER_OPERATION,
            ..Turbine::new()
        };
        let mut money = 0;
        let mut finished = false;
        for _ in 0..10 {
            let step = turbine.tick();
            money += step.energy_made;
            finished |= step.finished;
        }
        assert_eq!(turbine.steam_mb, 0);
        assert_eq!(turbine.energy, 320, "32 EU a tick for ten ticks");
        assert_eq!(money, 320);
        assert_eq!(turbine.water_mb, TURBINE_DISTILLED_WATER_MB);
        assert!(finished);
        assert!(!turbine.running());
        assert_eq!(TURBINE_CAPACITY, 2_048);
        assert_eq!(TURBINE_STEAM_TANK_MB, 16_000);
    }

    #[test]
    fn the_turbine_waits_for_a_whole_640_millibuckets() {
        let mut turbine = Turbine {
            steam_mb: TURBINE_STEAM_PER_OPERATION - 1,
            ..Turbine::new()
        };
        for _ in 0..100 {
            assert_eq!(turbine.tick(), TurbineStep::default());
        }
        assert_eq!(turbine.energy, 0);
        assert_eq!(turbine.steam_mb, 639);
    }

    #[test]
    fn the_turbine_stalls_while_its_buffer_cannot_take_a_tick() {
        let mut turbine = Turbine {
            steam_mb: TURBINE_STEAM_PER_OPERATION,
            energy: TURBINE_CAPACITY - TURBINE_EU_PER_TICK,
            ..Turbine::new()
        };
        let first = turbine.tick();
        assert!(first.started);
        assert_eq!(turbine.progress, 1);
        assert_eq!(turbine.energy, TURBINE_CAPACITY);
        // 32 EU more does not fit: the operation holds where it is.
        let next = turbine.tick();
        assert_eq!(next.energy_made, 0);
        assert_eq!(turbine.progress, 1);
        assert_eq!(turbine.energy, TURBINE_CAPACITY);
        // Room again, and it carries on.
        assert_eq!(
            turbine.take_energy(TURBINE_EU_PER_TICK),
            TURBINE_EU_PER_TICK
        );
        assert_eq!(turbine.tick().energy_made, TURBINE_EU_PER_TICK);
        assert_eq!(turbine.progress, 2);
    }

    #[test]
    fn the_turbine_will_not_start_without_room_for_its_water() {
        let mut turbine = Turbine {
            steam_mb: TURBINE_STEAM_PER_OPERATION,
            water_mb: TURBINE_OUTPUT_TANK_MB,
            ..Turbine::new()
        };
        assert!(!turbine.tick().started);
        assert_eq!(turbine.steam_mb, TURBINE_STEAM_PER_OPERATION);
    }

    #[test]
    fn the_turbine_gives_up_only_what_it_holds() {
        let mut turbine = Turbine {
            energy: 100,
            ..Turbine::new()
        };
        assert_eq!(turbine.take_energy(TURBINE_MAX_TRANSFER), 32);
        assert_eq!(turbine.energy, 68);
        assert_eq!(turbine.take_energy(1_000), 68);
        assert_eq!(turbine.take_energy(1), 0);
    }

    #[test]
    fn an_ore_operation_costs_eight_hundred_eu_over_four_hundred_ticks() {
        let mut macerator = Macerator {
            energy: MACERATOR_CAPACITY,
            ..Macerator::new()
        };
        let mut finished = false;
        for _ in 0..MACERATOR_DURATION {
            finished |= macerator.tick(true) == MaceratorStep::Finished;
        }
        assert!(finished);
        assert_eq!(macerator.progress, 0);
        assert_eq!(
            macerator.energy,
            MACERATOR_CAPACITY - MACERATOR_EUT * MACERATOR_DURATION as i32,
            "2 EU a tick for 400 ticks is 800 EU"
        );
        assert_eq!(macerator.energy, 1_248);
    }

    #[test]
    fn the_ore_recipe_gives_two_crushed_ore_and_no_more() {
        assert_eq!(MACERATOR_CRUSHED, 2);
        assert_eq!(MACERATOR_DURATION, 400);
        assert_eq!(MACERATOR_EUT, 2);
    }

    #[test]
    fn the_macerator_does_not_start_without_energy() {
        let mut macerator = Macerator {
            energy: MACERATOR_EUT - 1,
            ..Macerator::new()
        };
        for _ in 0..100 {
            assert_eq!(macerator.tick(true), MaceratorStep::Idle);
        }
        assert_eq!(macerator.progress, 0);
        assert_eq!(macerator.energy, 1);
    }

    #[test]
    fn the_macerator_does_nothing_without_a_recipe() {
        let mut macerator = Macerator {
            energy: MACERATOR_CAPACITY,
            progress: 7,
        };
        assert_eq!(macerator.tick(false), MaceratorStep::Idle);
        assert_eq!(macerator.progress, 0, "no recipe resets the operation");
        assert_eq!(macerator.energy, MACERATOR_CAPACITY);
    }

    #[test]
    fn the_macerator_holds_its_progress_while_the_power_stops() {
        let mut macerator = Macerator {
            energy: 10,
            progress: 0,
        };
        for _ in 0..5 {
            assert_eq!(macerator.tick(true), MaceratorStep::Working);
        }
        assert_eq!(macerator.energy, 0);
        for _ in 0..50 {
            assert_eq!(macerator.tick(true), MaceratorStep::Idle);
        }
        assert_eq!(
            macerator.progress, 5,
            "the operation waits, it does not regress"
        );
    }

    #[test]
    fn the_recipe_table_holds_the_iron_ore_family_only() {
        assert_eq!(macerator_output(IRON_ORE), Some(CRUSHED_IRON_ORE));
        assert_eq!(macerator_output(VANILLA_IRON_ORE), Some(CRUSHED_IRON_ORE));
        assert_eq!(macerator_output("minecraft:gold_ore"), None);
        assert_eq!(macerator_output("minecraft:diamond_block"), None);
        assert_eq!(macerator_output("gtceu:crushed_iron_ore"), None);
    }

    #[test]
    fn steam_moves_up_to_the_turbines_room() {
        let mut boiler = Boiler {
            steam_mb: 1_000,
            ..Boiler::new()
        };
        let mut turbine = Turbine::new();
        assert_eq!(move_steam(&mut boiler, &mut turbine, 600), 600);
        assert_eq!(boiler.steam_mb, 400);
        assert_eq!(turbine.steam_mb, 600);
        turbine.steam_mb = TURBINE_STEAM_TANK_MB - 5;
        assert_eq!(move_steam(&mut boiler, &mut turbine, 600), 5);
        assert_eq!(boiler.steam_mb, 395);
        assert_eq!(turbine.steam_mb, TURBINE_STEAM_TANK_MB);
        assert_eq!(move_steam(&mut boiler, &mut turbine, 600), 0);
    }

    #[test]
    fn energy_moves_up_to_the_macerators_room() {
        let mut turbine = Turbine {
            energy: 1_000,
            ..Turbine::new()
        };
        let mut macerator = Macerator {
            energy: MACERATOR_CAPACITY - 10,
            ..Macerator::new()
        };
        assert_eq!(
            move_energy(&mut turbine, &mut macerator, TURBINE_MAX_TRANSFER),
            10
        );
        assert_eq!(turbine.energy, 990);
        assert_eq!(macerator.energy, MACERATOR_CAPACITY);
        assert_eq!(
            move_energy(&mut turbine, &mut macerator, TURBINE_MAX_TRANSFER),
            0
        );
    }

    #[test]
    fn one_amp_of_lv_is_the_most_a_transfer_moves() {
        let mut turbine = Turbine {
            energy: 1_000,
            ..Turbine::new()
        };
        let mut macerator = Macerator::new();
        assert_eq!(
            move_energy(&mut turbine, &mut macerator, TURBINE_MAX_TRANSFER),
            32
        );
        assert_eq!(TURBINE_MAX_TRANSFER, 32);
    }

    #[test]
    fn a_saved_machine_comes_back_inside_its_limits() {
        let boiler = Boiler {
            temperature: 9_000,
            water_mb: -50,
            steam_mb: 99_999,
            ..Boiler::new()
        }
        .clamped();
        assert_eq!(boiler.temperature, MAX_TEMPERATURE);
        assert_eq!(boiler.water_mb, 0);
        assert_eq!(boiler.steam_mb, STEAM_TANK_MB);
        let macerator = Macerator {
            energy: -1,
            progress: 9_999,
        }
        .clamped();
        assert_eq!(macerator.energy, 0);
        assert_eq!(macerator.progress, MACERATOR_DURATION);
        let turbine = Turbine {
            steam_mb: -1,
            energy: 99_999,
            progress: 99,
            water_mb: -3,
        }
        .clamped();
        assert_eq!(turbine.steam_mb, 0);
        assert_eq!(turbine.energy, TURBINE_CAPACITY);
        assert_eq!(turbine.water_mb, 0);
        assert_eq!(turbine.progress, TURBINE_DURATION + 1);
    }
}
