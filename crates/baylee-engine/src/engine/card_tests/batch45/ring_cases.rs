//! Draw replacement ordering and boundary cases for Ring of Ma'rûf.
#[allow(clippy::wildcard_imports)] // Shared batch fixtures and real-card helpers.
use super::*;
use crate::state::DrawReplacement;
use crate::turn::Step;
use baylee_cards_dsl::Modifier;

fn table(rings: usize, sanctuary: bool, outside: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut battlefield = vec![index::RING_OF_MA_RUF; rings];
    if sanctuary {
        battlefield.push(index::ISLAND_SANCTUARY);
    }
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &battlefield)
        .hand(
            0,
            &[index::OPEN_COMMUNICATIONS, index::COUNSEL_OF_THE_SORATAMI],
        )
        .sideboard(0, outside)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    pool(&mut e, ManaColor::Blue, 30);
    for _ in 0..rings {
        activate(&mut e, P0, index::RING_OF_MA_RUF, 0);
        pass_until(&mut e, stack_is_empty);
    }
    e
}

fn wishing(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::Wish,
            ..
        }
    )
}

fn skipping(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        }
    )
}

fn ring_effects(e: &Engine<RegistryLookup>) -> usize {
    e.state
        .effects
        .iter()
        .filter(|fx| fx.modifier == Modifier::NextDrawWish)
        .count()
}

#[test]
fn batch45_ring_replaces_an_empty_library_draw_without_a_loss() {
    let mut e = table(1, false, &[index::SOL_RING]);
    for card in library(&e, P0) {
        e.state
            .move_object(
                card,
                ZoneLocation::Graveyard(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    cast_with_floating(&mut e, P0, index::OPEN_COMMUNICATIONS);
    pass_until(&mut e, wishing);
    let wished = cards_offered(&e)[0];
    choose(&mut e, vec![wished]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(wished).unwrap().zone, Zone::Hand);
    assert!(!e.state.players[0].tried_empty_draw);
    assert!(!e.state.has_left(P0));
    assert_eq!(e.state.per_turn.draws[0], 0);
}

#[test]
fn batch45_two_rings_replace_successive_draws_with_independent_choices() {
    let mut e = table(2, false, &[index::SOL_RING, index::GRIZZLY_BEARS]);
    assert_eq!(ring_effects(&e), 2);
    cast_with_floating(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    pass_until(&mut e, wishing);
    let outside = cards_offered(&e);
    choose(&mut e, vec![outside[1]]);
    assert!(wishing(&e));
    assert_eq!(ring_effects(&e), 1);
    assert_eq!(cards_offered(&e), vec![outside[0]]);
    choose(&mut e, vec![outside[0]]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(ring_effects(&e), 0);
    assert_eq!(e.state.per_turn.draws[0], 0);
    for card in outside {
        assert_eq!(e.state.object(card).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn batch45_ring_still_consumes_the_draw_when_no_outside_card_exists() {
    let mut e = table(1, false, &[]);
    let before = library(&e, P0).len();
    cast_with_floating(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(ring_effects(&e), 0);
    assert_eq!(e.state.per_turn.draws[0], 1);
    assert_eq!(library(&e, P0).len(), before - 1);
}

#[test]
fn batch45_ring_cannot_bring_an_outside_card_into_commander() {
    let mut e = table(1, false, &[index::SOL_RING]);
    // The fixture's deck is deliberately unconstrained; this tests the
    // format's outside-game rule rather than lobby deck validation.
    e.state.format = baylee_core::preset::FormatId::Commander;
    let outside = e.state.zones.list(ZoneLocation::OutsideGame(P0)).clone();
    cast_with_floating(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.zones.list(ZoneLocation::OutsideGame(P0)), &outside);
    assert_eq!(e.state.per_turn.draws[0], 1);
    assert_eq!(ring_effects(&e), 0);
}

#[test]
fn batch45_ring_unused_replacement_expires_at_end_of_turn() {
    let mut e = table(1, false, &[index::SOL_RING]);
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| e.state.turn.number > turn);
    assert_eq!(ring_effects(&e), 0);
    pass_until(&mut e, |e| {
        e.state.turn.active == P0 && e.state.turn.phase == crate::turn::Phase::FirstMain
    });
    pool(&mut e, ManaColor::Blue, 3);
    let before = e.state.per_turn.draws[0];
    cast_with_floating(&mut e, P0, index::OPEN_COMMUNICATIONS);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.per_turn.draws[0], before + 1);
    assert_eq!(e.state.zones.list(ZoneLocation::OutsideGame(P0)).len(), 1);
}

#[test]
fn batch45_sanctuary_skip_preserves_ring_for_the_following_draw() {
    let mut e = table(1, true, &[index::SOL_RING]);
    // Place the draw instruction in its controller's draw step. Both real
    // permanents' replacement effects then compete for each individual draw.
    e.state.turn.step = Step::Draw;
    e.state.draw_cards(P0, 2);
    assert!(e.offer_queued_draw());
    assert!(skipping(&e));
    yes(&mut e, true);
    assert!(skipping(&e));
    assert_eq!(ring_effects(&e), 1);
    yes(&mut e, false);
    assert!(wishing(&e));
    let wished = cards_offered(&e)[0];
    choose(&mut e, vec![wished]);
    assert_eq!(ring_effects(&e), 0);
    assert_eq!(e.state.per_turn.draws[0], 0);
    assert_eq!(e.state.object(wished).unwrap().zone, Zone::Hand);
    assert!(e.state.draws_to_offer.is_empty());
}

#[test]
fn batch45_sanctuary_decline_with_no_wish_options_settles_the_draw() {
    let mut e = table(1, true, &[]);
    e.state.turn.step = Step::Draw;
    e.state.draw_cards(P0, 1);
    assert!(e.offer_queued_draw());
    assert!(skipping(&e));
    yes(&mut e, false);
    assert!(!wishing(&e));
    assert_eq!(ring_effects(&e), 0);
    assert_eq!(e.state.per_turn.draws[0], 0);
    assert!(e.state.draws_to_offer.is_empty());
}

#[test]
fn batch45_same_source_identity_does_not_confuse_skip_and_wish_replacements() {
    let mut e = table(1, true, &[index::SOL_RING]);
    let sanctuary = on_battlefield(&e, P0, index::ISLAND_SANCTUARY).unwrap();
    let mut effect = e
        .state
        .effects
        .iter()
        .find(|fx| fx.modifier == Modifier::NextDrawWish)
        .unwrap()
        .clone();
    e.state.effects.remove_where(|fx| fx.id == effect.id);
    // Model a Ring source that subsequently became a Sanctuary. The Ring
    // effect outlives its original characteristics and is identified by id.
    effect.source = Some(sanctuary);
    let wish = e.state.effects.register(effect);
    e.state.turn.step = Step::Draw;
    e.state.draw_cards(P0, 1);
    assert_eq!(
        e.state.next_draw_offer(),
        Some((P0, DrawReplacement::Skip(sanctuary)))
    );
    assert!(e.offer_queued_draw());
    assert!(skipping(&e));
    yes(&mut e, false);
    assert_eq!(e.state.draw_wish_effect(P0, None).unwrap().0, wish);
    assert!(wishing(&e));
    let wished = cards_offered(&e)[0];
    choose(&mut e, vec![wished]);
    assert_eq!(ring_effects(&e), 0);
    assert_eq!(e.state.object(wished).unwrap().zone, Zone::Hand);
}

#[test]
fn batch45_controlled_player_cannot_choose_an_outside_card() {
    let mut e = table(1, false, &[index::SOL_RING]);
    let spell = in_hand(&e, P0, index::COUNSEL_OF_THE_SORATAMI).unwrap();
    cast(&mut e, spell);
    // Word of Command's resolution-control segment uses this registration:
    // the chosen card remains P0's, while P1 makes its resolving decisions.
    e.remember_controlled_spell(spell, P1, P0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(ring_effects(&e), 0);
    assert_eq!(e.state.per_turn.draws[0], 1);
    assert_eq!(e.state.zones.list(ZoneLocation::OutsideGame(P0)).len(), 1);
}
