//! `cards/enchantments/mv_4/browse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn browse() -> CardIndex {
    card_index("49491601-e272-4603-9348-185a2953bd4b")
}

/// `Browse` ({2}{U}{U}, Enchantment, `Coverage::Partial`) activates for
/// `{2}{U}{U}` to look at the top five cards of your library, put one of them
/// into your hand, and put the rest on the bottom of your library.
#[test]
fn browse_looks_at_top_five_and_puts_one_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 8])
        .hand(0, &[browse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, browse());
    pass_until(&mut engine, stack_is_empty);
    let br = on_battlefield(&engine, p0, browse()).expect("Browse enters");

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: br,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 5);
    let picked = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![picked],
            },
        )
        .unwrap();
    if let Pending::Arrange { cards, .. } = engine.pending().clone() {
        assert_eq!(cards.len(), 4);
        engine
            .apply(p0, PlayerAction::Arrange { piles: vec![cards] })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(picked).unwrap().zone, Zone::Hand);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
