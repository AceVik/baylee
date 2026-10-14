//! `cards/enchantments/mv_1/sunspring_expedition.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sunspring_expedition() -> CardIndex {
    card_index("f5aeed95-24f8-4f28-9e7c-1e3502d013cd")
}

/// "Landfall — Whenever a land you control enters, you may put a quest counter
/// on this enchantment." and "Remove three quest counters from this
/// enchantment and sacrifice it: You gain 8 life."
#[test]
fn sunspring_expedition_counts_lands_and_gains_eight_at_three() {
    let mut e = table(
        7321,
        &[forest(), forest(), forest(), forest()],
        &[sunspring_expedition()],
        &[],
    );
    let quest = on_battlefield(&e, P0, sunspring_expedition()).unwrap();
    // Declined: a real "may".
    play_land(&mut e, P0, forest());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    drive(&mut e, &[], &[], &[], false);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, quest, counters::QUEST), 0);
    for expected in 1..=3u16 {
        next_main(&mut e, P0);
        play_land(&mut e, P0, forest());
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        drive(&mut e, &[], &[], &[], true);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(counters_on(&e, quest, counters::QUEST), expected);
    }
    activate(&mut e, P0, sunspring_expedition(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P0), 28);
    assert!(in_graveyard(&e, P0, sunspring_expedition()).is_some());
}

/// Seat `p`'s next first main phase.
fn next_main(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    let turn = e.state().turn.number;
    pass_until(e, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p
            && e.state().turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p)
    });
}
