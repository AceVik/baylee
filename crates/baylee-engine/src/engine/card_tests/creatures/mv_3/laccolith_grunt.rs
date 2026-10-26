//! `cards/creatures/mv_3/laccolith_grunt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn laccolith_grunt_deals_power_damage_to_target_creature_and_assigns_no_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[card_index("f973daa7-241e-4eb7-9d98-9ea7557caa3a")])
        .battlefield(1, &[index::LLANOWAR_ELVES, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let grunt = on_battlefield(
        &engine,
        p0,
        card_index("f973daa7-241e-4eb7-9d98-9ea7557caa3a"),
    )
    .expect("Laccolith Grunt");
    let elves = on_battlefield(&engine, p1, index::LLANOWAR_ELVES).expect("Llanowar Elves");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(grunt, Defender::Player(p1))],
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
                blockers: vec![(elves, grunt)],
            },
        )
        .expect("declare blockers");
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("choose to deal damage");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS), None);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        on_battlefield(&engine, p1, index::LLANOWAR_ELVES),
        Some(elves),
        "1/1 blocker survives because Laccolith Grunt assigned no combat damage"
    );
}
