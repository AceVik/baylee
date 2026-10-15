//! The tests of the cards under `cards/enchantments/mv_4/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit::float;
use crate::turn::Step;

/// Rebuilds the projections after a test rewrote the board behind the
/// engine's back (a tap, a destroyed permanent), as the layers would have.
fn resync(engine: &mut Engine<RegistryLookup>) {
    engine.state.invalidate_projections();
    engine.sync_static_effects();
    engine.refresh_offer();
}

/// `seat`'s life total.
fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// Passes until `seat`'s `step` has a trigger waiting on the stack: the
/// question "what does it do" is then read off the stack, not off a pass.
#[track_caller]
fn walk_to_step_trigger(engine: &mut Engine<RegistryLookup>, seat: PlayerId, step: Step) {
    pass_until(engine, |e| {
        e.state().turn.active == seat && e.state().turn.step == step && !stack_is_empty(e)
    });
}

/// Passes until somebody is asked whether to pay mana (an "unless you pay"
/// upkeep cost) and says who was asked, for how much.
#[track_caller]
fn walk_to_price(engine: &mut Engine<RegistryLookup>) -> (PlayerId, baylee_core::mana::ManaCost) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    (player, cost)
}

/// Says yes to the price, makes mana out of the lands `want` accepts, closes
/// the payment window and lets whatever follows resolve.
#[track_caller]
fn pay_the_price_with(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    want: impl Fn(ObjectId) -> bool,
) {
    engine
        .apply(seat, PlayerAction::YesNo(true))
        .expect("the question can be answered");
    tap_mana_where(engine, seat, want);
    engine
        .apply(seat, PlayerAction::PassPriority)
        .expect("closing the payment window");
    pass_until(engine, stack_is_empty);
}

mod aether_sting;
mod arboria;
mod breeding_pit;
mod builder_s_blessing;
mod burning_earth;
mod castle;
mod conversion;
mod crucible_of_fire;
mod cyclone;
mod dampening_pulse;
mod day_of_destiny;
mod dogged_pursuit;
mod efficient_construction;
mod embargo;
mod emrakul_s_influence;
mod field_of_souls;
mod from_beyond;
mod glaciers;
mod grasping_shadows;
mod greed;
mod ill_gotten_inheritance;
mod invoke_prejudice;
mod karma;
mod last_laugh;
mod levitation;
mod living_lands;
mod living_plane;
mod manabarbs;
mod mystic_decree;
mod night_of_souls_betrayal;
mod opposition;
mod orcish_oriflamme;
mod path_of_annihilation;
mod pegasus_refuge;
mod pestilence;
mod pride_of_the_perfect;
mod primal_order;
mod pyrohemia;
mod reckless_assault;
mod sarkhan_s_unsealing;
mod serra_aviary;
mod smothering_tithe;
mod snake_pit;
mod spirit_mirror;
mod spiritual_asylum;
mod storm_the_vault;
mod the_abyss;
mod treasure_trove;
mod vance_s_blasting_cannons;
mod vernal_bloom;
mod whirlwind_of_thought;
