//! Shared library inspection and search restrictions, using the real card definitions.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::GameEvent;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

pub(super) fn choose(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let player = e.pending().asked().unwrap();
    e.apply(player, PlayerAction::ChooseObjects { objects })
        .unwrap();
}
pub(super) fn library(e: &Engine<RegistryLookup>, p: PlayerId) -> Vec<ObjectId> {
    e.state().zones.list(ZoneLocation::Library(p)).clone()
}
pub(super) fn searched(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::SearchLibrary,
            ..
        }
    )
}

#[test]
fn library_choices_mindcensor_limits_inspection_and_finds_before_filtering() {
    for own in [false, true] {
        let mut mine = vec![index::EVOLVING_WILDS];
        let mut theirs = vec![];
        if own {
            mine.push(index::AVEN_MINDCENSOR);
        } else {
            theirs.push(index::AVEN_MINDCENSOR);
        }
        let cards = [
            index::MOUNTAIN,
            index::FOREST,
            index::LIGHTNING_BOLT,
            index::ISLAND,
            index::GIANT_GROWTH,
        ];
        let mut e = prepared(&cards, &mine, &theirs);
        let tops: Vec<_> = cards
            .into_iter()
            .map(|c| hand_to_library_top(&mut e, P0, c))
            .collect();
        let before = library(&e, P0);
        activate(&mut e, P0, index::EVOLVING_WILDS, 0);
        pass_until(&mut e, searched);
        let looked = e.inspected_cards();
        assert_eq!(looked.len(), if own { before.len() } else { 4 });
        for id in &tops[1..] {
            assert!(looked.contains(id));
        }
        assert_eq!(looked.contains(&tops[0]), own);
        let Pending::ChooseCards { options, min, .. } = e.pending() else {
            unreachable!()
        };
        assert_eq!(*min, 0, "a qualified hidden-zone search may fail to find");
        assert!(!options.contains(&tops[2]) && !options.contains(&tops[4]));
        assert_eq!(options.contains(&tops[0]), own);
        let hash = e.snapshot_hash();
        assert!(
            e.apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![tops[2]]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), hash);
        let since = e.state().journal.last_seq();
        choose(&mut e, vec![]);
        assert!(
            e.state()
                .journal
                .entries()
                .iter()
                .skip(since as usize)
                .any(|event| matches!(event.event, GameEvent::Shuffled { player: P0, .. }))
        );
        assert!(e.inspected_cards().is_empty());
        assert_eq!(library(&e, P0).len(), before.len());
    }
}

#[test]
fn library_choices_mindcensor_no_matching_cards_still_allows_inspection_and_shuffles() {
    let mut e = prepared(
        &[index::LIGHTNING_BOLT; 4],
        &[index::EVOLVING_WILDS],
        &[index::AVEN_MINDCENSOR],
    );
    for _ in 0..4 {
        hand_to_library_top(&mut e, P0, index::LIGHTNING_BOLT);
    }
    activate(&mut e, P0, index::EVOLVING_WILDS, 0);
    pass_until(&mut e, searched);
    assert_eq!(e.inspected_cards().len(), 4);
    assert!(
        matches!(e.pending(), Pending::ChooseCards { options, min: 0, max: 0, .. } if options.is_empty())
    );
    choose(&mut e, vec![]);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
}

#[test]
fn library_choices_visions_looks_without_reordering_then_offers_the_targeted_shuffle() {
    for shuffle in [false, true] {
        let mut e = prepared(&[index::VISIONS], &[], &[]);
        let before = library(&e, P1);
        let since = e.state().journal.last_seq();
        cast_from_hand(&mut e, P0, index::VISIONS);
        e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::LookAtLibrary,
                    ..
                }
            )
        });
        let Pending::ChooseCards {
            options,
            min: 0,
            max: 0,
            ..
        } = e.pending()
        else {
            panic!("inspection");
        };
        assert_eq!(
            options,
            &before.iter().rev().take(5).copied().collect::<Vec<_>>()
        );
        assert!(e.information_pending_for(P0).is_some());
        assert!(e.information_pending_for(P1).is_none());
        assert_eq!(library(&e, P1), before);
        choose(&mut e, vec![]);
        e.apply(P0, PlayerAction::YesNo(shuffle)).unwrap();
        pass_until(&mut e, stack_is_empty);
        let events = &e.state().journal.entries()[since as usize..];
        assert!(
            !events
                .iter()
                .any(|entry| matches!(entry.event, GameEvent::Revealed { .. }))
        );
        assert_eq!(
            events
                .iter()
                .any(|entry| matches!(entry.event, GameEvent::Shuffled { player: P1, .. })),
            shuffle
        );
        if !shuffle {
            assert_eq!(library(&e, P1), before);
        }
    }
}

#[test]
fn library_choices_moonring_needs_two_blue_permanents_and_looks_privately() {
    for count in 0..=3 {
        let mut mine = vec![index::MOONRING_ISLAND];
        mine.extend(std::iter::repeat_n(
            index::MERFOLK_OF_THE_PEARL_TRIDENT,
            count,
        ));
        let mut e = prepared(&[], &mine, &[]);
        let source = on_battlefield(&e, P0, index::MOONRING_ISLAND).unwrap();
        let hash = e.snapshot_hash();
        let result = e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 1,
            },
        );
        assert_eq!(result.is_ok(), count >= 2);
        if count < 2 {
            assert_eq!(e.snapshot_hash(), hash);
            continue;
        }
        let before = library(&e, P1);
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::LookAtLibrary,
                    ..
                }
            )
        });
        let Pending::ChooseCards { options, .. } = e.pending() else {
            unreachable!()
        };
        assert_eq!(options, &vec![*before.last().unwrap()]);
        choose(&mut e, vec![]);
        assert_eq!(library(&e, P1), before);
    }
}

#[test]
fn library_choices_crossroads_uses_the_starting_seat_and_exclusive_scry_or_untap() {
    for starting in [P0, P1] {
        for untap in [false, true] {
            let mut e = prepared(&[index::FORSAKEN_CROSSROADS], &[], &[]);
            e.dev_state_mut(P0).unwrap().starting_player = starting;
            let land = play_land(&mut e, P0, index::FORSAKEN_CROSSROADS);
            e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
                .unwrap();
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::ChooseCastMode { .. } | Pending::Arrange { .. }
                )
            });
            if starting != P0 {
                e.apply(P0, PlayerAction::ChooseMode(usize::from(untap)))
                    .unwrap();
            }
            if starting == P0 || !untap {
                let Pending::Arrange { cards, .. } = e.pending() else {
                    panic!("scry");
                };
                let cards = cards.clone();
                e.apply(
                    P0,
                    PlayerAction::Arrange {
                        piles: vec![cards, vec![]],
                    },
                )
                .unwrap();
            }
            pass_until(&mut e, stack_is_empty);
            assert_eq!(
                e.state()
                    .object(land)
                    .unwrap()
                    .status
                    .contains(Status::TAPPED),
                starting == P0 || !untap
            );
        }
    }
}

#[test]
fn library_choices_crossroads_cannot_replace_scry_with_an_impossible_untap() {
    for departed in [false, true] {
        let mut e = prepared(&[index::FORSAKEN_CROSSROADS], &[], &[]);
        e.dev_state_mut(P0).unwrap().starting_player = P1;
        let land = play_land(&mut e, P0, index::FORSAKEN_CROSSROADS);
        e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
        let state = e.dev_state_mut(P0).unwrap();
        if departed {
            state
                .move_object(
                    land,
                    ZoneLocation::Graveyard(P0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        } else {
            state.set_tapped(land, false);
        }
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        let Pending::ChooseCastMode { options, .. } = e.pending() else {
            unreachable!()
        };
        assert_eq!(options.len(), 1, "only scry is possible");
        let hash = e.snapshot_hash();
        assert!(e.apply(P0, PlayerAction::ChooseMode(1)).is_err());
        assert_eq!(e.snapshot_hash(), hash);
        e.apply(P0, PlayerAction::ChooseMode(0)).unwrap();
        assert!(matches!(e.pending(), Pending::Arrange { .. }));
    }
}

#[test]
fn library_choices_watchtower_requires_two_more_lands_and_reveals_only_when_accepted() {
    for others in 0..=4 {
        for top in [index::FOREST, index::LIGHTNING_BOLT] {
            for reveal in [false, true] {
                let mut e = prepared(
                    &[top],
                    &[index::ISOLATED_WATCHTOWER],
                    &vec![index::ISLAND; others],
                );
                let card = hand_to_library_top(&mut e, P0, top);
                let source = on_battlefield(&e, P0, index::ISOLATED_WATCHTOWER).unwrap();
                let result = e.apply(
                    P0,
                    PlayerAction::ActivateAbility {
                        source,
                        ability_index: 1,
                    },
                );
                assert_eq!(result.is_ok(), others >= 3);
                if others < 3 {
                    continue;
                }
                pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
                e.apply(
                    P0,
                    PlayerAction::Arrange {
                        piles: vec![vec![card], vec![]],
                    },
                )
                .unwrap();
                let since = e.state().journal.last_seq();
                e.apply(P0, PlayerAction::YesNo(reveal)).unwrap();
                pass_until(&mut e, stack_is_empty);
                let moved = reveal && top == index::FOREST;
                assert_eq!(
                    e.state().object(card).unwrap().zone,
                    if moved {
                        Zone::Battlefield
                    } else {
                        Zone::Library
                    }
                );
                if moved {
                    assert!(
                        e.state()
                            .object(card)
                            .unwrap()
                            .status
                            .contains(Status::TAPPED)
                    );
                }
                assert_eq!(
                    e.state().journal.entries()[since as usize..]
                        .iter()
                        .any(|entry| matches!(entry.event, GameEvent::Revealed { .. })),
                    reveal
                );
            }
        }
    }
}

#[test]
fn library_choices_optional_search_can_be_declined_before_inspection_or_shuffle() {
    for accept in [false, true] {
        for agent in [false, true] {
            let mut mine = vec![index::AVEN_MINDCENSOR];
            if agent {
                mine.push(index::OPPOSITION_AGENT);
            }
            let mut e = prepared(&[index::PATH_TO_EXILE], &mine, &[index::GRIZZLY_BEARS]);
            let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
            let before = library(&e, P1);
            cast_from_hand(&mut e, P0, index::PATH_TO_EXILE);
            e.apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![],
                },
            )
            .unwrap();
            pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
            assert_eq!(e.pending().asked(), Some(P1));
            assert!(e.inspected_cards().is_empty());
            let since = e.state().journal.last_seq();
            e.apply(P1, PlayerAction::YesNo(accept)).unwrap();
            if accept {
                assert_eq!(e.pending().asked(), Some(if agent { P0 } else { P1 }));
                assert_eq!(
                    e.inspected_cards(),
                    &before.iter().rev().take(4).copied().collect::<Vec<_>>()
                );
                choose(&mut e, vec![]);
            }
            pass_until(&mut e, stack_is_empty);
            assert_eq!(
                e.state().journal.entries()[since as usize..]
                    .iter()
                    .any(|entry| matches!(entry.event, GameEvent::Shuffled { player: P1, .. })),
                accept
            );
            if !accept {
                assert_eq!(library(&e, P1), before);
            }
        }
    }
}

#[test]
fn library_choices_mindcensor_restricts_the_searcher_even_in_someone_elses_library() {
    let mut e = prepared(&[index::BRIBERY], &[], &[index::AVEN_MINDCENSOR]);
    let library = library(&e, P1);
    cast_from_hand(&mut e, P0, index::BRIBERY);
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut e, searched);
    assert_eq!(e.pending().asked(), Some(P0));
    assert_eq!(
        e.inspected_cards(),
        &library.iter().rev().take(4).copied().collect::<Vec<_>>()
    );
    assert!(
        matches!(e.pending(), Pending::ChooseCards { options, max: 0, .. } if options.is_empty())
    );
    choose(&mut e, vec![]);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
}
