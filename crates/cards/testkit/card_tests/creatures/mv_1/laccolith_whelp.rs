//! `cards/creatures/mv_1/laccolith_whelp.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn laccolith_whelp_deals_one_damage_to_target_creature_and_assigns_no_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[card_index("b16b10ec-a6a3-4bda-a561-9b7bb8eee194")])
        .battlefield(1, &[index::LLANOWAR_ELVES, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let whelp = on_battlefield(
        &engine,
        p0,
        card_index("b16b10ec-a6a3-4bda-a561-9b7bb8eee194"),
    )
    .expect("Laccolith Whelp");
    let elves = on_battlefield(&engine, p1, index::LLANOWAR_ELVES).expect("Llanowar Elves");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(whelp, Defender::Player(p1))],
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
                blockers: vec![(bears, whelp)],
            },
        )
        .expect("declare blockers");
    aim_at(&mut engine, p0, elves);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("choose to deal damage");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(on_battlefield(&engine, p1, index::LLANOWAR_ELVES), None);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS),
        Some(bears),
        "blocker takes no combat damage from Laccolith Whelp"
    );
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
}
