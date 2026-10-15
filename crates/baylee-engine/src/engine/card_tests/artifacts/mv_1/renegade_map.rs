//! Oracle: This artifact enters tapped. / {T}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;
use baylee_core::generated::index;

fn subject() -> CardIndex {
    card_index("f8506acb-3fc5-4fdf-8304-586843db6cfd")
}

// Checklist: real cast enters tapped and cannot activate; next own untap permits
// tap+sacrifice; only basic lands from own library, reveal, hand (not battlefield),
// shuffle; may fail a restricted search; no-basic library still shuffles.
#[test]
fn tapped_entry_then_basic_search_or_fail_and_shuffle() {
    let p0 = PlayerId::new(0);
    for filler in [forest(), index::GRAY_OGRE] {
        for choose_card in [false, true] {
            let mut e = Duel::new(SEED, filler)
                .battlefield(0, &[island()])
                .hand(0, &[subject(), badlands(), index::GRAY_OGRE])
                .start();
            keep_mulligans(&mut e);
            assert!(walk_to_own_main(&mut e, p0));
            cast_from_hand(&mut e, p0, subject());
            pass_until(&mut e, stack_is_empty);
            let source = on_battlefield(&e, p0, subject()).unwrap();
            assert!(is_tapped(&e, source));
            let Pending::Priority { legal, .. } = e.pending() else {
                panic!("priority")
            };
            assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
            let mut excluded = vec![];
            for fixture in [badlands(), index::GRAY_OGRE] {
                let id = in_hand(&e, p0, fixture).unwrap();
                e.dev_state_mut(p0)
                    .unwrap()
                    .move_object(
                        id,
                        ZoneLocation::Library(p0),
                        crate::zone::ZonePosition::Bottom,
                        crate::event::Cause::Effect,
                    )
                    .unwrap();
                excluded.push(id);
            }
            let turn = e.state().turn.number;
            pass_until(&mut e, |e| {
                e.state().turn.number > turn
                    && e.state().turn.active == p0
                    && matches!(e.state().turn.phase, Phase::FirstMain)
            });
            assert!(!is_tapped(&e, source));
            let hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();
            let count = library_size(&e, p0);
            let since = e.journal().last_seq();
            activate(&mut e, p0, subject(), 0);
            assert!(in_graveyard(&e, p0, subject()).is_some());
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::ChooseCards {
                        prompt: ChoicePrompt::SearchLibrary,
                        ..
                    }
                )
            });
            let Pending::ChooseCards {
                player,
                options,
                min,
                max,
                ..
            } = e.pending().clone()
            else {
                unreachable!()
            };
            assert_eq!(player, p0);
            for id in &excluded {
                assert!(!options.contains(id));
            }
            assert_eq!(min, 0);
            assert_eq!(options.is_empty(), filler != forest());
            let selected = if choose_card && !options.is_empty() {
                vec![options[0]]
            } else {
                vec![]
            };
            assert!(max <= 1);
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: selected.clone(),
                },
            )
            .unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(library_size(&e, p0), count - selected.len());
            assert_eq!(
                e.state().zones.list(ZoneLocation::Hand(p0)).len(),
                hand + selected.len()
            );
            for id in &selected {
                assert!(e.state().zones.list(ZoneLocation::Hand(p0)).contains(id));
            }
            let events = &e.journal().entries()[usize::try_from(since).unwrap()..];
            assert!(events.iter().any(|x| matches!(x.event,crate::event::GameEvent::Shuffled{player,zone:Zone::Library} if player==p0)));
            if !selected.is_empty() {
                assert!(events.iter().any(|x| matches!(&x.event,crate::event::GameEvent::Revealed{player,cards} if *player==p0 && cards==&selected)));
            }
        }
    }
}
