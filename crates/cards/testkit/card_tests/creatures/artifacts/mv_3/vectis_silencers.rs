//! `cards/creatures/artifacts/mv_3/vectis_silencers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vectis_silencers() -> CardIndex {
    card_index("e132bba0-40c5-4095-99a3-02cad520ca99")
}

/// Vectis Silencers — "{2}{B}: This creature gains deathtouch until end of
/// turn." With it, 1 damage kills a 0/6 blocker.
#[test]
fn vectis_silencers_kills_a_big_blocker_with_deathtouch() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Guardians of Meletis, a 0/6 defender.
    let wall = card_index("7604aac7-82ae-4f5b-888d-a008754f716a");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vectis_silencers(), swamp(), forest(), forest()])
        .battlefield(1, &[wall])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let silencers = on_battlefield(&engine, p0, vectis_silencers()).expect("seated");
    let blocker = on_battlefield(&engine, p1, wall).expect("seated");
    assert_eq!(pt(&engine, silencers), (1, 2));
    assert!(!keywords(&engine, silencers).contains(KeywordSet::DEATHTOUCH));

    attack_to_blocks(&mut engine, &[silencers]);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, silencers)],
            },
        )
        .unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, vectis_silencers(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, silencers).contains(KeywordSet::DEATHTOUCH));
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(
        in_graveyard(&engine, p1, wall).is_some(),
        "1 damage with deathtouch is lethal"
    );
    assert!(on_battlefield(&engine, p0, vectis_silencers()).is_some());
}

/// The control: without the activation, 1 damage does not kill a 0/6.
#[test]
fn vectis_silencers_without_deathtouch_does_not_kill_a_big_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let wall = card_index("7604aac7-82ae-4f5b-888d-a008754f716a");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vectis_silencers()])
        .battlefield(1, &[wall])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let silencers = on_battlefield(&engine, p0, vectis_silencers()).expect("seated");
    let blocker = on_battlefield(&engine, p1, wall).expect("seated");

    attack_to_blocks(&mut engine, &[silencers]);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, silencers)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(on_battlefield(&engine, p1, wall).is_some());
}
