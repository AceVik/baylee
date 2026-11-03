//! The plain filters, each on an object it matches and one it does not:
//! combinators, types, colours, names, control, tapped status, counters,
//! keywords, power, toughness and mana value. A filter that answered
//! "yes" to everything would pass every half that only checks a match.

use super::*;
use crate::object::CounterKind;
use baylee_core::color::{Color, ColorSet};

fn is(state: &GameState, filter: &Filter, id: ObjectId, you: PlayerId, this: ObjectId) -> bool {
    matches(filter, state, state.object(id).expect("exists"), you, this)
}

/// A creature of `power`/`toughness` controlled by `controller`.
fn body(state: &mut GameState, controller: PlayerId, power: i16, toughness: i16) -> ObjectId {
    let id = creature(state, controller, KeywordSet::EMPTY);
    let b = state.object_mut(id).unwrap().base_mut();
    b.power = Some(power);
    b.toughness = Some(toughness);
    id
}

#[test]
fn any_matches_everything_and_not_any_matches_nothing() {
    static NOTHING: Filter = Filter::Not(&Filter::Any);
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::EMPTY);
    assert!(is(&state, &Filter::Any, id, P0, id));
    assert!(!is(&state, &NOTHING, id, P0, id));
}

#[test]
fn this_is_the_source_and_another_is_everything_else() {
    let mut state = empty_state();
    let source = creature(&mut state, P0, KeywordSet::EMPTY);
    let other = creature(&mut state, P0, KeywordSet::EMPTY);
    assert!(is(&state, &Filter::This, source, P0, source));
    assert!(!is(&state, &Filter::This, other, P0, source));
    assert!(is(&state, &Filter::Another, other, P0, source));
    assert!(!is(&state, &Filter::Another, source, P0, source));
}

#[test]
fn and_needs_every_part_and_or_needs_one() {
    static FLYING_CREATURE: Filter = Filter::And(&[
        Filter::HasType(TypeSet::CREATURE),
        Filter::HasKeyword(KeywordSet::FLYING),
    ]);
    static FLYING_OR_REACH: Filter = Filter::Or(&[
        Filter::HasKeyword(KeywordSet::FLYING),
        Filter::HasKeyword(KeywordSet::REACH),
    ]);
    let mut state = empty_state();
    let bird = creature(&mut state, P0, KeywordSet::FLYING);
    let spider = creature(&mut state, P0, KeywordSet::REACH);
    let bear = creature(&mut state, P0, KeywordSet::EMPTY);
    assert!(is(&state, &FLYING_CREATURE, bird, P0, bird));
    assert!(!is(&state, &FLYING_CREATURE, spider, P0, bird));
    assert!(is(&state, &FLYING_OR_REACH, bird, P0, bird));
    assert!(is(&state, &FLYING_OR_REACH, spider, P0, bird));
    assert!(!is(&state, &FLYING_OR_REACH, bear, P0, bird));
}

#[test]
fn an_empty_and_is_true_and_an_empty_or_is_false() {
    static ALL_OF_NOTHING: Filter = Filter::And(&[]);
    static ONE_OF_NOTHING: Filter = Filter::Or(&[]);
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::EMPTY);
    assert!(is(&state, &ALL_OF_NOTHING, id, P0, id));
    assert!(!is(&state, &ONE_OF_NOTHING, id, P0, id));
}

#[test]
fn has_type_and_lacks_type_are_opposites() {
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::EMPTY);
    assert!(is(&state, &Filter::HasType(TypeSet::CREATURE), id, P0, id));
    assert!(!is(
        &state,
        &Filter::LacksType(TypeSet::CREATURE),
        id,
        P0,
        id
    ));
    assert!(!is(&state, &Filter::HasType(TypeSet::ARTIFACT), id, P0, id));
    assert!(is(
        &state,
        &Filter::LacksType(TypeSet::ARTIFACT),
        id,
        P0,
        id
    ));
}

#[test]
fn colour_filters_read_the_colours() {
    let mut state = empty_state();
    let green = creature(&mut state, P0, KeywordSet::EMPTY);
    let gold = creature(&mut state, P0, KeywordSet::EMPTY);
    let grey = creature(&mut state, P0, KeywordSet::EMPTY);
    state.object_mut(green).unwrap().base_mut().colors = ColorSet::of(Color::Green);
    state.object_mut(gold).unwrap().base_mut().colors =
        ColorSet::of(Color::Green).union(ColorSet::of(Color::White));
    let green_filter = Filter::HasColor(ColorSet::of(Color::Green));
    assert!(is(&state, &green_filter, green, P0, green));
    assert!(is(&state, &green_filter, gold, P0, green));
    assert!(!is(&state, &green_filter, grey, P0, green));
    assert!(is(&state, &Filter::Monocolored, green, P0, green));
    assert!(!is(&state, &Filter::Monocolored, gold, P0, green));
    assert!(
        !is(&state, &Filter::Monocolored, grey, P0, green),
        "colorless is no color"
    );
    assert!(is(&state, &Filter::IsColorless, grey, P0, green));
    assert!(!is(&state, &Filter::IsColorless, green, P0, green));
}

#[test]
fn a_name_filter_matches_that_name_only() {
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::EMPTY);
    assert!(is(&state, &Filter::Named("Test Creature"), id, P0, id));
    assert!(!is(&state, &Filter::Named("Other Creature"), id, P0, id));
}

#[test]
fn control_filters_are_read_from_the_asking_player() {
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::EMPTY);
    let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
    assert!(is(&state, &Filter::ControlledByYou, mine, P0, mine));
    assert!(!is(&state, &Filter::ControlledByYou, theirs, P0, mine));
    assert!(is(&state, &Filter::ControlledByOpponent, theirs, P0, mine));
    assert!(!is(&state, &Filter::ControlledByOpponent, mine, P0, mine));
    // The same objects asked from the other seat answer the other way.
    assert!(is(&state, &Filter::ControlledByYou, theirs, P1, theirs));
    assert!(is(&state, &Filter::ControlledByOpponent, mine, P1, theirs));
}

#[test]
fn tapped_and_untapped_follow_the_status() {
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::EMPTY);
    assert!(is(&state, &Filter::Untapped, id, P0, id));
    assert!(!is(&state, &Filter::Tapped, id, P0, id));
    state.set_tapped(id, true);
    assert!(is(&state, &Filter::Tapped, id, P0, id));
    assert!(!is(&state, &Filter::Untapped, id, P0, id));
}

#[test]
fn counter_filters_count_the_named_kind() {
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::EMPTY);
    assert!(!is(&state, &Filter::HasAnyCounter, id, P0, id));
    assert!(!is(
        &state,
        &Filter::HasCounter(CounterKind::P1P1),
        id,
        P0,
        id
    ));
    state
        .object_mut(id)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 2);
    assert!(is(&state, &Filter::HasAnyCounter, id, P0, id));
    assert!(is(
        &state,
        &Filter::HasCounter(CounterKind::P1P1),
        id,
        P0,
        id
    ));
    assert!(!is(
        &state,
        &Filter::HasCounter(CounterKind::M1M1),
        id,
        P0,
        id
    ));
    assert!(is(
        &state,
        &Filter::CountersAtLeast(CounterKind::P1P1, 2),
        id,
        P0,
        id
    ));
    assert!(!is(
        &state,
        &Filter::CountersAtLeast(CounterKind::P1P1, 3),
        id,
        P0,
        id
    ));
}

#[test]
fn a_keyword_filter_reads_that_keyword() {
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::FLYING);
    assert!(is(
        &state,
        &Filter::HasKeyword(KeywordSet::FLYING),
        id,
        P0,
        id
    ));
    assert!(!is(
        &state,
        &Filter::HasKeyword(KeywordSet::TRAMPLE),
        id,
        P0,
        id
    ));
}

#[test]
fn power_and_toughness_bounds_include_their_edge() {
    let mut state = empty_state();
    let id = body(&mut state, P0, 3, 4);
    assert!(is(&state, &Filter::PowerAtLeast(3), id, P0, id));
    assert!(!is(&state, &Filter::PowerAtLeast(4), id, P0, id));
    assert!(is(&state, &Filter::PowerAtMost(3), id, P0, id));
    assert!(!is(&state, &Filter::PowerAtMost(2), id, P0, id));
    assert!(is(&state, &Filter::ToughnessAtLeast(4), id, P0, id));
    assert!(!is(&state, &Filter::ToughnessAtLeast(5), id, P0, id));
    assert!(is(&state, &Filter::ToughnessAtMost(4), id, P0, id));
    assert!(!is(&state, &Filter::ToughnessAtMost(3), id, P0, id));
}

#[test]
fn mana_value_bounds_include_their_edge() {
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::EMPTY);
    state.object_mut(id).unwrap().base_mut().mana_cost =
        baylee_core::mana::ManaCost::parse("{2}{G}");
    assert!(is(&state, &Filter::CmcAtMost(3), id, P0, id));
    assert!(!is(&state, &Filter::CmcAtMost(2), id, P0, id));
    assert!(is(&state, &Filter::CmcAtLeast(3), id, P0, id));
    assert!(!is(&state, &Filter::CmcAtLeast(4), id, P0, id));
}

#[test]
fn a_creature_with_no_mana_cost_has_mana_value_zero() {
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::EMPTY);
    assert!(is(&state, &Filter::CmcAtMost(0), id, P0, id));
    assert!(!is(&state, &Filter::CmcAtLeast(1), id, P0, id));
}
