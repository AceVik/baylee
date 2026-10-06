use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;

#[test]
fn a_toggled_target_becomes_what_it_was_not() {
    let me = PlayerId::new(0);
    let mut state = GameState::from_preset(&preset(17, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let bare = |state: &mut GameState, label: &str| {
        let name = state.names.intern(label);
        state.create_bare(me, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    };
    let (up, down, spell) = (
        bare(&mut state, "Up"),
        bare(&mut state, "Down"),
        bare(&mut state, "Twiddle"),
    );
    state.set_tapped(down, true);
    let tapped = |state: &GameState, id| {
        state
            .object(id)
            .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
    };
    for (target, was) in [(up, false), (down, true)] {
        let mut res = Resolution {
            source: spell,
            on_stack: spell,
            controller: me,
            effects: vec![Effect::ToggleTapTarget],
            pc: 0,
            targets: SmallVec::from_slice(&[target]),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            subject: crate::resolve::SubjectContext::default(),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
        };
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert_eq!(tapped(&state, target), !was);
    }
}
