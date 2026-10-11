#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch45_mastery_casts_card_copies_in_chosen_order_and_keeps_originals_exiled() {
    let mut e = prepared(
        &[
            index::MIZZIX_S_MASTERY,
            index::COUNSEL_OF_THE_SORATAMI,
            index::OPEN_COMMUNICATIONS,
        ],
        &[],
        &[],
    );
    let a = hand_to_graveyard(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    let b = hand_to_graveyard(&mut e, P0, index::OPEN_COMMUNICATIONS);
    let mastery = in_hand(&e, P0, index::MIZZIX_S_MASTERY).unwrap();
    cast(&mut e, mastery);
    mode(&mut e, CastModeKind::Mode(1));
    pass_until(&mut e, exile_choice);
    let copies = cards_offered(&e);
    assert_eq!(copies.len(), 2);
    assert!(!copies.contains(&a) && !copies.contains(&b));
    for (offset, copy) in copies.iter().rev().copied().enumerate() {
        assert_eq!(e.state.object(mastery).unwrap().zone, Zone::Stack);
        choose(&mut e, vec![copy]);
        assert_eq!(e.state.object(copy).unwrap().zone, Zone::Stack);
        assert_eq!(e.state.per_turn.spells_cast[0], 2 + offset as u32);
    }
    assert_eq!(e.state.object(mastery).unwrap().zone, Zone::Exile);
    pass_until(&mut e, stack_is_empty);
    for original in [a, b] {
        assert_eq!(e.state.object(original).unwrap().zone, Zone::Exile);
    }
    for copy in copies {
        assert!(e.state.object(copy).is_none());
    }
}

#[test]
fn batch45_mastery_declining_a_copy_never_grants_a_later_cast() {
    let mut e = prepared(
        &[index::MIZZIX_S_MASTERY, index::COUNSEL_OF_THE_SORATAMI],
        &[],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    cast_with_floating(&mut e, P0, index::MIZZIX_S_MASTERY);
    mode(&mut e, CastModeKind::Mode(0));
    target(&mut e, vec![spell], vec![]);
    pass_until(&mut e, exile_choice);
    let copy = cards_offered(&e)[0];
    choose(&mut e, vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(e.state.object(copy).is_none());
    assert!(!castable(&e, spell));
}

#[test]
fn batch45_ring_replaces_only_one_draw_and_never_offers_exiled_cards() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::RING_OF_MA_RUF])
        .hand(0, &[index::COUNSEL_OF_THE_SORATAMI, index::GRIZZLY_BEARS])
        .sideboard(0, &[index::SOL_RING])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    pool(&mut e, ManaColor::Blue, 8);
    let bear = hand_to(&mut e, P0, index::GRIZZLY_BEARS, ZoneLocation::Exile(P0));
    activate(&mut e, P0, index::RING_OF_MA_RUF, 0);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.per_turn.draws[0];
    cast_with_floating(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Wish,
                ..
            }
        )
    });
    let options = cards_offered(&e);
    assert_eq!(options.len(), 1);
    assert!(!options.contains(&bear));
    choose(&mut e, options.clone());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(options[0]).unwrap().zone, Zone::Hand);
    assert_eq!(e.state.per_turn.draws[0], before + 1);
    assert!(e.state.draw_wish_effect(P0, None).is_none());
}

#[test]
fn batch45_proxy_empowers_one_jace_and_reuses_it_on_the_next_entry() {
    let mut e = prepared(
        &[index::THEORIST_S_PROXY, index::THEORIST_S_PROXY],
        &[],
        &[],
    );
    for loyalty in [3, 6] {
        cast_with_floating(&mut e, P0, index::THEORIST_S_PROXY);
        pass_until(&mut e, stack_is_empty);
        let jaces: Vec<_> = e
            .state
            .battlefield_seen()
            .filter(|id| {
                e.state
                    .object(*id)
                    .unwrap()
                    .characteristics()
                    .subtypes
                    .contains(subtypes::planeswalker::JACE)
            })
            .collect();
        assert_eq!(jaces.len(), 1);
        assert_eq!(counters_on(&e, jaces[0], CounterKind::Loyalty), loyalty);
    }
}

#[test]
fn batch45_village_and_proxy_grant_only_the_next_spell_uncounterability() {
    for (source, ability) in [(index::MISTRISE_VILLAGE, 1), (index::THEORIST_S_PROXY, 1)] {
        let mut e = prepared(
            &[index::COUNSEL_OF_THE_SORATAMI, index::OPEN_COMMUNICATIONS],
            &[source],
            &[],
        );
        activate(&mut e, P0, source, ability);
        pass_until(&mut e, stack_is_empty);
        let first = in_hand(&e, P0, index::COUNSEL_OF_THE_SORATAMI).unwrap();
        cast(&mut e, first);
        assert!(
            e.state
                .object(first)
                .unwrap()
                .riders
                .contains(&Rider::Uncounterable)
        );
        pass_until(&mut e, stack_is_empty);
        let second = in_hand(&e, P0, index::OPEN_COMMUNICATIONS).unwrap();
        cast(&mut e, second);
        assert!(
            !e.state
                .object(second)
                .unwrap()
                .riders
                .contains(&Rider::Uncounterable)
        );
    }
}

#[test]
fn batch45_pakpatiq_grants_rebound_and_its_temple_can_tap_without_a_counter() {
    let mut e = prepared(
        &[index::GIANT_GROWTH],
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH, index::GRIZZLY_BEARS],
        &[],
    );
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let spell = in_hand(&e, P0, index::GIANT_GROWTH).unwrap();
    cast(&mut e, spell);
    target(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Exile);
    let ojer = on_battlefield(&e, P0, index::OJER_PAKPATIQ_DEEPEST_EPOCH).unwrap();
    kill(&mut e, ojer);
    assert_eq!(e.state.object(ojer).unwrap().face_index, 1);
    assert_eq!(counters_on(&e, ojer, CounterKind::Time), 3);
    for remaining in [2, 1, 0, 0] {
        e.state.set_tapped(ojer, false);
        e.refresh_offer();
        activate(&mut e, P0, index::OJER_PAKPATIQ_DEEPEST_EPOCH, 0);
        assert_eq!(counters_on(&e, ojer, CounterKind::Time), remaining);
    }
    e.state.set_tapped(ojer, false);
    e.refresh_offer();
    activate(&mut e, P0, index::OJER_PAKPATIQ_DEEPEST_EPOCH, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ojer).unwrap().face_index, 0);
}

#[test]
fn batch45_welcome_animates_an_opponents_artifact_then_creates_a_hasty_dinosaur() {
    let mut e = prepared(&[index::WELCOME_TO], &[], &[index::SOL_RING]);
    let ring = on_battlefield(&e, P1, index::SOL_RING).unwrap();
    cast_with_floating(&mut e, P0, index::WELCOME_TO);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    target(&mut e, vec![ring], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, ring), (0, 4));
    assert!(keywords(&e, ring).contains(KeywordSet::DEFENDER));
    let saga = on_battlefield(&e, P0, index::WELCOME_TO).unwrap();
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    pass_until(&mut e, |e| {
        e.state.battlefield_seen().any(|id| {
            e.state
                .object(id)
                .unwrap()
                .characteristics()
                .subtypes
                .contains(subtypes::creature::DINOSAUR)
        })
    });
    let dinosaur = e
        .state
        .battlefield_seen()
        .find(|id| {
            e.state
                .object(*id)
                .unwrap()
                .characteristics()
                .subtypes
                .contains(subtypes::creature::DINOSAUR)
        })
        .unwrap();
    assert_eq!(pt(&e, dinosaur), (3, 3));
    assert!(keywords(&e, dinosaur).contains(KeywordSet::HASTE.union(KeywordSet::TRAMPLE)));
    pass_until(&mut e, stack_is_empty);
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    pass_until(&mut e, |e| e.state.object(saga).unwrap().face_index == 1);
    assert_eq!(e.state.object(ring).unwrap().zone, Zone::Graveyard);
    pass_until(&mut e, stack_is_empty);
    e.state.players[0].mana_pool = ManaPool::new();
    e.refresh_offer();
    activate(&mut e, P0, index::WELCOME_TO, 1);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Green), 1);
}
