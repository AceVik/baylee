//! `cards/enchantments/mv_1/quest_for_the_gravelord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn quest_for_the_gravelord() -> CardIndex {
    card_index("c5f92560-dc82-4bd2-ae6b-4c3942d458b7")
}

/// "Whenever a creature dies, you may put a quest counter on this enchantment."
/// "Remove three quest counters from this enchantment and sacrifice it: Create
/// a 5/5 black Zombie Giant creature token."
#[test]
fn quest_for_the_gravelord_counts_deaths_and_makes_a_giant_at_three() {
    let mut e = table(
        7311,
        &[],
        &[quest_for_the_gravelord()],
        &[
            index::GRIZZLY_BEARS,
            index::GRIZZLY_BEARS,
            index::GRIZZLY_BEARS,
        ],
    );
    let quest = on_battlefield(&e, P0, quest_for_the_gravelord()).unwrap();
    let victims: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|o| {
            e.state().object(*o).is_some_and(|x| {
                x.controller == P1 && x.card.is_some_and(|c| c.index == index::GRIZZLY_BEARS)
            })
        })
        .collect();
    assert_eq!(victims.len(), 3);
    // The first death is declined, so a "may" is a real question.
    kill_asking(&mut e, victims[0], false);
    assert_eq!(counters_on(&e, quest, counters::QUEST), 0);
    assert!(!ability_offered(&e, quest), "no counters, no giant");
    for v in &victims[1..] {
        kill_asking(&mut e, *v, true);
    }
    assert_eq!(counters_on(&e, quest, counters::QUEST), 2);
    assert!(!ability_offered(&e, quest), "two counters are not enough");
}

fn ability_offered(e: &Engine<RegistryLookup>, quest: ObjectId) -> bool {
    matches!(e.pending(), Pending::Priority { legal, .. } if legal.abilities.contains(&(quest, 0)))
}

/// Destroys `victim`, then answers the "may" with `yes`.
fn kill_asking(e: &mut Engine<RegistryLookup>, victim: ObjectId, yes: bool) {
    crate::sba::destroy(e.dev_state_mut(P0).unwrap(), victim);
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("priority expected")
    };
    e.apply(player, PlayerAction::PassPriority).unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    drive(e, &[], &[], &[], yes);
    pass_until(e, stack_is_empty);
}
