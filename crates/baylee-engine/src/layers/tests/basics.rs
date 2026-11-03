//! The layer system one modifier at a time, and two at a time where the
//! order is the point: timestamps within a layer (CR 613.7), regardless of
//! the order effects were registered in, and each layer
//! touching only its own characteristic.

use super::*;

fn projected(state: &GameState, id: ObjectId) -> crate::object::Characteristics {
    let obj = state.object(id).expect("in play");
    recompute(state, obj).characteristics
}

fn bear(state: &mut GameState) -> ObjectId {
    permanent(state, "Bear", TypeSet::CREATURE, Some((2, 2)))
}

#[test]
fn two_pumps_add_up() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(&mut state, 1, Modifier::ModifyPT(1, 1));
    register(&mut state, 2, Modifier::ModifyPT(2, -1));
    assert_eq!(body(&state, id), (Some(5), Some(2)));
}

#[test]
fn a_shrink_may_take_power_below_zero() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(&mut state, 1, Modifier::ModifyPT(-4, 0));
    assert_eq!(body(&state, id), (Some(-2), Some(2)));
}

#[test]
fn the_later_of_two_setting_effects_wins() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(&mut state, 3, Modifier::SetPT(5, 5));
    register(&mut state, 7, Modifier::SetPT(1, 4));
    assert_eq!(body(&state, id), (Some(1), Some(4)));
}

#[test]
fn the_later_setting_effect_wins_whatever_order_it_was_registered_in() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(&mut state, 7, Modifier::SetPT(1, 4));
    register(&mut state, 3, Modifier::SetPT(5, 5));
    assert_eq!(body(&state, id), (Some(1), Some(4)), "timestamp 7 is later");
}

#[test]
fn setting_power_alone_keeps_the_toughness() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(&mut state, 1, Modifier::SetPower(0));
    assert_eq!(body(&state, id), (Some(0), Some(2)));
}

#[test]
fn setting_toughness_alone_keeps_the_power() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(&mut state, 1, Modifier::SetToughness(7));
    assert_eq!(body(&state, id), (Some(2), Some(7)));
}

#[test]
fn two_switches_cancel_out() {
    let mut state = fresh();
    let id = permanent(&mut state, "Wall", TypeSet::CREATURE, Some((0, 6)));
    register(&mut state, 1, Modifier::SwitchPT);
    assert_eq!(body(&state, id), (Some(6), Some(0)));
    register(&mut state, 2, Modifier::SwitchPT);
    assert_eq!(body(&state, id), (Some(0), Some(6)));
}

#[test]
fn a_granted_keyword_is_added_to_the_printed_ones() {
    let mut state = fresh();
    let id = bear(&mut state);
    state.object_mut(id).unwrap().base_mut().keywords = KeywordSet::TRAMPLE;
    register(&mut state, 1, Modifier::AddKeyword(KeywordSet::FLYING));
    let kw = projected(&state, id).keywords;
    assert!(kw.contains(KeywordSet::FLYING));
    assert!(kw.contains(KeywordSet::TRAMPLE));
}

#[test]
fn a_later_removal_takes_away_a_granted_keyword() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(&mut state, 1, Modifier::AddKeyword(KeywordSet::FLYING));
    register(&mut state, 2, Modifier::RemoveKeyword(KeywordSet::FLYING));
    assert!(!projected(&state, id).keywords.contains(KeywordSet::FLYING));
}

#[test]
fn a_grant_after_a_removal_puts_the_keyword_back() {
    let mut state = fresh();
    let id = bear(&mut state);
    state.object_mut(id).unwrap().base_mut().keywords = KeywordSet::FLYING;
    register(&mut state, 1, Modifier::RemoveKeyword(KeywordSet::FLYING));
    assert!(!projected(&state, id).keywords.contains(KeywordSet::FLYING));
    register(&mut state, 2, Modifier::AddKeyword(KeywordSet::FLYING));
    assert!(projected(&state, id).keywords.contains(KeywordSet::FLYING));
}

#[test]
fn losing_all_keywords_leaves_none_of_them() {
    let mut state = fresh();
    let id = bear(&mut state);
    state.object_mut(id).unwrap().base_mut().keywords =
        KeywordSet::FLYING.union(KeywordSet::FIRST_STRIKE);
    register(&mut state, 1, Modifier::LoseKeywords);
    let kw = projected(&state, id).keywords;
    assert!(!kw.contains(KeywordSet::FLYING));
    assert!(!kw.contains(KeywordSet::FIRST_STRIKE));
}

#[test]
fn setting_a_color_replaces_the_printed_one() {
    let mut state = fresh();
    let id = bear(&mut state);
    state.object_mut(id).unwrap().base_mut().colors = ColorSet::of(Color::Green);
    register(&mut state, 1, Modifier::SetColor(ColorSet::of(Color::Blue)));
    let colors = projected(&state, id).colors;
    assert!(colors.contains(Color::Blue));
    assert!(!colors.contains(Color::Green));
}

#[test]
fn adding_a_color_keeps_the_printed_one() {
    let mut state = fresh();
    let id = bear(&mut state);
    state.object_mut(id).unwrap().base_mut().colors = ColorSet::of(Color::Green);
    register(&mut state, 1, Modifier::AddColor(ColorSet::of(Color::Red)));
    let colors = projected(&state, id).colors;
    assert!(colors.contains(Color::Red));
    assert!(colors.contains(Color::Green));
}

#[test]
fn the_later_of_two_color_settings_wins() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(
        &mut state,
        5,
        Modifier::SetColor(ColorSet::of(Color::White)),
    );
    register(
        &mut state,
        2,
        Modifier::SetColor(ColorSet::of(Color::Black)),
    );
    let colors = projected(&state, id).colors;
    assert!(colors.contains(Color::White));
    assert!(!colors.contains(Color::Black));
}

#[test]
fn an_added_type_keeps_the_others() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(&mut state, 1, Modifier::AddType(TypeSet::ARTIFACT));
    let types = projected(&state, id).types;
    assert!(types.contains(TypeSet::ARTIFACT));
    assert!(types.contains(TypeSet::CREATURE));
}

#[test]
fn a_removed_type_is_gone_and_the_rest_stay() {
    let mut state = fresh();
    let id = permanent(
        &mut state,
        "Dryad Arbor",
        TypeSet::CREATURE.union(TypeSet::LAND),
        Some((1, 1)),
    );
    register(&mut state, 1, Modifier::RemoveType(TypeSet::CREATURE));
    let types = projected(&state, id).types;
    assert!(!types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
}

#[test]
fn a_type_change_does_not_touch_the_body_of_a_creature_that_stays_one() {
    let mut state = fresh();
    let id = bear(&mut state);
    register(&mut state, 1, Modifier::AddType(TypeSet::ARTIFACT));
    assert_eq!(body(&state, id), (Some(2), Some(2)));
}

#[test]
fn an_effect_on_a_creature_reaches_no_noncreature() {
    let mut state = fresh();
    let rock = permanent(&mut state, "Rock", TypeSet::ARTIFACT, None);
    register(&mut state, 1, Modifier::ModifyPT(3, 3));
    assert_eq!(body(&state, rock), (None, None));
}

#[test]
fn with_no_effects_the_projection_is_the_printed_object() {
    let mut state = fresh();
    let id = bear(&mut state);
    let c = projected(&state, id);
    assert_eq!((c.power, c.toughness), (Some(2), Some(2)));
    assert_eq!(c.types, TypeSet::CREATURE);
    assert_eq!(c.keywords, KeywordSet::EMPTY);
}
