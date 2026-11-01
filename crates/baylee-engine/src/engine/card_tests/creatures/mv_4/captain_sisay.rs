//! Captain Sisay — {2}{G}{W} — Legendary Creature — Human Soldier
//! Oracle: {T}: Search your library for a legendary card, reveal that card, put it into your hand, then shuffle.

use super::*;
use baylee_core::generated::index;

#[test]
fn captain_sisay_taps_to_search_library_for_a_legendary_card_into_hand() {
    let sisay = card_index("2e7a9ea9-f76c-4c12-950f-c613fa16cfa8");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, index::LIN_SIVVI_DEFIANT_HERO)
        .battlefield(0, &[forest(), forest(), plains(), plains()])
        .hand(0, &[sisay])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, sisay);
    pass_until(&mut engine, stack_is_empty);
    let sisay_id = on_battlefield(&engine, p0, sisay).expect("Captain Sisay on battlefield");
    assert_eq!(pt(&engine, sisay_id), (2, 2));

    // Advance to seat 0's next turn so summoning sickness wears off.
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && stack_is_empty(e)
    });

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sisay_id,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search choice");
    };
    assert!(!options.is_empty());
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
        hand_before + 1
    );
    assert!(in_hand(&engine, p0, index::LIN_SIVVI_DEFIANT_HERO).is_some());
}
