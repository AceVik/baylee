//! `cards/creatures/mv_3/elvish_healer.rs`, played.
//!
//! Oracle:
//! - "{T}: Prevent the next 1 damage that would be dealt to any target this turn. If it's a green creature, prevent the next 2 damage instead."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "ea46835c-9dac-4e1e-8338-ad99136f511a"
fn elvish_healer() -> CardIndex {
    card_index("ea46835c-9dac-4e1e-8338-ad99136f511a")
}

#[test]
fn elvish_healer_prevents_two_damage_to_green_creature_and_one_to_other_targets() {
    // Case 1: Target a green creature (2/2 Grizzly Bears) -> prevents 2 of Lightning Bolt's 3 damage, so Bears survives with 1 damage marked!
    let mut engine = lk::game(
        SEED,
        &[elvish_healer(), index::GRIZZLY_BEARS],
        &[],
        &[index::LIGHTNING_BOLT],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    activate(&mut engine, P0, elvish_healer(), 0);
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "green 2/2 creature survives 3 damage because Elvish Healer prevented 2"
    );
    assert_eq!(lk::damage_on(&engine, bears), 1);

    // Case 2: Target a player -> prevents 1 of Lightning Bolt's 3 damage (player takes 2).
    let mut engine2 = lk::game(SEED, &[elvish_healer()], &[], &[index::LIGHTNING_BOLT], &[]);

    activate(&mut engine2, P0, elvish_healer(), 0);
    lk::aim(&mut engine2, &[], &[P0]);
    pass_until(&mut engine2, stack_is_empty);

    lk::float(&mut engine2, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine2, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine2, &[], &[P0]);
    pass_until(&mut engine2, stack_is_empty);

    assert_eq!(
        engine2.state().players[0].life,
        18,
        "non-green-creature target had 1 of the 3 damage prevented"
    );
}
