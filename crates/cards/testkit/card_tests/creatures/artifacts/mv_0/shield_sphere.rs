//! `cards/creatures/artifacts/mv_0/shield_sphere.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn shield_sphere() -> CardIndex {
    card_index("df9ec9b0-be22-4136-923b-11a71ca4b6c2")
}

/// Shield Sphere ({0} 0/6 Artifact Creature — Wall):
/// - Defender
/// - "Whenever this creature blocks, put a -0/-1 counter on it."
#[test]
fn shield_sphere_has_defender_and_gets_minus_zero_minus_one_counter_when_blocking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shield_sphere()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let sphere = on_battlefield(&engine, p0, shield_sphere()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, sphere).contains(KeywordSet::DEFENDER));
    assert_eq!(pt(&engine, sphere), (0, 6));

    // P1 attacks with Grizzly Bears; Shield Sphere blocks and gets a -0/-1 counter (becoming 0/5).
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(sphere, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine
            .state()
            .object(sphere)
            .unwrap()
            .counters
            .get(CounterKind::Minus {
                power: 0,
                toughness: 1,
            }),
        1
    );
    assert_eq!(pt(&engine, sphere), (0, 5));
}
