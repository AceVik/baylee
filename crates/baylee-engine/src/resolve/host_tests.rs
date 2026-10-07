use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::Filter;
use baylee_core::ids::SeatSet;

static UNTAP: &[Effect] = &[Effect::UntapAll {
    filter: &Filter::AttachedToBySource,
}];
static REGENERATE: &[Effect] = &[Effect::RegenerateAll {
    filter: &Filter::AttachedToBySource,
}];

/// An Aura on one tapped creature, another tapped creature beside it,
/// and `effects` resolving from the Aura.
fn resolved(effects: &'static [Effect]) -> (GameState, ObjectId, ObjectId) {
    let me = PlayerId::new(0);
    let mut state = GameState::from_preset(&preset(17, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let bare = |state: &mut GameState, label: &str| {
        let name = state.names.intern(label);
        state.create_bare(me, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    };
    let host = bare(&mut state, "Host");
    let other = bare(&mut state, "Other");
    let aura = bare(&mut state, "Aura");
    state.object_mut(aura).expect("here").attached_to = Some(host);
    state.set_tapped(host, true);
    state.set_tapped(other, true);
    let mut res = Resolution {
        source: aura,
        on_stack: aura,
        controller: me,
        effects: effects.to_vec(),
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
    };
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    (state, host, other)
}

fn tapped(state: &GameState, id: ObjectId) -> bool {
    state
        .object(id)
        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
}

#[test]
fn an_auras_ability_reaches_its_host_and_nothing_else() {
    let (state, host, other) = resolved(UNTAP);
    assert!(!tapped(&state, host), "the enchanted creature untaps");
    assert!(tapped(&state, other), "and only it");

    let (state, host, other) = resolved(REGENERATE);
    let shields = |id| state.object(id).map(|o| o.regeneration_shields);
    assert_eq!(shields(host), Some(1));
    assert_eq!(shields(other), Some(0));
}
