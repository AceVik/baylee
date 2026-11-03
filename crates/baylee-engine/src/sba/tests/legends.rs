//! The legend rule (CR 704.5j) on bare permanents: two legendary
//! permanents with the same name under one controller ask that player to
//! keep one; different names, different controllers or a nonlegendary
//! twin ask nothing; and the answer puts the others into the graveyard.

use super::{RegistryLookup, apply_legend_choice, bare, empty_boards_preset, on_battlefield, run};
use crate::state::GameState;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::{SupertypeSet, TypeSet};

fn board(seed: u64) -> GameState {
    GameState::from_preset(&empty_boards_preset(seed), &RegistryLookup).expect("game starts")
}

fn legend(state: &mut GameState, name: &str) -> ObjectId {
    let id = bare(state, name, TypeSet::CREATURE, &[]);
    state.object_mut(id).unwrap().base_mut().supertypes = SupertypeSet::LEGENDARY;
    state.invalidate_projections();
    id
}

#[test]
fn one_legend_alone_asks_nothing() {
    let mut state = board(401);
    let _ = legend(&mut state, "Lone Hero");
    let outcome = run(&mut state, &RegistryLookup);
    assert!(outcome.legend_choice.is_none());
}

#[test]
fn two_legends_with_one_name_under_one_player_ask_that_player() {
    let mut state = board(402);
    let a = legend(&mut state, "Twin Hero");
    let b = legend(&mut state, "Twin Hero");
    let outcome = run(&mut state, &RegistryLookup);
    let (player, group) = outcome.legend_choice.expect("a choice is due");
    assert_eq!(player, PlayerId::new(0));
    assert_eq!(group.len(), 2);
    assert!(group.contains(&a) && group.contains(&b));
}

#[test]
fn two_legends_with_different_names_coexist() {
    let mut state = board(403);
    let _ = legend(&mut state, "First Hero");
    let _ = legend(&mut state, "Second Hero");
    assert!(run(&mut state, &RegistryLookup).legend_choice.is_none());
}

#[test]
fn the_same_legend_under_two_players_is_no_conflict() {
    let mut state = board(404);
    let _ = legend(&mut state, "Shared Hero");
    let theirs = legend(&mut state, "Shared Hero");
    let timestamp = state.next_timestamp();
    let filter = crate::effects::EffectFilter::object(&state, theirs);
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: PlayerId::new(1),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Control,
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier: baylee_cards_dsl::Modifier::GainControl,
    });
    state.refresh_characteristics();
    assert_eq!(state.object(theirs).unwrap().controller, PlayerId::new(1));
    assert!(run(&mut state, &RegistryLookup).legend_choice.is_none());
}

#[test]
fn a_nonlegendary_twin_is_no_conflict() {
    let mut state = board(405);
    let _ = legend(&mut state, "Copied Hero");
    let _ = bare(&mut state, "Copied Hero", TypeSet::CREATURE, &[]);
    assert!(run(&mut state, &RegistryLookup).legend_choice.is_none());
}

#[test]
fn three_copies_are_one_group_of_three() {
    let mut state = board(406);
    for _ in 0..3 {
        let _ = legend(&mut state, "Triple Hero");
    }
    let (_, group) = run(&mut state, &RegistryLookup)
        .legend_choice
        .expect("a choice is due");
    assert_eq!(group.len(), 3);
}

#[test]
fn the_answer_keeps_the_chosen_one_and_bins_the_rest() {
    let mut state = board(407);
    let a = legend(&mut state, "Chosen Hero");
    let b = legend(&mut state, "Chosen Hero");
    let c = legend(&mut state, "Chosen Hero");
    let (player, group) = run(&mut state, &RegistryLookup)
        .legend_choice
        .expect("a choice is due");
    apply_legend_choice(&mut state, player, b, &group);
    let outcome = run(&mut state, &RegistryLookup);
    assert!(outcome.legend_choice.is_none(), "nothing left to choose");
    assert!(on_battlefield(&state, b));
    assert!(!on_battlefield(&state, a));
    assert!(!on_battlefield(&state, c));
}

/// The legend rule is not destruction: indestructible does not keep the
/// one not chosen.
#[test]
fn an_indestructible_legend_that_was_not_kept_still_goes() {
    let mut state = board(408);
    let a = legend(&mut state, "Sturdy Hero");
    let b = legend(&mut state, "Sturdy Hero");
    state.object_mut(a).unwrap().base_mut().keywords = baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE;
    state.invalidate_projections();
    let (player, group) = run(&mut state, &RegistryLookup)
        .legend_choice
        .expect("a choice is due");
    apply_legend_choice(&mut state, player, b, &group);
    let _ = run(&mut state, &RegistryLookup);
    assert!(on_battlefield(&state, b));
    assert!(!on_battlefield(&state, a));
}

#[test]
fn a_legendary_noncreature_follows_the_same_rule() {
    let mut state = board(409);
    let mk = |state: &mut GameState| {
        let id = bare(state, "Unique Relic", TypeSet::ARTIFACT, &[]);
        state.object_mut(id).unwrap().base_mut().supertypes = SupertypeSet::LEGENDARY;
        state.invalidate_projections();
        id
    };
    let _ = mk(&mut state);
    let _ = mk(&mut state);
    assert!(run(&mut state, &RegistryLookup).legend_choice.is_some());
}
