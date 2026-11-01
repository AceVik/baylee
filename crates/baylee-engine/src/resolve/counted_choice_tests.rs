use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};

mod fractions;
mod target_groups;

fn me() -> PlayerId {
    PlayerId::new(0)
}

/// A two-seat game whose seat 0 has `cards` cards in its library.
fn library_of(cards: usize) -> (GameState, Vec<ObjectId>) {
    let mut state = GameState::from_preset(&preset(311, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let library = state.zones.list(ZoneLocation::Library(me())).clone();
    for card in library {
        let _ = state.move_object(
            card,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    let name = state.names.intern("Card");
    let made = (0..cards)
        .map(|_| state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Library(me())))
        .collect();
    (state, made)
}

fn mine(effect: Effect) -> Resolution {
    Resolution {
        source: ObjectId::NO_SOURCE,
        on_stack: ObjectId::NO_SOURCE,
        controller: me(),
        effects: vec![effect],
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    }
}

/// "Put two of them into your hand" over a library of one asks for the
/// one (CR 609.3). It asked for two, and nothing could answer.
#[test]
fn a_look_that_picks_more_than_the_library_holds_asks_for_what_is_there() {
    let (mut state, cards) = library_of(1);
    let mut res = mine(Effect::LookAtTopPick {
        count: baylee_cards_dsl::Amount::Fixed(7),
        pick: 2,
        random: false,
    });
    let Flow::Wait(Pending::ChooseCards {
        options, min, max, ..
    }) = run(&mut state, &mut res)
    else {
        panic!("the look asks");
    };
    assert_eq!(options, cards);
    assert_eq!((min, max), (1, 1));
}

/// A search told to find two cards over a library holding one finds as
/// many as possible (CR 701.23d): one.
#[test]
fn a_search_for_more_than_the_library_holds_finds_what_is_there() {
    const TWO: &[baylee_cards_dsl::effect::Find] = &[
        baylee_cards_dsl::effect::Find::HAND,
        baylee_cards_dsl::effect::Find::HAND,
    ];
    let (mut state, cards) = library_of(1);
    let mut res = mine(Effect::SearchLibrary {
        filter: &baylee_cards_dsl::Filter::Any,
        finds: TWO,
        optional: false,
    });
    let Flow::Wait(Pending::ChooseCards {
        options, min, max, ..
    }) = run(&mut state, &mut res)
    else {
        panic!("the search asks");
    };
    assert_eq!(options, cards);
    assert_eq!((min, max), (1, 1));
}

#[test]
fn computed_arithmetic_saturates_and_keeps_resolution_context() {
    use baylee_cards_dsl::Amount;
    let (mut state, _) = library_of(0);
    let mut res = mine(Effect::draw(0));
    let name = state.names.intern("Amount source");
    let source = state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Battlefield);
    res.source = source;
    let name = state.names.intern("Amount target");
    let target = state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    state.object_mut(target).unwrap().base_mut().power = Some(7);
    res.targets.push(target);
    let nested = Amount::Difference {
        left: &Amount::Times {
            base: &Amount::TargetPower,
            factor: 3,
        },
        right: &Amount::Fixed(4),
    };
    assert_eq!(amount2(&nested, &state, me(), &res), 17);
    assert_eq!(
        amount2(&Amount::Negated(&Amount::TargetPower), &state, me(), &res),
        7
    );
    let mut sample = 1_u32;
    for _ in 0..10_000 {
        sample = sample.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        for x in [0, 1, sample, u32::MAX] {
            res.x = Some(x);
            for round_up in [false, true] {
                let half = Amount::Half {
                    base: &Amount::X,
                    round_up,
                };
                let expected = u32::try_from(u64::from(x).midpoint(u64::from(round_up))).unwrap();
                assert_eq!(amount2(&half, &state, me(), &res), expected);
                assert_eq!(eval::amount(&half, &state, me(), source, Some(x)), expected);
            }
            let sum = Amount::Sum {
                left: &Amount::X,
                right: &Amount::Fixed(13),
            };
            let expected_sum = u32::try_from(u64::from(x) + 13).unwrap_or(u32::MAX);
            assert_eq!(amount2(&sum, &state, me(), &res), expected_sum);
            assert_eq!(
                eval::amount(&sum, &state, me(), source, Some(x)),
                expected_sum
            );
            let scaled = Amount::Times {
                base: &Amount::X,
                factor: 3,
            };
            let difference = Amount::Difference {
                left: &Amount::Fixed(4),
                right: &Amount::X,
            };
            let expected = u32::try_from(u64::from(x) * 3).unwrap_or(u32::MAX);
            assert_eq!(amount2(&scaled, &state, me(), &res), expected);
            assert_eq!(
                eval::amount(&scaled, &state, me(), source, Some(x)),
                expected
            );
            assert_eq!(
                amount2(&difference, &state, me(), &res),
                4_u32.saturating_sub(x)
            );
            assert_eq!(
                eval::amount(&difference, &state, me(), source, Some(x)),
                4_u32.saturating_sub(x)
            );
        }
    }
}
