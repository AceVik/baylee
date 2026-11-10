//! `cards/creatures/mv_7/cat_gator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn cat_gator() -> CardIndex {
    card_index("349e6b9e-40a8-49bc-9cd3-4a22f5363fa5")
}

/// Cat-Gator — a 3/2 lifelinker for {6}{B}. "When this creature enters, it
/// deals damage equal to the number of Swamps you control to any target": with
/// seven Swamps, 7 to the opponent — and the opponent's own Swamps do not
/// count.
#[test]
fn cat_gator_deals_damage_equal_to_the_swamps_it_controls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&lots(swamp(), 7), &[cat_gator()], &lots(swamp(), 3));
    pay_exactly_and_cast(&mut engine, cat_gator());
    let (_, players) = {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        target_menu(&engine)
    };
    assert!(players.contains(&p1), "any target includes a player");
    point_at(&mut engine, p0, &[], &[p1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 1), 13, "7 Swamps of its controller");
    let gator = on_battlefield(&engine, p0, cat_gator()).expect("resolved");
    assert_body(&engine, gator, (3, 2), KeywordSet::LIFELINK);
}

/// Lifelink: 3 damage dealt by the Cat-Gator is 3 life for its controller.
#[test]
fn cat_gator_gains_life_for_the_damage_it_deals() {
    let (mut engine, gator, _ogre) = duel_vs_ogre(cat_gator(), &[]);
    fight(&mut engine, &[gator], &[]);
    assert_eq!(life_of(&engine, 1), 17);
    assert_eq!(life_of(&engine, 0), 23, "lifelink");
}
