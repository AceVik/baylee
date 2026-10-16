//! `cards/artifacts/mv_4/anchovy_banana_pizza.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn anchovy_banana_pizza() -> CardIndex {
    card_index("5f4834c7-cd63-420b-8492-ef127ffd4f45")
}

/// "When this artifact enters, destroy target creature." The target is a
/// choice, so the Ogre is named while the Elves stand beside it untouched.
#[test]
fn entering_destroys_the_target_creature_and_only_that_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(); 4])
        .battlefield(1, &[gray_ogre(), llanowar_elves()])
        .hand(0, &[anchovy_banana_pizza()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("the Ogre is out");

    cast_from_hand(&mut engine, p0, anchovy_banana_pizza());
    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&ogre),
        "a creature is a legal target: {options:?}"
    );
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "the target was destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and only the target"
    );
    assert!(
        on_battlefield(&engine, p0, anchovy_banana_pizza()).is_some(),
        "the Pizza itself stays"
    );
}

/// "{2}, {T}, Sacrifice this artifact: You gain 3 life."
#[test]
fn the_pizza_is_eaten_for_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[anchovy_banana_pizza(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let before = life_of(&engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, anchovy_banana_pizza(), 1);
    assert!(
        in_graveyard(&engine, p0, anchovy_banana_pizza()).is_some(),
        "the sacrifice is a cost, paid on activation"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), before + 3, "three life");
}
