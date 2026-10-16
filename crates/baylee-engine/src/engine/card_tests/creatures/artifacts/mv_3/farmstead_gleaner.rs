//! `cards/creatures/artifacts/mv_3/farmstead_gleaner.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn farmstead_gleaner() -> CardIndex {
    card_index("e26f7255-8b18-47d6-aafa-22ca22c35d2e")
}

/// Farmstead Gleaner — "This creature doesn't untap during your untap step"
/// and "{2}, {Q}: Put a +1/+1 counter on this creature." The {Q} cost needs
/// it tapped: not offered untapped; after attacking it stays tapped through
/// the next untap step, and then {2} untaps it for a counter.
#[test]
fn farmstead_gleaner_stays_tapped_and_untaps_for_a_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[farmstead_gleaner(), forest(), forest()]);
    let gleaner = on_battlefield(&engine, p0, farmstead_gleaner()).expect("seated");
    tap_all_mana(&mut engine, p0);
    assert!(
        !priority_offer(&engine).abilities.contains(&(gleaner, 1)),
        "{{Q}} wants it tapped: an untapped Gleaner offers nothing"
    );

    attack_to_blocks(&mut engine, &[gleaner]);
    engine
        .apply(
            PlayerId::new(1),
            PlayerAction::DeclareBlockers { blockers: vec![] },
        )
        .unwrap();
    assert!(is_tapped(&engine, gleaner), "attacking tapped it");

    // Seat 0's next turn: the untap step passes it by.
    pass_until(&mut engine, |e| e.state().turn.number > 1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        is_tapped(&engine, gleaner),
        "it doesn't untap during its untap step"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(priority_offer(&engine).abilities.contains(&(gleaner, 1)));
    activate(&mut engine, p0, farmstead_gleaner(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, gleaner), "{{Q}} untapped it");
    assert_eq!(counters_on(&engine, gleaner, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, gleaner), (3, 3));
}
