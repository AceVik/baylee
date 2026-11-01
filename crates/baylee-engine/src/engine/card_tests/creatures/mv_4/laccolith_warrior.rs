//! `cards/creatures/mv_4/laccolith_warrior.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn laccolith_warrior_deals_three_damage_to_target_creature_and_assigns_no_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[card_index("bf957e0b-3070-4a85-a63d-3dadd7364d53")])
        .battlefield(1, &[index::WALL_OF_WOOD, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let warrior = on_battlefield(
        &engine,
        p0,
        card_index("bf957e0b-3070-4a85-a63d-3dadd7364d53"),
    )
    .expect("Laccolith Warrior");
    let wall = on_battlefield(&engine, p1, index::WALL_OF_WOOD).expect("Wall of Wood");
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).expect("Hill Giant");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(warrior, Defender::Player(p1))],
            },
        )
        .expect("declare attackers");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, warrior)],
            },
        )
        .expect("declare blockers");
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("choose to deal damage");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(on_battlefield(&engine, p1, index::HILL_GIANT), None);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        on_battlefield(&engine, p1, index::WALL_OF_WOOD),
        Some(wall),
        "Wall of Wood survives because Laccolith Warrior assigned no combat damage"
    );
}
