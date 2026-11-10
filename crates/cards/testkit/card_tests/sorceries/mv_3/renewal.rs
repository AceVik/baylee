//! Renewal — `{2}{G}` Sorcery (Homelands)
//!
//! As an additional cost to cast this spell, sacrifice a land.
//! Search your library for a basic land card, put that card onto the
//! battlefield, then shuffle.
//! Draw a card at the beginning of the next turn's upkeep.

#[allow(clippy::wildcard_imports)]
use super::*;

fn renewal() -> CardIndex {
    card_index("8df46910-ee13-416e-8282-dfe85a90e6d5")
}

#[test]
fn renewal_sacrifices_land_searches_basic_land_to_battlefield_and_draws_next_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[renewal(), treetop_village()])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A nonbasic land on top of the library: "basic land card" skips it.
    let village = hand_to_library_top(&mut engine, p0, treetop_village());
    let spell = in_hand(&engine, p0, renewal()).unwrap();
    let sac_forest = on_battlefield(&engine, p0, forest()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sac_forest],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    // Choose a basic Plains from library to put onto the battlefield.
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        !options.contains(&village),
        "a nonbasic land is not a basic"
    );
    let found_plains = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found_plains],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let plains_on_battlefield = on_battlefield(&engine, p0, plains()).unwrap();
    assert!(
        !is_tapped(&engine, plains_on_battlefield),
        "it enters untapped"
    );
    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(on_battlefield(&engine, p0, treetop_village()).is_none());

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
