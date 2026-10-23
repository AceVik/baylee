//! Koskun Falls — `{2}{B}{B}` World Enchantment (Homelands)
//!
//! At the beginning of your upkeep, sacrifice this enchantment unless you tap
//! an untapped creature you control.
//! Creatures can't attack you unless their controller pays `{2}` for each
//! creature they control that's attacking you.
//!
//! (`Coverage::Partial`: only the upkeep sentence is played. The attack tax is
//! left off, because `CantAttackUnlessPayMana` taxes every attack by the
//! creatures it names and not only the attacks at you.)

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn koskun_falls() -> CardIndex {
    card_index("a9b9e798-debc-4f2c-9770-e16f670082f7")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// P0 casts Koskun Falls in the first main phase of turn 1, with `mine` on
/// the battlefield beside four Swamps and `theirs` across the table. Returns
/// the engine with the Falls out, still in that main phase.
fn falls_cast(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut lands = vec![swamp(), swamp(), swamp(), swamp()];
    lands.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[koskun_falls()])
        .battlefield(0, &lands)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let falls = in_hand(&engine, P0, koskun_falls()).unwrap();
    tap_all_mana(&mut engine, P0);
    engine
        .apply(P0, PlayerAction::CastSpell { card: falls })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, koskun_falls()).is_some());
    engine
}

/// The question P0's next upkeep asks: which untapped creature to tap.
fn upkeep_question(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == P0),
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    options
}

/// "Unless you tap an untapped creature you control": tapping one keeps the
/// enchantment, and only one of your own creatures is on the menu.
#[test]
fn koskun_falls_stays_when_its_controller_taps_a_creature_in_their_upkeep() {
    let mut engine = falls_cast(&[bears()], &[bears()]);
    let mine = on_battlefield(&engine, P0, bears()).unwrap();
    let theirs = on_battlefield(&engine, P1, bears()).unwrap();

    // Their upkeep asks P0 nothing: the Falls is still out in P1's main phase.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        on_battlefield(&engine, P0, koskun_falls()).is_some(),
        "the opponent's upkeep is not yours"
    );

    let options = upkeep_question(&mut engine);
    assert!(options.contains(&mine));
    assert!(
        !options.contains(&theirs),
        "only a creature you control can be tapped"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(on_battlefield(&engine, P0, koskun_falls()).is_some());
    assert!(is_tapped(&engine, mine), "the creature paid by tapping");
}

/// Declining the tap, with a creature available, sacrifices it.
#[test]
fn koskun_falls_is_sacrificed_when_its_controller_declines_to_tap() {
    let mut engine = falls_cast(&[bears()], &[]);
    let mine = on_battlefield(&engine, P0, bears()).unwrap();

    let options = upkeep_question(&mut engine);
    assert!(options.contains(&mine), "the tap was on offer");
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(on_battlefield(&engine, P0, koskun_falls()).is_none());
    assert!(in_graveyard(&engine, P0, koskun_falls()).is_some());
    assert!(!is_tapped(&engine, mine), "nothing was tapped");
}

/// With no creature to tap there is nothing to ask and the Falls goes.
#[test]
fn koskun_falls_is_sacrificed_in_the_upkeep_without_a_creature_to_tap() {
    let mut engine = falls_cast(&[], &[bears()]);

    // The walk starts in P0's own first main phase, so it goes through P1's
    // whole turn first (nothing is asked there either) and stops in the main
    // phase of P0's next turn, after the upkeep with nothing to tap.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        on_battlefield(&engine, P0, koskun_falls()).is_some(),
        "the opponent's upkeep is not yours"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(on_battlefield(&engine, P0, koskun_falls()).is_none());
    assert!(in_graveyard(&engine, P0, koskun_falls()).is_some());
}
