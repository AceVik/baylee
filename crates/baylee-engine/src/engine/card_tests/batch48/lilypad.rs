//! Historical entry facts and real card actions.
#[allow(clippy::wildcard_imports)]
use super::*;

fn surveil(e: &mut Engine<RegistryLookup>, village: ObjectId) {
    priority(e, P0);
    e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    e.refresh_offer();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: village,
            ability_index: 2,
        },
    )
    .unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange { cards, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(cards.len(), 2);
    // Existing surveil two-pile answer, keeping both cards on top.
    e.apply(
        P0,
        PlayerAction::Arrange {
            piles: vec![cards, vec![]],
        },
    )
    .unwrap();
    pass_until(e, stack_is_empty);
}

#[test]
fn batch48_lilypad_retains_entry_after_departure_control_and_type_changes() {
    for change in 0..3 {
        let mut e = prepared(
            &[index::SEWER_RATS, index::UNSUMMON],
            &[index::LILYPAD_VILLAGE],
            &[],
        );
        let village = source(&e, P0, index::LILYPAD_VILLAGE);
        assert!(
            e.apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: village,
                    ability_index: 2
                }
            )
            .is_err()
        );
        cast(&mut e, index::SEWER_RATS);
        pass_until(&mut e, stack_is_empty);
        let rat = source(&e, P0, index::SEWER_RATS);
        match change {
            0 => {
                cast(&mut e, index::UNSUMMON);
                targets(&mut e, vec![rat], vec![]);
                pass_until(&mut e, stack_is_empty);
            }
            1 => {
                // Isolate a subsequent control change after the real creature cast.
                let object = e.state.object_mut(rat).unwrap();
                object.controller = P1;
                object.base_controller = P1;
                e.state.invalidate_projections();
            }
            _ => {
                e.state.object_mut(rat).unwrap().base_mut().subtypes =
                    baylee_core::types::SubtypeSet::EMPTY;
                e.state.invalidate_projections();
            }
        }
        surveil(&mut e, village);
        assert!(is_tapped(&e, village));
    }
}

#[test]
fn batch48_lilypad_does_not_count_an_opponents_entry_or_last_turns_entry() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::LILYPAD_VILLAGE])
        .hand(1, &[index::SEWER_RATS])
        .start();
    keep_mulligans(&mut e);
    next_main(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::Black, 1);
    let rat = in_hand(&e, P1, index::SEWER_RATS).unwrap();
    e.refresh_offer();
    e.apply(P1, PlayerAction::CastSpell { card: rat }).unwrap();
    pass_until(&mut e, stack_is_empty);
    priority(&mut e, P0);
    let village = source(&e, P0, index::LILYPAD_VILLAGE);
    e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    e.refresh_offer();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: village,
                ability_index: 2
            }
        )
        .is_err()
    );

    let mut own = prepared(&[index::SEWER_RATS], &[index::LILYPAD_VILLAGE], &[]);
    cast(&mut own, index::SEWER_RATS);
    pass_until(&mut own, stack_is_empty);
    let village = source(&own, P0, index::LILYPAD_VILLAGE);
    next_main(&mut own, P1);
    priority(&mut own, P0);
    own.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    own.refresh_offer();
    assert!(
        own.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: village,
                ability_index: 2
            }
        )
        .is_err()
    );
}

#[test]
fn batch48_lilypad_restricted_blue_cannot_pay_its_own_surveil_cost() {
    let mut e = prepared(&[index::SEWER_RATS], &[index::LILYPAD_VILLAGE], &[]);
    cast(&mut e, index::SEWER_RATS);
    pass_until(&mut e, stack_is_empty);
    let village = source(&e, P0, index::LILYPAD_VILLAGE);
    e.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    e.refresh_offer();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: village,
            ability_index: 1,
        },
    )
    .unwrap();
    e.state.set_tapped(village, false); // Isolate the mana restriction from the tap cost.
    e.refresh_offer();
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: village,
                ability_index: 2
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
}

#[test]
fn batch48_lilypad_counts_entry_before_village_was_present() {
    let mut e = prepared(&[index::SEWER_RATS, index::LILYPAD_VILLAGE], &[], &[]);
    cast(&mut e, index::SEWER_RATS);
    pass_until(&mut e, stack_is_empty);
    let village = in_hand(&e, P0, index::LILYPAD_VILLAGE).unwrap();
    priority(&mut e, P0);
    e.apply(P0, PlayerAction::PlayLand { card: village })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    surveil(&mut e, village);
    assert!(is_tapped(&e, village));
}

fn next_main(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == player
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player)
    });
}

#[test]
fn batch48_lilypad_records_projected_entry_before_nexus_leaves() {
    let mut e = prepared(
        &[index::GRIZZLY_BEARS, index::DISENCHANT],
        &[index::LILYPAD_VILLAGE, index::MASKWOOD_NEXUS],
        &[],
    );
    let village = source(&e, P0, index::LILYPAD_VILLAGE);
    let nexus = source(&e, P0, index::MASKWOOD_NEXUS);
    cast(&mut e, index::GRIZZLY_BEARS);
    pass_until(&mut e, stack_is_empty);
    cast(&mut e, index::DISENCHANT);
    targets(&mut e, vec![nexus], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state
            .object(source(&e, P0, index::GRIZZLY_BEARS))
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::BEAR)
    );
    surveil(&mut e, village);
}

#[test]
fn batch48_lilypad_records_token_and_copy_replacement_entries() {
    for token in [true, false] {
        let hand = if token {
            vec![index::RAISE_THE_ALARM]
        } else {
            vec![index::CLONE]
        };
        let board = if token {
            vec![index::LILYPAD_VILLAGE, index::MASKWOOD_NEXUS]
        } else {
            vec![index::LILYPAD_VILLAGE]
        };
        let theirs = if token {
            vec![]
        } else {
            vec![index::SEWER_RATS]
        };
        let mut e = prepared(&hand, &board, &theirs);
        let village = source(&e, P0, index::LILYPAD_VILLAGE);
        if token {
            cast(&mut e, index::RAISE_THE_ALARM);
        } else {
            let rat = source(&e, P1, index::SEWER_RATS);
            cast(&mut e, index::CLONE);
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseTargets { .. })
            });
            choose(&mut e, vec![rat]);
        }
        pass_until(&mut e, stack_is_empty);
        surveil(&mut e, village);
    }
}
