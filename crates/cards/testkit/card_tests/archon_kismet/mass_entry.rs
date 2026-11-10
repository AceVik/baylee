//! Large token instructions must not ask or sort separately for each identical body.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn kismet_two_thousand_simultaneous_creatures_enter_tapped_without_questions() {
    let mut e = Duel::new(524_001, index::FOREST)
        .battlefield(0, &[index::KISMET])
        .battlefield(1, &[index::DOUBLING_SEASON; 10])
        .hand(1, &[index::RAISE_THE_ALARM])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast(&mut e, P1, index::RAISE_THE_ALARM);
    pass_until(&mut e, stack_is_empty);
    let incoming: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state
                .object(*id)
                .is_some_and(|o| o.controller == P1 && o.card.is_none())
        })
        .collect();
    assert_eq!(incoming.len(), 2_048, "two Soldiers doubled ten times");
    for id in incoming {
        assert!(is_tapped(&e, id));
        assert!(!e.state.entries.contains(id));
    }
}
