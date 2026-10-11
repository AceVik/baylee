//! Saga chapters follow every lore placement, including proliferate.
#[allow(clippy::wildcard_imports)] // The batch's real-card helpers.
use super::*;
use baylee_cards_dsl::AbilityDef;

fn welcome(doubling: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut mine = vec![index::KARN_S_BASTION];
    if doubling {
        mine.push(index::DOUBLING_SEASON);
    }
    let mut e = prepared(&[index::WELCOME_TO], &mine, &[index::SOL_RING]);
    let artifact = on_battlefield(&e, P1, index::SOL_RING).unwrap();
    cast_with_floating(&mut e, P0, index::WELCOME_TO);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    target(&mut e, vec![artifact], vec![]);
    pass_until(&mut e, stack_is_empty);
    let saga = on_battlefield(&e, P0, index::WELCOME_TO).unwrap();
    (e, saga, artifact)
}

fn dinosaurs(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    e.state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|object| {
                object
                    .characteristics()
                    .subtypes
                    .contains(subtypes::creature::DINOSAUR)
            })
        })
        .collect()
}

fn proliferate(e: &mut Engine<RegistryLookup>, cards: Vec<ObjectId>) {
    let bastion = on_battlefield(e, P0, index::KARN_S_BASTION).unwrap();
    e.state.set_tapped(bastion, false);
    e.refresh_offer();
    activate(e, P0, index::KARN_S_BASTION, 1);
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Proliferate,
                ..
            }
        )
    });
    choose(e, cards);
}

#[test]
fn batch45_welcome_proliferate_triggers_chapters_two_and_three() {
    let (mut e, saga, artifact) = welcome(false);
    assert_eq!(counters_on(&e, saga, CounterKind::Lore), 1);
    assert!(dinosaurs(&e).is_empty());
    proliferate(&mut e, vec![saga]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, saga, CounterKind::Lore), 2);
    let created = dinosaurs(&e);
    assert_eq!(created.len(), 1);
    assert!(keywords(&e, created[0]).contains(KeywordSet::HASTE));
    proliferate(&mut e, vec![saga]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(saga).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.object(saga).unwrap().face_index, 1);
    assert_eq!(e.state.object(artifact).unwrap().zone, Zone::Graveyard);
    assert_eq!(dinosaurs(&e).len(), 1);
}

#[test]
fn batch45_welcome_doubled_lore_runs_each_crossed_chapter_once() {
    let (mut e, saga, artifact) = welcome(true);
    // Entry lore doubles, crossing I and II; chapter II creates two tokens.
    assert_eq!(counters_on(&e, saga, CounterKind::Lore), 2);
    assert_eq!(dinosaurs(&e).len(), 2);
    assert_eq!(pt(&e, artifact), (0, 4));
    proliferate(&mut e, vec![saga]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(saga).unwrap().face_index, 1);
    assert_eq!(e.state.object(artifact).unwrap().zone, Zone::Graveyard);
    assert_eq!(dinosaurs(&e).len(), 2, "chapter II never repeats at 2 -> 4");
}

#[test]
fn batch45_saga_counter_events_keep_the_departed_incarnation_and_abilities() {
    let (mut e, saga, _) = welcome(false);
    let version = e.state.object(saga).unwrap().version;
    let since = e.state.journal.last_seq();
    crate::replacement::put_counters_by(&mut e.state, P0, saga, CounterKind::Lore, 1);
    e.state
        .move_object(
            saga,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            saga,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let triggers = crate::trigger::collect(&e.state, &RegistryLookup, since);
    let chapters: Vec<_> = triggers
        .iter()
        .filter(|trigger| trigger.source == saga)
        .collect();
    assert_eq!(chapters.len(), 1);
    let chapter = chapters[0];
    assert_eq!(chapter.source_version, Some(version));
    assert_ne!(e.state.object(saga).unwrap().version, version);
    let abilities = chapter.abilities.as_ref().expect("event-time ability list");
    assert!(matches!(
        abilities.abilities[chapter.ability_index as usize],
        AbilityDef::SagaChapter { chapter: 2, .. }
    ));
}

#[test]
fn batch45_saga_counter_removal_and_zero_placement_never_trigger_a_chapter() {
    let (mut e, saga, _) = welcome(false);
    let since = e.state.journal.last_seq();
    crate::replacement::remove_counters(&mut e.state, saga, CounterKind::Lore, 1);
    crate::replacement::record_counters(&mut e.state, saga, CounterKind::Lore, 0);
    assert!(crate::trigger::collect(&e.state, &RegistryLookup, since).is_empty());
}

#[test]
fn batch45_proliferating_two_sagas_collects_both_chapters_in_one_batch() {
    let mut e = prepared(
        &[index::WELCOME_TO, index::WELCOME_TO],
        &[index::KARN_S_BASTION],
        &[index::SOL_RING, index::SOL_RING],
    );
    let artifacts: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|object| {
                object.controller == P1
                    && object
                        .card
                        .is_some_and(|card| card.index == index::SOL_RING)
            })
        })
        .collect();
    for artifact in artifacts {
        cast_with_floating(&mut e, P0, index::WELCOME_TO);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        target(&mut e, vec![artifact], vec![]);
        pass_until(&mut e, stack_is_empty);
    }
    let sagas: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|object| {
                object
                    .card
                    .is_some_and(|card| card.index == index::WELCOME_TO)
            })
        })
        .collect();
    assert_eq!(sagas.len(), 2);
    proliferate(&mut e, sagas.clone());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(dinosaurs(&e).len(), 2);
    for saga in sagas {
        assert_eq!(counters_on(&e, saga, CounterKind::Lore), 2);
    }
}
