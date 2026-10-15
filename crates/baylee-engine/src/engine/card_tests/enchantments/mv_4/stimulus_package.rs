//! `cards/enchantments/mv_4/stimulus_package.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn stimulus_package() -> CardIndex {
    card_index("ce76d17e-d536-46f0-98e2-3e1b1b4f61a4")
}

/// "When this enchantment enters, create two Treasure tokens." and
/// "Sacrifice a Treasure: Create a 1/1 green and white Citizen creature
/// token.": two Treasures on entry, one is spent for a Citizen.
#[test]
fn stimulus_package_makes_two_treasures_and_trades_one_for_a_citizen() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[stimulus_package()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Red, 2);
    float(&mut engine, p0, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p0, stimulus_package());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(treasures(&engine, p0).len(), 2, "two Treasures");

    activate(&mut engine, p0, stimulus_package(), 1);
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert_eq!(options.len(), 2, "either Treasure may be the cost");
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .expect("a Treasure was on the menu");
    }
    assert_eq!(
        treasures(&engine, p0).len(),
        1,
        "one Treasure was sacrificed"
    );
    pass_until(&mut engine, stack_is_empty);
    let citizens: Vec<_> = tokens_of(&engine, p0)
        .into_iter()
        .filter(|id| !treasures(&engine, p0).contains(id))
        .collect();
    assert_eq!(citizens.len(), 1, "one Citizen");
    let chars = engine
        .state()
        .object(citizens[0])
        .expect("token")
        .characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(1), Some(1)));
    assert!(chars.colors.contains(Color::Green) && chars.colors.contains(Color::White));
}
