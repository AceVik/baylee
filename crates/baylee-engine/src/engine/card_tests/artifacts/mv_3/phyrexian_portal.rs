//! `cards/artifacts/mv_3/phyrexian_portal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn phyrexian_portal() -> CardIndex {
    card_index("1a3ce329-ee48-4abc-b820-b43e9e68eae9")
}

/// Phyrexian Portal ({3} Artifact, Partial):
/// "{3}" looks at the top ten cards of your library, puts one into your hand, and shuffles the rest into your library.
#[test]
fn phyrexian_portal_picks_one_card_from_top_ten_into_hand_and_shuffles() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[phyrexian_portal(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let portal = on_battlefield(&engine, p0, phyrexian_portal()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: portal,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    assert_eq!(options.len(), 10);
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    assert_eq!(engine.state().object(chosen).unwrap().zone, Zone::Hand);
}
