//! `cards/enchantments/mv_3/still_life.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Still Life` is an enchantment under `Coverage::Implemented` with an activated ability costing `{G}{G}`.
/// When activated, it becomes a 4/3 Centaur creature in addition to its other types until end of turn.
/// After cycling to the next turn, the continuous animation effect expires and it reverts to a noncreature enchantment.
#[test]
fn still_life_becomes_a_four_three_centaur_creature_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), still_life()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment = on_battlefield(&engine, p0, still_life()).expect("Still Life deployed");
    assert!(
        types(&engine, enchantment).contains(TypeSet::ENCHANTMENT),
        "Still Life is an enchantment"
    );
    assert!(
        !types(&engine, enchantment).contains(TypeSet::CREATURE),
        "Still Life is not a creature before activation"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "two Forests provide {{G}}{{G}}"
    );

    activate(&mut engine, p0, still_life(), 0);
    pass_until(&mut engine, stack_is_empty);

    let animated_types = types(&engine, enchantment);
    assert!(
        animated_types.contains(TypeSet::CREATURE),
        "Still Life gained the creature type"
    );
    assert!(
        animated_types.contains(TypeSet::ENCHANTMENT),
        "Still Life retains the enchantment type"
    );
    assert_eq!(
        pt(&engine, enchantment),
        (4, 3),
        "Still Life is a 4/3 creature"
    );

    // End of turn duration expires on the next turn.
    reach_their_main_phase(&mut engine, p1);
    let expired_types = types(&engine, enchantment);
    assert!(
        !expired_types.contains(TypeSet::CREATURE),
        "Still Life is no longer a creature after the turn ends"
    );
    assert!(
        expired_types.contains(TypeSet::ENCHANTMENT),
        "Still Life remains an enchantment"
    );
}
