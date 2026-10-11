//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn top_card_land_choices_are_private_optional_and_leave_unchosen_cards_untouched() {
    let p0 = PlayerId::new(0);
    for (land, index, matching) in [
        (index::SCRYING_SHEETS, 1, index::SNOW_COVERED_FOREST),
        (index::BUCOLIC_RANCH, 2, index::GRIZZLY_BEARS),
        (index::THE_BIBLIOPLEX, 1, index::LIGHTNING_BOLT),
    ] {
        for matches in [false, true] {
            for take in [false, true] {
                for away in [false, true] {
                    let top = if matches {
                        matching
                    } else {
                        index::GRIZZLY_BEARS
                    };
                    let mut e = Duel::new(SEED, forest())
                        .battlefield(0, &[land])
                        .hand(0, &[top])
                        .start();
                    keep_mulligans(&mut e);
                    reach_main_phase(&mut e, p0);
                    let card = hand_to_library_top(&mut e, p0, top);
                    if matches && land == index::BUCOLIC_RANCH {
                        // The current pool has no Mount creature printing: give this
                        // public rules fixture the subtype that the filter must read.
                        e.dev_state_mut(p0)
                            .unwrap()
                            .object_mut(card)
                            .unwrap()
                            .base_mut()
                            .subtypes
                            .insert(baylee_core::generated::subtypes::creature::MOUNT);
                    }
                    let version = e.state().object(card).unwrap().version;
                    e.dev_state_mut(p0).unwrap().players[0]
                        .mana_pool
                        .add_snow(ManaColor::Colorless, 3);
                    e.refresh_offer();
                    activate(&mut e, p0, land, index);
                    pass_until(&mut e, |e| {
                        matches!(e.pending(), Pending::ChooseCards { .. })
                    });
                    assert_eq!(e.inspected_cards(), &[card]);
                    let Pending::ChooseCards { options, max, .. } = e.pending() else {
                        unreachable!()
                    };
                    assert_eq!(options.contains(&card), matches);
                    assert_eq!(*max, u8::from(matches));
                    let accepted = matches && take;
                    let since = e.state().journal.last_seq();
                    e.apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: if accepted { vec![card] } else { vec![] },
                        },
                    )
                    .unwrap();
                    let moved = away && !accepted && land != index::SCRYING_SHEETS;
                    if !accepted && land != index::SCRYING_SHEETS {
                        assert_eq!(e.inspected_cards(), &[card]);
                        e.apply(
                            p0,
                            PlayerAction::ChooseObjects {
                                objects: if away { vec![card] } else { vec![] },
                            },
                        )
                        .unwrap();
                    }
                    pass_until(&mut e, stack_is_empty);
                    let zone = if accepted {
                        Zone::Hand
                    } else if moved && land == index::THE_BIBLIOPLEX {
                        Zone::Graveyard
                    } else {
                        Zone::Library
                    };
                    assert_eq!(e.state().object(card).unwrap().zone, zone);
                    if zone == Zone::Library {
                        assert_eq!(
                            e.state().object(card).unwrap().version,
                            version,
                            "reordering preserves identity"
                        );
                        let lib = e.state().zones.list(ZoneLocation::Library(p0));
                        assert_eq!(if moved { lib.first() } else { lib.last() }, Some(&card));
                    }
                    let revealed = e.state().journal.entries().iter()
                        .filter(|entry| entry.seq > since)
                        .any(|entry| matches!(
                            &entry.event,
                            crate::event::GameEvent::Revealed { cards, .. } if cards.contains(&card)
                        ));
                    assert_eq!(revealed, accepted, "only the accepted match is public");
                }
            }
        }
    }
}
#[test]
fn biblioplex_activates_only_with_zero_or_seven_cards_and_does_not_recheck_on_resolution() {
    let p0 = PlayerId::new(0);
    for size in [0, 1, 6, 7, 8] {
        let cards = vec![index::GRIZZLY_BEARS; size];
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::THE_BIBLIOPLEX])
            .hand(0, &cards)
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let land = on_battlefield(&e, p0, index::THE_BIBLIOPLEX).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 2);
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            unreachable!()
        };
        assert_eq!(legal.abilities.contains(&(land, 1)), size == 0 || size == 7);
        if size == 7 {
            activate(&mut e, p0, index::THE_BIBLIOPLEX, 1);
            hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseCards { .. })
            });
            assert_eq!(e.inspected_cards().len(), 1);
        }
    }
}
