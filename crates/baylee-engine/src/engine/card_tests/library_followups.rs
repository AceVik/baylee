//! Found-card identity, behold and conditional entry counters through real cards.
#[allow(clippy::wildcard_imports)]
use super::*;
use super::{
    counter_resources::prepared,
    library_choices::{choose, library, searched},
};
use crate::event::{Cause, GameEvent};
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

#[test]
fn library_followups_prohibited_mandatory_search_still_shuffles() {
    for (land, ability) in [
        (index::FABLED_PASSAGE, 0),
        (index::ELVEN_PASSAGE, 0),
        (index::MYRIAD_LANDSCAPE, 1),
    ] {
        let mut e = prepared(&[], &[land], &[index::ASHIOK_DREAM_RENDER]);
        let before = library(&e, P0);
        let since = e.state().journal.last_seq();
        activate(&mut e, P0, land, ability);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library(&e, P0).len(), before.len());
        assert_eq!(
            e.state()
                .journal
                .entries()
                .iter()
                .skip(since as usize)
                .filter(|entry| { matches!(entry.event, GameEvent::Shuffled { player: P0, .. }) })
                .count(),
            1,
            "the prohibited search still completes its shuffle"
        );
    }
}

#[test]
fn library_followups_fabled_passage_counts_lands_after_its_found_land_enters() {
    for lands in 0..=5 {
        for find in [false, true] {
            let mut mine = vec![index::FOREST; lands];
            mine.push(index::FABLED_PASSAGE);
            let mut e = prepared(&[], &mine, &[]);
            activate(&mut e, P0, index::FABLED_PASSAGE, 0);
            pass_until(&mut e, searched);
            let Pending::ChooseCards { options, .. } = e.pending() else {
                unreachable!()
            };
            let found = options[0];
            let version = e.state().object(found).unwrap().version;
            choose(&mut e, if find { vec![found] } else { vec![] });
            pass_until(&mut e, stack_is_empty);
            let object = e.state().object(found).unwrap();
            assert_eq!(object.zone == Zone::Battlefield, find);
            if find {
                assert_eq!(object.version, version.wrapping_add(1));
                assert_eq!(object.status.contains(Status::TAPPED), lands < 3);
            }
        }
    }
}

#[test]
fn library_followups_fabled_passage_waits_for_entry_replacements_and_their_sacrifices() {
    for lands in [3, 4] {
        for sacrifice_found in [false, true] {
            let mut mine = vec![index::FOREST; lands];
            mine.push(index::FABLED_PASSAGE);
            let mut e = prepared(&[], &mine, &[index::LAND_EQUILIBRIUM]);
            let old_land = on_battlefield(&e, P0, index::FOREST).unwrap();
            activate(&mut e, P0, index::FABLED_PASSAGE, 0);
            pass_until(&mut e, searched);
            let Pending::ChooseCards { options, .. } = e.pending() else {
                unreachable!()
            };
            let found = options[0];
            choose(&mut e, vec![found]);
            assert!(matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::SacrificeFor { .. },
                    ..
                }
            ));
            assert!(
                is_tapped(&e, found),
                "no early untap before the entry finishes"
            );
            choose(&mut e, vec![if sacrifice_found { found } else { old_land }]);
            pass_until(&mut e, stack_is_empty);
            if sacrifice_found {
                assert_eq!(e.state().object(found).unwrap().zone, Zone::Graveyard);
            } else {
                assert_eq!(is_tapped(&e, found), lands < 4);
            }
        }
    }
}

#[test]
fn library_followups_elven_passage_beholds_from_hand_or_controlled_battlefield() {
    for in_hand in [false, true] {
        for accept in [false, true] {
            let mut mine = vec![index::ELVEN_PASSAGE];
            let hand = if in_hand {
                vec![index::LLANOWAR_ELVES]
            } else {
                mine.push(index::LLANOWAR_ELVES);
                vec![]
            };
            let mut e = prepared(&hand, &mine, &[index::LLANOWAR_ELVES]);
            let elf = if in_hand {
                super::in_hand(&e, P0, index::LLANOWAR_ELVES).unwrap()
            } else {
                on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap()
            };
            let enemy = on_battlefield(&e, P1, index::LLANOWAR_ELVES).unwrap();
            activate(&mut e, P0, index::ELVEN_PASSAGE, 0);
            assert_eq!(e.state().players[0].life, 19);
            pass_until(&mut e, searched);
            let Pending::ChooseCards { options, .. } = e.pending() else {
                unreachable!()
            };
            let found = options[0];
            choose(&mut e, vec![found]);
            let Pending::ChooseCards {
                options,
                prompt: ChoicePrompt::Behold,
                min: 0,
                max: 1,
                ..
            } = e.pending()
            else {
                panic!("behold choice")
            };
            assert_eq!(options, &vec![elf]);
            let hash = e.snapshot_hash();
            assert!(
                e.apply(
                    P0,
                    PlayerAction::ChooseObjects {
                        objects: vec![enemy]
                    }
                )
                .is_err()
            );
            assert_eq!(e.snapshot_hash(), hash);
            let since = e.state().journal.last_seq();
            choose(&mut e, if accept { vec![elf] } else { vec![] });
            pass_until(&mut e, stack_is_empty);
            assert_eq!(is_tapped(&e, found), !accept);
            assert_eq!(
                e.state().object(elf).unwrap().zone,
                if in_hand {
                    Zone::Hand
                } else {
                    Zone::Battlefield
                }
            );
            let revealed = e.state().journal.entries().iter().any(|entry| {
                entry.seq > since && matches!(&entry.event, GameEvent::Revealed { cards, .. } if cards.contains(&elf))
            });
            assert_eq!(revealed, in_hand && accept);
        }
    }
}

#[test]
fn library_followups_behold_does_not_untap_a_different_incarnation() {
    let mut e = prepared(&[], &[index::ELVEN_PASSAGE, index::LLANOWAR_ELVES], &[]);
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    activate(&mut e, P0, index::ELVEN_PASSAGE, 0);
    pass_until(&mut e, searched);
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    let found = options[0];
    choose(&mut e, vec![found]);
    assert!(matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::Behold,
            ..
        }
    ));
    // Fixture-only zone changes isolate the retained-reference invariant;
    // players cannot normally act in the middle of this resolution.
    let state = e.dev_state_mut(P0).unwrap();
    state
        .move_object(
            found,
            ZoneLocation::Hand(P0),
            crate::zone::ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            found,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state.set_tapped(found, true);
    choose(&mut e, vec![elf]);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, found));
}

#[test]
fn library_followups_elven_passage_without_an_elf_or_with_a_replaced_search() {
    for agent in [false, true] {
        let theirs = if agent {
            vec![index::OPPOSITION_AGENT]
        } else {
            vec![index::LLANOWAR_ELVES]
        };
        let mut e = prepared(&[], &[index::ELVEN_PASSAGE], &theirs);
        activate(&mut e, P0, index::ELVEN_PASSAGE, 0);
        pass_until(&mut e, searched);
        let Pending::ChooseCards { options, .. } = e.pending() else {
            unreachable!()
        };
        let found = options[0];
        choose(&mut e, vec![found]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(found).unwrap().zone,
            if agent {
                Zone::Exile
            } else {
                Zone::Battlefield
            }
        );
        if !agent {
            assert!(is_tapped(&e, found));
        }
    }
}

#[test]
fn library_followups_turntimber_uses_library_mana_value_and_randomizes_only_the_rest() {
    for candidate in [
        index::WALKING_BALLISTA,
        index::AVEN_MINDCENSOR,
        index::SERRA_ANGEL,
    ] {
        for doubled in [false, true] {
            for take in [false, true] {
                let mine = if doubled {
                    vec![index::DOUBLING_SEASON]
                } else {
                    vec![]
                };
                let mut e = prepared(
                    &[index::TURNTIMBER_SYMBIOSIS, candidate],
                    &mine,
                    &[index::AVEN_MINDCENSOR],
                );
                let card = hand_to_library_top(&mut e, P0, candidate);
                let before = library(&e, P0);
                cast_with_floating(&mut e, P0, index::TURNTIMBER_SYMBIOSIS);
                pass_until(&mut e, |e| {
                    matches!(
                        e.pending(),
                        Pending::ChooseCards {
                            prompt: ChoicePrompt::PutOntoBattlefield,
                            ..
                        }
                    )
                });
                assert_eq!(e.inspected_cards().len(), 7, "looking is not searching");
                let Pending::ChooseCards {
                    options,
                    min: 0,
                    max: 1,
                    ..
                } = e.pending()
                else {
                    panic!("optional creature")
                };
                assert_eq!(options, &vec![card]);
                choose(&mut e, if take { vec![card] } else { vec![] });
                pass_until(&mut e, stack_is_empty);
                assert_eq!(
                    e.state().object(card).unwrap().zone,
                    if take {
                        Zone::Battlefield
                    } else {
                        Zone::Library
                    }
                );
                if take {
                    let expected = if candidate == index::SERRA_ANGEL {
                        0
                    } else if doubled {
                        6
                    } else {
                        3
                    };
                    assert_eq!(
                        e.state()
                            .object(card)
                            .unwrap()
                            .counters
                            .get(CounterKind::P1P1),
                        expected
                    );
                }
                let after = library(&e, P0);
                let bottom_count = 7 - usize::from(take);
                assert_eq!(&after[bottom_count..], &before[..before.len() - 7]);
                let mut bottom = after[..bottom_count].to_vec();
                let mut expected = before[before.len() - 7..]
                    .iter()
                    .copied()
                    .filter(|id| !take || *id != card)
                    .collect::<Vec<_>>();
                bottom.sort_unstable();
                expected.sort_unstable();
                assert_eq!(bottom, expected);
            }
        }
    }
}

#[test]
fn library_followups_turntimber_entry_triggers_see_the_additional_counters() {
    let mut e = prepared(
        &[index::TURNTIMBER_SYMBIOSIS, index::WALKING_BALLISTA],
        &[index::GARRUK_S_UPRISING, index::DOUBLING_SEASON],
        &[],
    );
    let card = hand_to_library_top(&mut e, P0, index::WALKING_BALLISTA);
    cast_with_floating(&mut e, P0, index::TURNTIMBER_SYMBIOSIS);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PutOntoBattlefield,
                ..
            }
        )
    });
    let hand_before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    choose(&mut e, vec![card]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1
    );
}

#[test]
fn library_followups_turntimber_keeps_its_counter_decision_through_copy_replacements() {
    for (candidate, target, counters) in [
        (index::CLONE, index::AVEN_MINDCENSOR, 0),
        (index::GLASSPOOL_MIMIC, index::SERRA_ANGEL, 6),
    ] {
        let mut e = prepared(
            &[index::TURNTIMBER_SYMBIOSIS, candidate],
            &[target, index::DOUBLING_SEASON],
            &[],
        );
        let card = hand_to_library_top(&mut e, P0, candidate);
        let target = on_battlefield(&e, P0, target).unwrap();
        cast_with_floating(&mut e, P0, index::TURNTIMBER_SYMBIOSIS);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::PutOntoBattlefield,
                    ..
                }
            )
        });
        choose(&mut e, vec![card]);
        assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(card).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            e.state()
                .object(card)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            counters
        );
    }
}
