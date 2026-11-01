//! `cards/creatures/mv_1/roc_hatchling.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_cards_dsl::counters::SHELL;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn roc_hatchling() -> CardIndex {
    card_index("6764d4d9-1fe5-4b42-830c-a3b4c3b37f1f")
}

fn shells(e: &Engine<RegistryLookup>, id: ObjectId) -> u32 {
    u32::from(e.state().object(id).unwrap().counters.get(SHELL))
}

fn to_main(e: &mut Engine<RegistryLookup>, turn: u32, seat: PlayerId) {
    pass_until(e, |e| {
        e.state().turn.number == turn
            && e.state().turn.active == seat
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { .. })
    });
}

/// Roc Hatchling (6764d4d9-1fe5-4b42-830c-a3b4c3b37f1f) — "This creature enters
/// with four shell counters on it. At the beginning of **your** upkeep, remove a
/// shell counter. As long as it has no shell counters on it, it gets +3/+2 and
/// has flying." Cast: four counters, a 0/1 without flying. The opponent's upkeep
/// removes nothing, each of its controller's removes one, and only at zero is it
/// a 3/3 flyer.
#[test]
fn roc_hatchling_loses_a_shell_per_own_upkeep_and_hatches_at_none() {
    let mut e = lk::game(SEED, &[mountain()], &[], &[roc_hatchling()], &[]);

    cast_from_hand(&mut e, P0, roc_hatchling());
    pass_until(&mut e, stack_is_empty);
    let roc = lk::obj(&e, P0, roc_hatchling());
    assert_eq!(shells(&e, roc), 4, "enters with four shell counters");
    assert_eq!(pt(&e, roc), (0, 1));
    assert!(!keywords(&e, roc).contains(KeywordSet::FLYING));

    // Two shells left, so the shell stays on through the opponent's turn.
    e.state.object_mut(roc).unwrap().counters.set(SHELL, 2);
    to_main(&mut e, 2, P1);
    assert_eq!(shells(&e, roc), 2, "the opponent's upkeep removes nothing");

    to_main(&mut e, 3, P0);
    assert_eq!(shells(&e, roc), 1, "its controller's upkeep removes one");
    assert_eq!(pt(&e, roc), (0, 1), "still shelled: no bonus");
    assert!(
        !keywords(&e, roc).contains(KeywordSet::FLYING),
        "one shell left, no flying"
    );

    to_main(&mut e, 5, P0);
    assert_eq!(shells(&e, roc), 0);
    assert_eq!(pt(&e, roc), (3, 3), "no shell counters: +3/+2");
    assert!(keywords(&e, roc).contains(KeywordSet::FLYING), "and flying");
}
