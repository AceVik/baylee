//! `cards/instants/mv_5/fall_of_the_gavel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "5ad33099-0392-4f68-a4c5-7fa3ef2d473e"
fn fall_of_the_gavel() -> CardIndex {
    card_index("5ad33099-0392-4f68-a4c5-7fa3ef2d473e")
}

/// Fall of the Gavel: "Counter target spell. You gain 5 life."
#[test]
fn fall_of_the_gavel_counters_a_spell_and_gains_five() {
    let lands = [
        vec![mountain(); 3],
        vec![plains(), island(), forest(), forest(), forest()],
    ]
    .concat();
    let mut engine = table(917, &[gray_ogre(), fall_of_the_gavel()], &lands, &[], &[]);
    let ogre = ogre_on_the_stack(&mut engine);
    assert_eq!(life_of(&engine, me()), 20);

    tap_all_mana(&mut engine, me());
    cast_with_floating(&mut engine, me(), fall_of_the_gavel());
    let menu = aim_at(&mut engine, me(), ogre);
    assert!(menu.contains(&ogre), "{menu:?}");
    settle(&mut engine);

    assert!(
        in_graveyard(&engine, me(), gray_ogre()).is_some(),
        "countered"
    );
    assert!(on_battlefield(&engine, me(), gray_ogre()).is_none());
    assert_eq!(life_of(&engine, me()), 25, "five life");
}
