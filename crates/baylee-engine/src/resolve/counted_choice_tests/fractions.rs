//! Fractions and per-player simultaneous choices (CR 101.4, 608.2e).

use super::*;
use baylee_cards_dsl::{Amount, Filter, PlayerRel};

const OTHER: PlayerId = PlayerId::new(1);

fn cards(state: &mut GameState, player: PlayerId, count: usize) -> Vec<ObjectId> {
    let name = state.names.intern("Private choice");
    (0..count)
        .map(|_| state.create_bare(player, ObjectKind::Card, name, ZoneLocation::Hand(player)))
        .collect()
}

#[track_caller]
fn choice(flow: Flow, expected_player: PlayerId, expected_count: u8) -> Vec<ObjectId> {
    let Flow::Wait(Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    }) = flow
    else {
        panic!("expected a counted choice");
    };
    assert_eq!(player, expected_player);
    assert_eq!((min, max), (expected_count, expected_count));
    options
}

#[test]
fn division_rounds_without_overflow_and_preserves_every_amount_context() {
    let (mut state, _) = library_of(0);
    let mut res = mine(Effect::draw(0));
    for life in [-10, 0, 1, 2, 3, 4, 20, i32::MAX] {
        state.players[0].life = life;
        state.players[1].life = 13;
        for divisor in [1, 2, 3, 7, u32::MAX] {
            for round_up in [false, true] {
                let expected = |n: u32| {
                    let n = u64::from(n);
                    let divisor = u64::from(divisor);
                    u32::try_from((n + if round_up { divisor - 1 } else { 0 }) / divisor).unwrap()
                };
                let own = Amount::divided(&Amount::LifeTotal(PlayerRel::You), divisor, round_up);
                assert_eq!(
                    amount2(&own, &state, me(), &res),
                    expected(life.max(0) as u32)
                );
                assert_eq!(
                    eval::amount(&own, &state, me(), res.source, None),
                    expected(life.max(0) as u32)
                );
                let recipient =
                    Amount::divided(&Amount::LifeTotal(PlayerRel::Recipient), divisor, round_up);
                assert_eq!(
                    amount_for_player(&recipient, &state, me(), &res, OTHER),
                    expected(13)
                );
                let x = Amount::divided(&Amount::X, divisor, round_up);
                res.x = Some(u32::MAX);
                assert_eq!(amount2(&x, &state, me(), &res), expected(u32::MAX));
                assert_eq!(
                    amount_for_player(&x, &state, me(), &res, OTHER),
                    expected(u32::MAX)
                );
                assert_eq!(
                    eval::amount(&x, &state, me(), res.source, res.x),
                    expected(u32::MAX)
                );
            }
        }
    }
}

#[test]
#[should_panic(expected = "amount divisor must be nonzero")]
fn zero_divisor_is_rejected_at_construction() {
    let _ = Amount::divided(&Amount::Fixed(1), 0, true);
}

#[test]
fn fractional_discards_collect_private_choices_in_apnap_order_before_any_move() {
    let (mut state, _) = library_of(0);
    state.turn.active = OTHER;
    let mine = cards(&mut state, me(), 4);
    let theirs = cards(&mut state, OTHER, 7);
    let mut res = super::mine(Effect::DiscardAmountForPlayers {
        who: PlayerRel::EachPlayer,
        amount: Amount::divided(&Amount::HandSize(PlayerRel::Recipient), 3, true),
    });
    let since = state.journal.last_seq();
    assert_eq!(choice(run(&mut state, &mut res), OTHER, 4), theirs);
    assert_eq!(
        choice(resume(&mut state, &mut res, &theirs[..4]), me(), 2),
        mine
    );
    assert_eq!(state.zones.list(ZoneLocation::Hand(me())), &mine);
    assert_eq!(state.zones.list(ZoneLocation::Hand(OTHER)), &theirs);
    assert!(
        state
            .journal
            .entries()
            .iter()
            .filter(|entry| entry.seq > since)
            .all(|entry| {
                !matches!(
                    entry.event,
                    GameEvent::Revealed { .. }
                        | GameEvent::CardsKept { .. }
                        | GameEvent::Discarded { .. }
                )
            }),
        "private choices must remain private until all players decide"
    );
    assert!(matches!(
        resume(&mut state, &mut res, &mine[..2]),
        Flow::Complete
    ));
    assert_eq!(state.zones.list(ZoneLocation::Hand(me())), &mine[..2]);
    assert_eq!(state.zones.list(ZoneLocation::Hand(OTHER)), &theirs[..4]);
}

#[test]
fn fractional_choices_larger_than_a_byte_keep_every_card_selected() {
    let (mut state, _) = library_of(0);
    let hand = cards(&mut state, me(), 400);
    let mut res = mine(Effect::DiscardAmountForPlayers {
        who: PlayerRel::You,
        amount: Amount::divided(&Amount::HandSize(PlayerRel::Recipient), 3, true),
    });
    assert_eq!(choice(run(&mut state, &mut res), me(), 255), hand);
    assert_eq!(
        choice(resume(&mut state, &mut res, &hand[..255]), me(), 11),
        hand[255..]
    );
    assert_eq!(state.zones.list(ZoneLocation::Hand(me())).len(), 400);
    assert!(matches!(
        resume(&mut state, &mut res, &hand[255..266]),
        Flow::Complete
    ));
    assert_eq!(state.zones.list(ZoneLocation::Hand(me())), &hand[..266]);
    assert_eq!(state.zones.list(ZoneLocation::Graveyard(me())).len(), 134);
}

#[test]
fn all_discard_choices_precede_replacement_questions_and_every_discard() {
    use baylee_cards_dsl::ReplacementRule;
    let (mut state, _) = library_of(0);
    let mine = cards(&mut state, me(), 3);
    let theirs = cards(&mut state, OTHER, 3);
    for player in [me(), OTHER] {
        let name = state.names.intern("Discard replacement");
        let source = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state
            .replacement_rules
            .push(crate::state::ReplacementEntry {
                source,
                controller: player,
                rule: ReplacementRule::MayDiscardToLibraryTop,
            });
    }
    let mut res = super::mine(Effect::DiscardAmountForPlayers {
        who: PlayerRel::EachPlayer,
        amount: Amount::divided(&Amount::HandSize(PlayerRel::Recipient), 3, true),
    });
    choice(run(&mut state, &mut res), me(), 2);
    choice(resume(&mut state, &mut res, &mine[..2]), OTHER, 2);
    let Flow::Wait(Pending::Arrange {
        player,
        cards: arranged,
        ..
    }) = resume(&mut state, &mut res, &theirs[..2])
    else {
        panic!("first player's replacement follows all selections");
    };
    assert_eq!(player, me());
    assert_eq!(arranged, mine[2..]);
    let Flow::Wait(Pending::Arrange {
        player,
        cards: arranged,
        ..
    }) = resume_arranged(&mut state, &mut res, &[vec![], mine[2..].to_vec()])
    else {
        panic!("second player's replacement precedes every discard");
    };
    assert_eq!(player, OTHER);
    assert_eq!(arranged, theirs[2..]);
    assert_eq!(state.zones.list(ZoneLocation::Hand(me())), &mine);
    assert_eq!(state.zones.list(ZoneLocation::Hand(OTHER)), &theirs);
    assert!(matches!(
        resume_arranged(&mut state, &mut res, &[theirs[2..].to_vec(), vec![]]),
        Flow::Complete
    ));
    assert_eq!(
        state.zones.list(ZoneLocation::Library(me())).last(),
        Some(&mine[2])
    );
    assert_eq!(
        state.zones.list(ZoneLocation::Graveyard(OTHER)),
        &theirs[2..]
    );
    assert_eq!(state.zones.list(ZoneLocation::Hand(me())), &mine[..2]);
    assert_eq!(state.zones.list(ZoneLocation::Hand(OTHER)), &theirs[..2]);
}

#[test]
fn each_sacrifice_instruction_recounts_after_previous_simultaneous_departures() {
    use baylee_core::types::TypeSet;
    let (mut state, _) = library_of(0);
    let mut groups = Vec::new();
    for player in [me(), OTHER] {
        let mut group = Vec::new();
        for land in [true, false, false, true, true] {
            let name = state.names.intern("Permanent choice");
            let id = state.create_bare(
                player,
                ObjectKind::Permanent,
                name,
                ZoneLocation::Battlefield,
            );
            let object = state.object_mut(id).unwrap();
            object.base_mut().types = if land {
                TypeSet::LAND
            } else {
                TypeSet::CREATURE
            };
            group.push(id);
        }
        state.object_mut(group[0]).unwrap().base_mut().types =
            TypeSet::LAND.union(TypeSet::CREATURE);
        groups.push(group);
    }
    let mut res = mine(Effect::SacrificeAmountForPlayers {
        who: PlayerRel::EachPlayer,
        filter: &Filter::CREATURE,
        amount: Amount::divided(
            &Amount::ControlledCount {
                who: PlayerRel::Recipient,
                filter: &Filter::CREATURE,
            },
            3,
            true,
        ),
    });
    res.effects.push(Effect::SacrificeAmountForPlayers {
        who: PlayerRel::EachPlayer,
        filter: &Filter::LAND,
        amount: Amount::divided(
            &Amount::ControlledCount {
                who: PlayerRel::Recipient,
                filter: &Filter::LAND,
            },
            3,
            true,
        ),
    });
    choice(run(&mut state, &mut res), me(), 2);
    choice(resume(&mut state, &mut res, &groups[0][1..3]), OTHER, 2);
    assert!(
        groups
            .iter()
            .flatten()
            .all(|id| state.object(*id).unwrap().zone == crate::zone::Zone::Battlefield)
    );
    assert!(state.journal.entries().iter().any(|entry| matches!(&entry.event,
        GameEvent::CardsKept { player, cards } if *player == me() && cards == &groups[0][1..3]
    )), "the next player sees the earlier public permanent choice");
    assert_eq!(
        choice(resume(&mut state, &mut res, &groups[1][1..3]), me(), 1),
        groups[0][3..5]
    );
    assert_eq!(
        choice(resume(&mut state, &mut res, &groups[0][3..4]), OTHER, 1),
        groups[1][3..5]
    );
    assert!(matches!(
        resume(&mut state, &mut res, &groups[1][3..4]),
        Flow::Complete
    ));
    for (player, group) in [me(), OTHER].into_iter().zip(groups) {
        assert_eq!(
            state.zones.list(ZoneLocation::Graveyard(player)),
            &[group[0], group[4]]
        );
    }
}
