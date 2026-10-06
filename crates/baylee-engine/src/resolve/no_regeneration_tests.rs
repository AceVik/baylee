use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;

static NO_REGEN: &[Effect] = &[Effect::CantBeRegeneratedThisTurn {
    target: TargetSpec::AnyTarget,
}];

#[test]
fn the_target_keeps_its_shield_and_is_destroyed_through_it() {
    let me = PlayerId::new(0);
    let mut state = GameState::from_preset(&preset(19, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let bare = |state: &mut GameState, label: &str| {
        let name = state.names.intern(label);
        state.create_bare(me, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    };
    let troll = bare(&mut state, "Troll");
    let spell = bare(&mut state, "Disintegrate");
    state
        .object_mut(troll)
        .expect("seated")
        .regeneration_shields = 1;
    let mut res = Resolution {
        source: spell,
        on_stack: spell,
        controller: me,
        effects: NO_REGEN.to_vec(),
        pc: 0,
        targets: SmallVec::from_slice(&[troll]),
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
    let version = state.object(troll).expect("still there").version;
    assert_eq!(state.per_turn.cant_regenerate, vec![(troll, version)]);
    crate::sba::destroy(&mut state, troll);
    assert_ne!(
        state.object(troll).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the shield stands and is not applied"
    );
}
