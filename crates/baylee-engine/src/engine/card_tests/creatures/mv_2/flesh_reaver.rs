#[allow(clippy::wildcard_imports)]
use super::*;

fn flesh_reaver() -> CardIndex {
    card_index("a143fdfe-f1aa-4eab-8c81-8c72a4dc1695")
}

#[test]
fn flesh_reaver_deals_damage_to_you_when_it_damages_opponent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[flesh_reaver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let reaver = on_battlefield(&engine, p0, flesh_reaver()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(reaver, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| e.state().players[0].life == 16);

    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(engine.state().players[0].life, 16);
}
