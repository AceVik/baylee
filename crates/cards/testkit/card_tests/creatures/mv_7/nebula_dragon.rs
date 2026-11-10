//! `cards/creatures/mv_7/nebula_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn nebula_dragon() -> CardIndex {
    card_index("724b4a59-9c69-4f2b-a436-72183c62f1ff")
}

/// Nebula Dragon — a 4/4 flier for {6}{R}: "it deals 3 damage to any target".
/// Pointed at the opponent it takes 3.
#[test]
fn nebula_dragon_burns_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&lots(mountain(), 7), &[nebula_dragon()], &[]);
    pay_exactly_and_cast(&mut engine, nebula_dragon());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let (_, players) = target_menu(&engine);
    assert!(players.contains(&p1) && players.contains(&p0), "any target");
    point_at(&mut engine, p0, &[], &[p1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 1), 17);
    let dragon = on_battlefield(&engine, p0, nebula_dragon()).expect("out");
    assert_body(&engine, dragon, (4, 4), KeywordSet::FLYING);
}

/// Pointed at a creature (any target includes creatures) it deals 3 to it.
#[test]
fn nebula_dragon_burns_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&lots(mountain(), 7), &[nebula_dragon()], &[gray_ogre()]);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    pay_exactly_and_cast(&mut engine, nebula_dragon());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&ogre));
    point_at(&mut engine, p0, &[ogre], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert_eq!(life_of(&engine, 1), 20, "the player was spared");
}
