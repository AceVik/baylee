//! `cards/enchantments/mv_5/enchanted_evening.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn enchanted_evening() -> CardIndex {
    card_index("c56ea5ce-c8de-46c2-a8dd-bdd41467a28c")
}

/// "All permanents are enchantments in addition to their other types."
/// Lands and creatures of both players gain the type and keep their own.
#[test]
fn enchanted_evening_makes_every_permanent_an_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[enchanted_evening()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[gray_ogre(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let island_id = on_battlefield(&engine, p0, island()).expect("my Island");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest");
    assert!(!types(&engine, mine).contains(TypeSet::ENCHANTMENT));

    cast_from_hand(&mut engine, p0, enchanted_evening());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, enchanted_evening()).is_some()
    });
    for creature in [mine, theirs] {
        let t = types(&engine, creature);
        assert!(t.contains(TypeSet::ENCHANTMENT), "an enchantment now");
        assert!(t.contains(TypeSet::CREATURE), "and still a creature");
    }
    for land in [island_id, their_forest] {
        let t = types(&engine, land);
        assert!(t.contains(TypeSet::ENCHANTMENT), "an enchantment now");
        assert!(t.contains(TypeSet::LAND), "and still a land");
    }
}
