//! Resolution boundaries shared by the batch's real cards.
#[allow(clippy::wildcard_imports)] // Batch scenarios share the card harness.
use super::*;

#[test]
fn batch45_empower_chooses_one_of_doubled_tokens_before_zero_loyalty_sbas() {
    let mut e = prepared(&[index::THEORIST_S_PROXY], &[index::DOUBLING_SEASON], &[]);
    cast_with_floating(&mut e, P0, index::THEORIST_S_PROXY);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Generic,
                ..
            }
        )
    });
    let tokens = cards_offered(&e);
    assert_eq!(tokens.len(), 2);
    for token in &tokens {
        assert_eq!(counters_on(&e, *token, CounterKind::Loyalty), 0);
    }
    choose(&mut e, vec![tokens[1]]);
    pass_until(&mut e, stack_is_empty);
    assert!(e.state.object(tokens[0]).is_none());
    assert_eq!(counters_on(&e, tokens[1], CounterKind::Loyalty), 6);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: tokens[1],
            ability_index: 1,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, tokens[1], CounterKind::Loyalty), 3);
    assert_eq!(e.state.per_turn.draws[0], 1);
}

#[test]
fn batch45_pakpatiq_returns_to_its_owner_who_places_the_entry_counters() {
    let mut e = prepared(
        &[],
        &[],
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH, index::DOUBLING_SEASON],
    );
    let ojer = on_battlefield(&e, P1, index::OJER_PAKPATIQ_DEEPEST_EPOCH).unwrap();
    // The death trigger belongs to the former controller, while its owner
    // controls the returning Temple and puts the entry counters on it.
    e.state.object_mut(ojer).unwrap().set_controller(P0);
    e.state.invalidate_projections();
    e.refresh_offer();
    kill(&mut e, ojer);
    assert_eq!(e.state.object(ojer).unwrap().controller, P1);
    assert_eq!(counters_on(&e, ojer, CounterKind::Time), 6);
    assert!(e.state.journal.entries().iter().any(|entry| matches!(entry.event,
        crate::event::GameEvent::CounterChanged { object, kind: CounterKind::Time, by: Some(P1), new: 6, .. } if object == ojer)));
}

#[test]
fn batch45_mistrise_protects_a_spell_against_an_actual_counterspell() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::MISTRISE_VILLAGE])
        .hand(0, &[index::GRIZZLY_BEARS])
        .hand(1, &[index::COUNTERSPELL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for player in &mut e.state.players {
        for color in ManaColor::ALL {
            player.mana_pool.add(color, 10);
        }
    }
    e.refresh_offer();
    activate(&mut e, P0, index::MISTRISE_VILLAGE, 1);
    pass_until(&mut e, stack_is_empty);
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    cast(&mut e, bear);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    let counter = in_hand(&e, P1, index::COUNTERSPELL).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: counter })
        .unwrap();
    target(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.object(counter).unwrap().zone, Zone::Graveyard);
}

#[test]
fn batch45_karn_in_commander_can_still_choose_face_up_exile_but_not_outside_cards() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::KARN_THE_GREAT_CREATOR])
        .hand(0, &[index::SOL_RING, index::SOL_RING])
        .sideboard(0, &[index::SOL_RING])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e.state.format = baylee_core::preset::FormatId::Commander;
    let hidden = hand_to(&mut e, P0, index::SOL_RING, ZoneLocation::Exile(P0));
    e.state
        .object_mut(hidden)
        .unwrap()
        .status
        .insert(crate::object::Status::FACE_DOWN);
    e.state.invalidate_projections();
    let visible = hand_to(&mut e, P0, index::SOL_RING, ZoneLocation::Exile(P0));
    e.refresh_offer();
    activate(&mut e, P0, index::KARN_THE_GREAT_CREATOR, 2);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Wish,
                ..
            }
        )
    });
    assert_eq!(cards_offered(&e), vec![visible]);
    choose(&mut e, vec![visible]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(visible).unwrap().zone, Zone::Hand);
    assert_eq!(e.state.object(hidden).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.zones.list(ZoneLocation::OutsideGame(P0)).len(), 1);
}

#[test]
fn batch45_mastery_card_copy_does_not_keep_lattices_continuous_color_change() {
    let mut e = prepared(
        &[index::MIZZIX_S_MASTERY, index::COUNSEL_OF_THE_SORATAMI],
        &[index::MYCOSYNTH_LATTICE],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    let lattice = on_battlefield(&e, P0, index::MYCOSYNTH_LATTICE).unwrap();
    e.state.refresh_characteristics();
    assert!(
        e.state
            .object(spell)
            .unwrap()
            .characteristics()
            .colors
            .is_empty()
    );
    cast_with_floating(&mut e, P0, index::MIZZIX_S_MASTERY);
    mode(&mut e, CastModeKind::Mode(0));
    target(&mut e, vec![spell], vec![]);
    pass_until(&mut e, exile_choice);
    let copy = cards_offered(&e)[0];
    assert_eq!(
        e.state.object(copy).unwrap().base.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Blue])
    );
    // Remove the continuous effect while the copy still exists in exile:
    // only the original spell's copiable color may remain afterward.
    crate::sba::destroy(&mut e.state, lattice);
    e.sync_static_effects();
    e.state.refresh_characteristics();
    assert_eq!(
        e.state.object(copy).unwrap().characteristics().colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Blue])
    );
    choose(&mut e, vec![copy]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.per_turn.draws[0], 2);
}
