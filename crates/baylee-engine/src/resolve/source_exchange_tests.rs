use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;

fn resolution(source: ObjectId, controller: PlayerId) -> Resolution {
    Resolution {
        source,
        on_stack: source,
        controller,
        effects: vec![Effect::ExchangeControlOfSourceAndTarget],
        pc: 0,
        targets: SmallVec::new(),
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
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    }
}

#[test]
fn source_and_target_exchange_control_indefinitely() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let lookup = SyntheticLookup::new(vec![]);
    let mut state = GameState::from_preset(&preset(70112, &[]), &lookup).unwrap();
    let source_name = state.names.intern("Exchange source");
    let source = state.create_bare(
        me,
        ObjectKind::Permanent,
        source_name,
        ZoneLocation::Battlefield,
    );
    let target_name = state.names.intern("Exchange target");
    let target = state.create_bare(
        them,
        ObjectKind::Permanent,
        target_name,
        ZoneLocation::Battlefield,
    );
    let mut res = resolution(source, me);
    res.targets = SmallVec::from_slice(&[target]);

    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(state.object(source).unwrap().controller, them);
    assert_eq!(state.object(target).unwrap().controller, me);
    assert_eq!(state.object(source).unwrap().owner, me);
    assert_eq!(state.object(target).unwrap().owner, them);

    state
        .effects
        .remove_where(|fx| fx.duration == baylee_cards_dsl::Duration::UntilEndOfTurn);
    state.refresh_characteristics();
    assert_eq!(state.object(source).unwrap().controller, them);
    assert_eq!(state.object(target).unwrap().controller, me);
}
