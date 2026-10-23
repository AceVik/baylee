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
        .battlefield(0, &[grizzly_bears()])
        .battlefield(1, &[fog_bank()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
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

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });

    assert!(on_battlefield(&engine, p1, fog_bank()).is_some());
}
