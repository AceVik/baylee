//! `cards/artifacts/mv_4/thran_tome.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::THRAN_TOME;

/// `Thran Tome` ({4}, Artifact — Book, Partial) pays `{5}, {T}` targeting an
/// opponent to mill one card and draw two cards.
#[test]
fn thran_tome_mills_one_and_draws_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[THRAN_TOME, plains(), plains(), plains(), plains(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, THRAN_TOME, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy_before + 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
}
