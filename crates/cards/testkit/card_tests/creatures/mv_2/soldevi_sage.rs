//! `cards/creatures/mv_2/soldevi_sage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldevi_sage() -> CardIndex {
    card_index("1f612df3-53b6-4317-9d63-1f903ee3f0c4")
}

/// Soldevi Sage ({1}{U} 1/1 Creature — Human Wizard, Partial):
/// "{T}, Sacrifice two lands: Draw three cards, then discard one of them."
/// The discard must be one of the three cards drawn, and no effect says that
/// (the discard effect takes any card from the hand, which is more than
/// printed), so the card offers no ability. A pinned limitation: this test
/// moves when the discard can be restricted to the cards drawn.
#[test]
fn soldevi_sage_offers_no_ability_instead_of_a_wider_discard_than_printed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[soldevi_sage(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sage = on_battlefield(&engine, p0, soldevi_sage()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    let lands_before = all_on_battlefield(&engine, p0, island()).len();
    assert_eq!(lands_before, 2);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == sage),
        "the {{T}}, sacrifice-two-lands ability is not offered"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: sage,
                    ability_index: 0,
                },
            )
            .is_err()
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, island()).len(),
        lands_before
    );
    assert!(!is_tapped(&engine, sage));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before
    );
}
