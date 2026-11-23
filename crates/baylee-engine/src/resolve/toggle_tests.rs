use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::Filter;
use baylee_core::ids::SeatSet;
use baylee_core::types::TypeSet;

fn resolution(source: ObjectId, controller: PlayerId, effects: Vec<Effect>) -> Resolution {
    Resolution {
        source,
        on_stack: source,
        controller,
        effects,
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
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    }
}

fn tapped(state: &GameState, id: ObjectId) -> bool {
    state
        .object(id)
        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
}

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
    for (target, was) in [(up, false), (down, true)] {
        let mut res = resolution(spell, me, vec![Effect::ToggleTapTarget]);
        res.targets = SmallVec::from_slice(&[target]);
        res.targeted = true;
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert_eq!(tapped(&state, target), !was);
    }
}

#[test]
fn a_toggle_all_uses_one_snapshot_for_both_halves() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut state = GameState::from_preset(&preset(17, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let permanent = |state: &mut GameState, controller: PlayerId, label: &str| {
        let name = state.names.intern(label);
        state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        )
    };
    let my_up = permanent(&mut state, me, "My up");
    let my_down = permanent(&mut state, me, "My down");
    let their_up = permanent(&mut state, them, "Their up");
    let their_down = permanent(&mut state, them, "Their down");
    let noncreature = permanent(&mut state, me, "Noncreature");
    let spell = permanent(&mut state, me, "Spell");
    for id in [my_up, my_down, their_up, their_down] {
        state.object_mut(id).unwrap().base_mut().types = TypeSet::CREATURE;
    }
    state.set_tapped(my_down, true);
    state.set_tapped(their_down, true);
    state.set_tapped(noncreature, true);

    let mut res = resolution(
        spell,
        me,
        vec![Effect::ToggleTapAll {
            filter: &Filter::CREATURE,
        }],
    );
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));

    assert!(tapped(&state, my_up));
    assert!(!tapped(&state, my_down));
    assert!(tapped(&state, their_up));
    assert!(!tapped(&state, their_down));
    assert!(tapped(&state, noncreature));
}
