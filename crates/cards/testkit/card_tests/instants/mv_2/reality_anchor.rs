//! `cards/instants/mv_2/reality_anchor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::DAUTHI_GHOUL;

fn reality_anchor() -> CardIndex {
    card_index("a7b4a519-d85f-40ae-bb5f-08bc1d301418")
}

/// `Reality Anchor` ({1}{G}, Instant, `Coverage::Implemented`):
/// `Target creature loses shadow until end of turn.`
/// `Draw a card.`
#[test]
fn reality_anchor_removes_shadow_from_target_creature_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[DAUTHI_GHOUL])
        .hand(0, &[reality_anchor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ghoul = on_battlefield(&engine, p1, DAUTHI_GHOUL).unwrap();
    assert!(
        engine
            .state()
            .object(ghoul)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHADOW)
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, reality_anchor());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ghoul],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(ghoul)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHADOW)
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(ghoul)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHADOW)
    );
}
