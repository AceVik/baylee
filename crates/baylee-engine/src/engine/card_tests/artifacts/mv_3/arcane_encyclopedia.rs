//! Oracle checklist: pay {3}, tap; draw exactly one card; leave Encyclopedia on the battlefield tapped; no other player's hand changes.
#[allow(clippy::wildcard_imports)]
use super::*;
fn arcane_encyclopedia() -> CardIndex {
    card_index("d91ea728-2d69-42cb-bb5d-e2b058f0d7b1")
}
#[test]
fn encyclopedia_taps_and_draws_one_card() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                arcane_encyclopedia(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, arcane_encyclopedia()).unwrap();
    let hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, arcane_encyclopedia(), 0);
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Green),
        3,
        "the {{3}} activation cost spends exactly three of six mana, leaving enough to test the tapped source"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, source));
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), hand + 1);
    assert!(on_battlefield(&e, p0, arcane_encyclopedia()).is_some());
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("the active player has priority after the ability resolves")
    };
    assert!(
        !legal.abilities.contains(&(source, 0)),
        "the tapped source cannot be activated again"
    );
}
