//! `cards/sorceries/mv_2/balance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// An empty opposing board/hand forces every surplus card to go, without menus.
#[test]
fn balance_pays_its_printed_cost_and_zero_minima_need_no_choices() {
    let p0 = PlayerId::new(0);
    let card = card_index("17fa98cd-ed8f-483f-9525-7e989a82ebb2");
    let mut engine = Duel::new(1001, forest())
        .battlefield(0, &[plains(); 2])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, card);
    assert!(on_stack(&engine, card).is_some(), "the card was cast");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed cost was paid"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, card).is_some());
    assert!(in_hand(&engine, p0, card).is_none());
    assert!(lands_of(&engine, p0).is_empty());
}

/// Balance collects an entire stage in APNAP order before any object moves.
#[test]
#[allow(clippy::too_many_lines)] // One spell, all three simultaneous stages and three seats.
fn balance_three_players_choose_in_active_order_and_move_each_group_together() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let balance = card_index("17fa98cd-ed8f-483f-9525-7e989a82ebb2");
    let mut engine = Duel::table(1071, forest(), 3)
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                quiet_creature(),
                quiet_creature(),
                sol_ring(),
            ],
        )
        .battlefield(
            1,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                quiet_creature(),
                quiet_creature(),
                quiet_creature(),
            ],
        )
        .battlefield(2, &[plains(), quiet_creature()])
        .hand(0, &[island(), mountain(), swamp()])
        .hand(1, &[balance, island(), mountain(), swamp()])
        .hand(2, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, balance);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    for stage in 0..3 {
        let before: Vec<_> = [p0, p1, p2]
            .map(|p| {
                if stage == 1 {
                    engine.state().zones.list(ZoneLocation::Hand(p)).clone()
                } else {
                    engine
                        .state()
                        .battlefield_seen()
                        .filter(|&id| engine.state().object(id).unwrap().controller == p)
                        .collect()
                }
            })
            .into();
        let journal_from = engine.state().journal.len();
        for (turn, expected) in [p1, p0].into_iter().enumerate() {
            let Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } = engine.pending().clone()
            else {
                panic!("{:?}", engine.pending());
            };
            assert_eq!(player, expected);
            assert_eq!(
                prompt,
                [
                    ChoicePrompt::KeepLands,
                    ChoicePrompt::Keep,
                    ChoicePrompt::KeepCreatures
                ][stage]
            );
            assert_eq!((min, max), (1, 1));
            for id in &options {
                let object = engine.state().object(*id).unwrap();
                assert!(match stage {
                    0 => object.characteristics().types.contains(TypeSet::LAND),
                    1 => object.zone == Zone::Hand && object.zone_owner == Some(player),
                    _ => object.characteristics().types.contains(TypeSet::CREATURE),
                });
            }
            engine
                .apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![*options.last().unwrap()],
                    },
                )
                .unwrap();
            if turn == 0 {
                for (seat, cards) in [p0, p1, p2].into_iter().zip(&before) {
                    if stage == 1 {
                        assert_eq!(engine.state().zones.list(ZoneLocation::Hand(seat)), cards);
                    } else {
                        assert!(cards.iter().all(
                            |id| engine.state().object(*id).unwrap().zone == Zone::Battlefield
                        ));
                    }
                }
                let announcements: Vec<_> = engine.state().journal.entries()[journal_from..]
                    .iter()
                    .filter_map(|entry| {
                        if let GameEvent::CardsKept { player, .. } = entry.event {
                            Some(player)
                        } else {
                            None
                        }
                    })
                    .collect();
                if stage == 1 {
                    assert!(announcements.is_empty(), "hand choices are private");
                } else {
                    assert_eq!(
                        announcements,
                        vec![p1, p2],
                        "later choosers know previous public choices"
                    );
                }
            }
        }
        for player in [p0, p1, p2] {
            match stage {
                0 => assert_eq!(lands_of(&engine, player).len(), 1),
                1 => assert_eq!(
                    engine.state().zones.list(ZoneLocation::Hand(player)).len(),
                    1
                ),
                _ => assert_eq!(
                    all_on_battlefield(&engine, player, quiet_creature()).len(),
                    1
                ),
            }
        }
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, sol_ring()).is_some(),
        "unrelated artifacts stay"
    );
    assert!(in_graveyard(&engine, p1, balance).is_some());
}

/// The creature minimum is recomputed after land creatures have been lost.
#[test]
fn balance_land_creature_lost_first_does_not_count_in_the_creature_stage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let balance = card_index("17fa98cd-ed8f-483f-9525-7e989a82ebb2");
    let arbor = card_index("e996cd67-739c-40f4-b276-0042acf26c71");
    let mut engine = Duel::new(1072, forest())
        .battlefield(0, &[plains(), plains(), arbor])
        .battlefield(1, &[forest(), quiet_creature(), quiet_creature()])
        .hand(0, &[balance])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // Do not try to tap the summoning-sick Arbor.
    for source in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let spell = in_hand(&engine, p0, balance).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let kept = on_battlefield(&engine, p0, plains()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![kept],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, arbor).is_some());
    assert!(all_on_battlefield(&engine, p1, quiet_creature()).is_empty());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        2
    );
}

/// Pending's byte-sized count must not truncate a large equalization.
#[test]
fn balance_keeps_more_than_255_cards_in_chunks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let balance = card_index("17fa98cd-ed8f-483f-9525-7e989a82ebb2");
    let mut engine = Duel::new(1073, forest())
        .battlefield(0, &[plains(); 258])
        .battlefield(1, &[forest(); 256])
        .hand(0, &[balance])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    for source in all_on_battlefield(&engine, p0, plains())
        .into_iter()
        .take(2)
    {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let spell = in_hand(&engine, p0, balance).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let mut kept = Vec::new();
    for count in [255, 1] {
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = engine.pending().clone()
        else {
            panic!("{:?}", engine.pending());
        };
        assert_eq!(player, p0);
        assert_eq!((min, max), (count, count));
        assert!(options.iter().all(|id| !kept.contains(id)));
        assert_eq!(lands_of(&engine, p0).len(), 258);
        let chosen = options[..usize::from(count)].to_vec();
        kept.extend_from_slice(&chosen);
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: chosen })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(lands_of(&engine, p0).len(), 256);
    assert_eq!(lands_of(&engine, p1).len(), 256);
    assert!(
        kept.iter()
            .all(|id| engine.state().object(*id).unwrap().zone == Zone::Battlefield)
    );
}

/// Sacrifice is neither destruction nor targeting, even when a choice is made.
#[test]
fn balance_can_choose_protected_creatures_and_sacrifice_indestructible_ones() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let balance = card_index("17fa98cd-ed8f-483f-9525-7e989a82ebb2");
    let gargoyle = card_index("73010421-374f-458e-aa88-248ef8ae4f8b");
    let budoka = card_index("9508efe6-0528-4a13-9961-c2a1b05085a6");
    let knight = card_index("9456c5b6-946d-403a-8ed0-dff9f921d98c");
    let mut engine = Duel::new(1074, forest())
        .battlefield(0, &[plains(), plains(), gargoyle, budoka, knight])
        .battlefield(1, &[plains(), plains(), quiet_creature()])
        .hand(0, &[balance])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, balance);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let keep = on_battlefield(&engine, p0, knight).unwrap();
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending()
    else {
        unreachable!()
    };
    assert_eq!((*min, *max), (1, 1));
    for card in [gargoyle, budoka, knight] {
        assert!(options.contains(&on_battlefield(&engine, p0, card).unwrap()));
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![keep],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, gargoyle).is_some());
    assert!(in_graveyard(&engine, p0, budoka).is_some());
    assert!(on_battlefield(&engine, p0, knight).is_some());
    assert!(on_battlefield(&engine, p1, quiet_creature()).is_some());
}

/// Two private selections with the same board must still hash differently.
#[test]
fn balance_private_keep_decisions_participate_in_snapshot_hashes() {
    let p0 = PlayerId::new(0);
    let balance = card_index("17fa98cd-ed8f-483f-9525-7e989a82ebb2");
    let setup = || {
        let mut engine = Duel::table(1075, forest(), 3)
            .battlefield(0, &[plains(), plains()])
            .hand(0, &[balance, island(), swamp(), mountain()])
            .hand(1, &[island(), swamp(), mountain()])
            .hand(2, &[forest()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        cast_from_hand(&mut engine, p0, balance);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        engine
    };
    let mut left = setup();
    let mut right = setup();
    assert_eq!(left.snapshot_hash(), right.snapshot_hash());
    for (engine, choice) in [(&mut left, 0), (&mut right, 1)] {
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            unreachable!()
        };
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[choice]],
                },
            )
            .unwrap();
    }
    assert_eq!(
        left.state().snapshot_hash(),
        right.state().snapshot_hash(),
        "no card moved or private choice was journaled"
    );
    assert_ne!(left.snapshot_hash(), right.snapshot_hash());
}
