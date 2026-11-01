//! The tests of the cards under `cards/creatures/mv_8/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A duel at seat 0's first main phase: `board` already on its battlefield
/// and `hand` in its hand.
#[track_caller]
fn at_main(board: &[CardIndex], hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "seat 0 reaches a main phase of its own"
    );
    engine
}

/// Seat 0 taps everything and casts `card`; the stack settles and the
/// permanent it became is returned.
#[track_caller]
fn cast_and_settle(engine: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    let p0 = PlayerId::new(0);
    cast_from_hand(engine, p0, card);
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, p0, card).expect("the spell resolved onto the battlefield")
}

/// Seat 0 taps everything it has and still cannot cast `card`: it stays in
/// hand and nothing is on the stack.
#[track_caller]
fn cannot_cast(engine: &mut Engine<RegistryLookup>, card: CardIndex) {
    let p0 = PlayerId::new(0);
    tap_all_mana(engine, p0);
    let spell = in_hand(engine, p0, card).expect("the card is in hand");
    let _ = engine.apply(p0, PlayerAction::CastSpell { card: spell });
    assert!(
        in_hand(engine, p0, card).is_some(),
        "one mana short, the cast must not go through"
    );
    assert!(stack_is_empty(engine), "and nothing is on the stack");
}

/// Seat 0 declares `attacker` against seat 1 and the engine reaches the
/// blockers question, whose options are returned.
#[track_caller]
fn attack_player(
    engine: &mut Engine<RegistryLookup>,
    attacker: ObjectId,
) -> Vec<crate::choice::BlockOption> {
    attack_and_collect_blocks(engine, attacker, PlayerId::new(1))
}

/// Seat 1 declares no blocks and play runs on until the stack is empty in
/// the second main phase.
#[track_caller]
fn no_blocks_and_settle(engine: &mut Engine<RegistryLookup>) {
    engine
        .apply(
            PlayerId::new(1),
            PlayerAction::DeclareBlockers { blockers: vec![] },
        )
        .expect("declaring no blockers is always legal");
    pass_until(engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && stack_is_empty(e)
    });
}

fn life_of(engine: &Engine<RegistryLookup>, seat: usize) -> i32 {
    engine.state().players[seat].life
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: u8) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(seat)))
        .len()
}
mod akroma_angel_of_wrath;
mod ancient_brontodon;
mod ashen_firebeast;
mod avacyn_angel_of_hope;
mod avatar_of_fury;
mod avatar_of_hope;
mod avatar_of_might;
mod avatar_of_will;
mod avatar_of_woe;
mod axelrod_gunnarson;
mod baleful_force;
mod baron_sengir;
mod benthic_behemoth;
mod bloodfire_colossus;
mod brimstone_dragon;
mod celestial_force;
mod chromium;
mod copper_host_crusher;
mod crash_of_rhinos;
mod deep_spawn;
mod devouring_strossus;
mod eldrazi_devastator;
mod gigantomancer;
mod godsire;
mod goliath_spider;
mod griselbrand;
mod hearthcage_giant;
mod hellkite_overlord;
mod living_hive;
mod maelstrom_wanderer;
mod magmatic_force;
mod pestilence_demon;
mod piru_the_volatile;
mod razaketh_the_foulblooded;
mod scaled_wurm;
mod scion_of_darkness;
mod silver_seraph;
mod skeletal_wurm;
mod symbiotic_wurm;
mod tidal_force;
mod tidal_kraken;
mod tidespout_tyrant;
mod tunneler_wurm;
mod utvara_hellkite;
mod vampiric_dragon;
mod verdant_force;
mod zetalpa_primal_dawn;
