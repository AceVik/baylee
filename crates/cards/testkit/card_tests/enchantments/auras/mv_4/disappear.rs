//! `cards/enchantments/auras/mv_4/disappear.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn disappear() -> CardIndex {
    card_index("039e2e39-8f7f-43f2-8448-de35ef7b8dc1")
}

#[test]
fn disappear_returns_enchanted_creature_and_itself_to_owners_hands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[disappear()])
        .battlefield(
            0,
            &[
                gray_ogre(),
                llanowar_elves(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, disappear());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, disappear(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, gray_ogre()).is_some());
    assert!(in_hand(&engine, p0, disappear()).is_some());
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, PlayerId::new(1), gray_ogre()).is_some());
}
