//! `cards/lands/utility/crawling_barrens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crawling Barrens prints `{T}: Add {C}` and `{4}: Put two +1/+1 counters on this land. Then you
/// may have it become a 0/0 Elemental creature until end of turn. It's still a land.`
/// The card is marked `Coverage::Implemented`.
/// Activating the second ability with four mana floating places two `CounterKind::P1P1` counters
/// on the land, and accepting the optional animation turns it into a 2/2 Elemental creature that
/// remains a land and is still untapped.
#[test]
fn crawling_barrens_animates_into_a_two_two_creature_with_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[crawling_barrens()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let barrens = play_land(&mut engine, p0, crawling_barrens());
    assert!(!types(&engine, barrens).contains(TypeSet::CREATURE));
    assert_eq!(counters_on(&engine, barrens, CounterKind::P1P1), 0);

    tap_all_mana_but(&mut engine, p0, Some(crawling_barrens()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4
    );

    activate(&mut engine, p0, crawling_barrens(), 1);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, barrens, CounterKind::P1P1), 2);
    let t = types(&engine, barrens);
    assert!(t.contains(TypeSet::CREATURE));
    assert!(t.contains(TypeSet::LAND));
    assert_eq!(pt(&engine, barrens), (2, 2));
    assert!(!is_tapped(&engine, barrens));
}
