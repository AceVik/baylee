#[allow(clippy::wildcard_imports)]
use super::*;

fn no_mercy() -> CardIndex {
    card_index("b9538d53-480b-481a-abbf-83ab17e1a45b")
}

#[test]
fn no_mercy_destroys_creature_that_deals_damage_to_you() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[no_mercy()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, baylee_core::ids::Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[0].life, 18);
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
}
