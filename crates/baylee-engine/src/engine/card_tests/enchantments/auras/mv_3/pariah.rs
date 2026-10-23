#[allow(clippy::wildcard_imports)]
use super::*;

fn pariah() -> CardIndex {
    card_index("2c5c8250-1860-42a1-a335-071f54830d37")
}

#[test]
fn pariah_redirects_damage_dealt_to_you_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[pariah()])
        .battlefield(0, &[plains(), plains(), plains(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, pariah());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, opp_bear);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(walk_to_own_main(&mut engine, p1));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(opp_bear, baylee_core::ids::Defender::Player(p0))],
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
        e.state().turn.step == crate::turn::Step::End
    });

    assert_eq!(engine.state().players[0].life, 20);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
}
