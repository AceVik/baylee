#[allow(clippy::wildcard_imports)]
use super::*;

fn fog_bank() -> CardIndex {
    card_index("3a2bb47d-228d-499e-a34b-9e10d99e9b2c")
}

#[test]
fn fog_bank_prevents_combat_damage_to_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[baylee_core::generated::index::PHANTOM_MONSTER])
        .battlefield(1, &[fog_bank()])
        .hand(0, &[lightning_bolt()])
        .hand(1, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, baylee_core::generated::index::PHANTOM_MONSTER)
        .expect("seated");
    let bank = on_battlefield(&engine, p1, fog_bank()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bank, bear)],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    crate::engine::card_tests::legends_kit::float(&mut engine, p1, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut engine, p1, giant_growth());
    aim_at(&mut engine, p1, bank);
    pass_until(&mut engine, stack_is_empty);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert_eq!(
        engine.state().object(bear).unwrap().damage,
        0,
        "a pumped Fog Bank still deals no combat damage"
    );
    assert_eq!(engine.state().object(bank).unwrap().damage, 0);
    reach_their_main_phase(&mut engine, p1);
    let offered = crate::engine::card_tests::legends_kit::attackers_offered(&mut engine);
    assert!(!offered.contains(&bank), "defender cannot attack");
    engine
        .apply(p1, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    reach_their_main_phase(&mut engine, p0);
    crate::engine::card_tests::legends_kit::float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, bank);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, fog_bank()).is_none(),
        "noncombat damage is not prevented"
    );
}
