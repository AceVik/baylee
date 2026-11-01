//! `cards/creatures/artifacts/mv_3/watchdog.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn watchdog() -> CardIndex {
    card_index("6a951d2d-fc5a-4f41-bd77-3e301b76cf47")
}

/// `Watchdog` ({3}, 1/2 Artifact Creature — Dog, `Coverage::Implemented`):
/// `This creature blocks each combat if able.`
/// `As long as this creature is untapped, all creatures attacking you get
/// -1/-0.`
#[test]
fn watchdog_reduces_attackers_power_while_untapped_and_must_block_if_able() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[watchdog()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let dog = on_battlefield(&engine, p0, watchdog()).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
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
    // While Watchdog is untapped, attacking Grizzly Bears gets -1/-0 (becomes 1/2).
    assert_eq!(pt(&engine, bears), (1, 2));
    // Watchdog must block if able: declaring no blockers is illegal.
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(dog, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    // Both 1/2 creatures survive 1 combat damage.
    assert!(on_battlefield(&engine, p0, watchdog()).is_some());
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
}

#[test]
fn tapped_watchdog_neither_weakens_attackers_nor_has_to_block() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[watchdog()], &[GRIZZLY_BEARS], &[], &[]);
    let dog = on_battlefield(&e, p0, watchdog()).unwrap();
    let bears = on_battlefield(&e, p1, GRIZZLY_BEARS).unwrap();
    lk::attack(&mut e, &[dog]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    reach_their_main_phase(&mut e, p1);
    lk::attack(&mut e, &[bears]);
    assert!(is_tapped(&e, dog));
    assert_eq!(pt(&e, bears), (2, 2));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(e.state().players[0].life, 18);
}
