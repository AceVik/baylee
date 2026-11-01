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

/// Walks to the declare-attackers question and sends `attacker` at
/// `defender`; every other creature stays home.
#[track_caller]
fn attack_with(engine: &mut Engine<RegistryLookup>, attacker: ObjectId, defender: PlayerId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert!(
        attackers.contains(&attacker),
        "the creature may attack: {attackers:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(defender))],
            },
        )
        .expect("the attacker came off the list");
}

mod aether_flash;
mod aether_sting;
mod aether_storm;
mod an_zerrin_ruins;
mod aquatic_incursion;
mod arboria;
mod awakening;
mod barbed_foliage;
mod bedlam;
mod blood_mist;
mod breath_of_dreams;
mod breeding_pit;
mod brink_of_madness;
mod builder_s_blessing;
mod burning_earth;
mod call_of_the_wild;
mod castle;
mod caverns_of_despair;
mod chaos_moon;
mod coastal_piracy;
mod conversion;
mod crucible_of_fire;
mod cyclone;
mod dampening_pulse;
mod daxos_s_torment;
mod day_of_destiny;
mod death_pit_offering;
mod defense_of_the_heart;
mod delusions_of_mediocrity;
mod dogged_pursuit;
mod dream_tides;
mod dreams_of_the_dead;
mod drought;
mod dwarven_armory;
mod efficient_construction;
mod elderfang_venom;
mod embargo;
mod emrakul_s_influence;
mod enduring_renewal;
mod epic_struggle;
mod fervent_charge;
mod field_of_souls;
mod flooded_woodlands;
mod freyalise_s_winds;
mod from_beyond;
mod frozen_aether;
mod glaciers;
mod glare_of_subdual;
mod grasping_shadows;
mod grave_pact;
mod greater_good;
mod greed;
mod gruul_war_chant;
mod hum_of_the_radix;
mod humility;
mod ill_gotten_inheritance;
mod illusions_of_grandeur;
mod infernal_darkness;
mod invoke_prejudice;
mod karma;
mod kyren_negotiations;
mod last_laugh;
mod lethal_vapors;
mod levitation;
mod light_of_day;
mod lightning_cloud;
mod living_lands;
mod living_plane;
mod manabarbs;
mod march_of_the_machines;
mod martyrs_tomb;
mod monsoon;
mod mortal_combat;
mod mortuary;
mod mystic_decree;
mod natural_emergence;
mod nature_s_blessing;
mod night_of_souls_betrayal;
mod no_mercy;
mod opal_titan;
mod opposition;
mod orcish_oriflamme;
mod parallax_tide;
mod parallax_wave;
mod path_of_annihilation;
mod pegasus_refuge;
mod pendrell_mists;
mod pestilence;
mod pillar_tombs_of_aku;
mod powerstone_minefield;
mod pride_of_the_perfect;
mod primal_order;
mod pursuit_of_knowledge;
mod pyrohemia;
mod reckless_assault;
mod reconnaissance_mission;
mod royal_decree;
mod sarkhan_s_unsealing;
mod security_detail;
mod serra_aviary;
mod serra_s_liturgy;
mod shimmer;
mod smoldering_tar;
mod smothering_tithe;
mod snake_pit;
mod spirit_mirror;
mod spiritual_asylum;
mod squeeze;
mod statecraft;
mod stimulus_package;
mod storm_the_vault;
mod strands_of_night;
mod tainted_aether;
mod test_of_endurance;
mod the_abyss;
mod tooth_and_claw;
mod total_war;
mod treasure_trove;
mod vance_s_blasting_cannons;
mod vernal_bloom;
mod vigilante_justice;
mod volrath_s_dungeon;
mod whirlwind_of_thought;
mod worship;
