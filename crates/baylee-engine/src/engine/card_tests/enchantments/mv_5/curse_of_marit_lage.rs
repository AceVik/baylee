//! `cards/enchantments/mv_5/curse_of_marit_lage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn curse_of_marit_lage() -> CardIndex {
    card_index("6fde4790-4186-4786-aa1a-b6a53bfadc1c")
}

/// "When this enchantment enters, tap all Islands. Islands don't untap
/// during their controllers' untap steps." The Islands of both players are
/// tapped on entry (the Islands here are not the mana that paid for it) and
/// stay tapped through their controllers' next untap steps, while a
/// Mountain untaps as ever.
#[test]
fn curse_of_marit_lage_taps_islands_and_keeps_them_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[curse_of_marit_lage()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
            ],
        )
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_island = on_battlefield(&engine, p0, island()).expect("my Island");
    let their_island = on_battlefield(&engine, p1, island()).expect("their Island");
    assert!(!is_tapped(&engine, my_island));
    assert!(!is_tapped(&engine, their_island));

    tap_mana_except(&mut engine, p0, my_island);
    cast_with_floating(&mut engine, p0, curse_of_marit_lage());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, curse_of_marit_lage()).is_some()
    });
    assert!(is_tapped(&engine, my_island), "all Islands, mine");
    assert!(is_tapped(&engine, their_island), "and theirs");

    // Their untap step, then mine: both Islands stay tapped.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(is_tapped(&engine, their_island), "theirs skipped its untap");
    assert!(is_tapped(&engine, my_island), "mine skipped its untap");
    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("a Mountain");
    assert!(!is_tapped(&engine, mountain_id), "a Mountain untapped");
}
