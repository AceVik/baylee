//! Library of Lat-Nam — `{4}{U}` Sorcery (Alliances)
//!
//! (`Coverage::Partial`: having an opponent choose the mode (CR 601.2b) is not
//! supported; the caster chooses between drawing three cards at the beginning
//! of the next turn's upkeep or searching their library for a card, putting it
//! into their hand, and shuffling.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn library_of_lat_nam() -> CardIndex {
    card_index("67f68a0f-bd3d-47ba-ad99-95c1a2acd170")
}

#[test]
fn library_of_lat_nam_mode_0_draws_three_next_upkeep_and_mode_1_searches_library_to_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[library_of_lat_nam(), library_of_lat_nam()])
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mode 0: draw three cards at the beginning of the next turn's upkeep.
    let lib1 = in_hand(&engine, p0, library_of_lat_nam()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: lib1 })
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 3
    );

    // Next turn, Mode 1: search your library for a card, put it into your hand, then shuffle.
    reach_their_main_phase(&mut engine, p0);
    let hand_before_mode_1 = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lib2 = in_hand(&engine, p0, library_of_lat_nam()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: lib2 })
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected search library choice");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before_mode_1,
        "cast 1 card and tutored 1 card to hand"
    );
}
