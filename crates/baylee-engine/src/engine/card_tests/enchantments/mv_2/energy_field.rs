#[allow(clippy::wildcard_imports)]
use super::*;

fn energy_field() -> CardIndex {
    card_index("fbf7a7f4-9915-4a53-84b3-e55e9f487323")
}

#[test]
fn energy_field_prevents_opponent_damage_and_sacrifices_when_card_enters_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[energy_field(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    let attacker = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, baylee_core::ids::Defender::Player(p0))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p1));

    assert_eq!(engine.state().players[0].life, 20);
    assert!(on_battlefield(&engine, p0, energy_field()).is_some());

    let p0_bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    kill(&mut engine, p0_bear);
    assert!(on_battlefield(&engine, p0, energy_field()).is_none());
}
