use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use crate::event::{Cause, GameEvent};
use crate::object::ObjectKind;
use crate::resolve::SubjectContext;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use smallvec::SmallVec;

fn resolution(source: ObjectId, target: ObjectId, version: u32) -> Resolution {
    Resolution {
        source,
        on_stack: source,
        controller: PlayerId::new(0),
        effects: vec![],
        pc: 0,
        targets: smallvec::smallvec![target],
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_lki: None,
        subject: SubjectContext::default().with_source_version(Some(version)),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        target_players: SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        this_way_from: None,
        coin_stake: None,
    }
}

fn permanent(state: &mut GameState) -> ObjectId {
    let name = state.names.intern("Synthetic observer subject");
    state.create_bare(
        PlayerId::new(0),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    )
}

#[test]
fn observer_registration_freezes_separate_subjects_and_ignores_prior_events() {
    let lookup = SyntheticLookup::new(vec![]);
    let mut state = GameState::from_preset(&preset(713, &[]), &lookup).unwrap();
    let source = permanent(&mut state);
    let target = permanent(&mut state);
    let source_ref = state.source_identity(source).unwrap();
    let target_ref = state.source_identity(target).unwrap();
    state.journal.record(GameEvent::ObjectUntapped {
        object: source,
        cause: Cause::Effect,
    });
    let res = resolution(source, target, source_ref.version);
    register(
        &mut state,
        &res,
        TargetSpec::ThisObject,
        DelayedObservation::LeavesBattlefieldOrUntaps,
        TargetSpec::Object(&baylee_cards_dsl::Filter::Any),
        &[Effect::SacrificeEvent],
    );
    assert_eq!(state.delayed.len(), 1);
    assert!(crate::trigger::collect(&state, &lookup, 0).is_empty());
    state.journal.record(GameEvent::ObjectUntapped {
        object: source,
        cause: Cause::Effect,
    });
    let triggers = crate::trigger::collect(&state, &lookup, 0);
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].source, source);
    assert_eq!(triggers[0].source_version, Some(source_ref.version));
    assert_eq!(triggers[0].event_object, Some(target));
    assert_eq!(
        triggers[0].event_object_identity,
        Some((target_ref.version, 0))
    );
    state
        .move_object(
            target,
            ZoneLocation::Hand(PlayerId::new(0)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let triggers = crate::trigger::collect(&state, &lookup, 0);
    assert_eq!(
        triggers[0].event_object_identity,
        Some((target_ref.version, 0)),
        "the delayed ability cannot adopt the target's later incarnation"
    );
}

#[test]
fn observer_registration_accepts_foreign_source_but_not_departed_source() {
    let lookup = SyntheticLookup::new(vec![]);
    let mut state = GameState::from_preset(&preset(714, &[]), &lookup).unwrap();
    let source = permanent(&mut state);
    let target = permanent(&mut state);
    let version = state.object(source).unwrap().version;
    let res = resolution(source, target, version);
    state
        .object_mut(source)
        .unwrap()
        .set_controller(PlayerId::new(1));
    register(
        &mut state,
        &res,
        TargetSpec::ThisObject,
        DelayedObservation::YouLoseControl,
        TargetSpec::Object(&baylee_cards_dsl::Filter::Any),
        &[Effect::SacrificeEvent],
    );
    assert_eq!(
        state.delayed.len(),
        1,
        "registration does not require current control"
    );
    state
        .move_object(
            source,
            ZoneLocation::Hand(PlayerId::new(0)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(
        crate::trigger::collect(&state, &lookup, 0).is_empty(),
        "another player's loss is not our loss"
    );
    register(
        &mut state,
        &res,
        TargetSpec::ThisObject,
        DelayedObservation::YouLoseControl,
        TargetSpec::Object(&baylee_cards_dsl::Filter::Any),
        &[Effect::SacrificeEvent],
    );
    assert_eq!(
        state.delayed.len(),
        1,
        "a gone source creates no new observer"
    );
}

#[test]
fn aura_reanimation_tapped_instruction_sets_status_before_entry() {
    for tapped in [false, true] {
        let lookup = SyntheticLookup::new(vec![]);
        let mut state = GameState::from_preset(&preset(715, &[]), &lookup).unwrap();
        let player = PlayerId::new(0);
        let source = permanent(&mut state);
        let version = state.object(source).unwrap().version;
        let host = state.zones.list(ZoneLocation::Library(player))[0];
        state
            .move_object(
                host,
                ZoneLocation::Graveyard(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state.object_mut(source).unwrap().attached_to = Some(host);
        let res = resolution(source, host, version);
        let finish = crate::aura_bindings::begin_reanimation(&mut state, &res, false, tapped)
            .expect("the enchanted card returns");
        assert_eq!(finish.returned.object, host);
        assert_eq!(
            state
                .object(host)
                .unwrap()
                .status
                .contains(crate::object::Status::TAPPED),
            tapped,
            "entry observes the requested tapped status, before ETB collection"
        );
    }
}

#[test]
fn original_source_watch_does_not_follow_the_sources_own_return() {
    let lookup = SyntheticLookup::new(vec![]);
    let mut state = GameState::from_preset(&preset(723, &[]), &lookup).unwrap();
    let source = permanent(&mut state);
    let version = state.object(source).unwrap().version;
    let mut res = resolution(source, source, version);
    let before = super::super::subjects::before(&state, &res);
    state
        .move_object(
            source,
            ZoneLocation::Graveyard(res.controller),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state.object_mut(source).unwrap().kind = ObjectKind::Permanent;
    state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    super::super::subjects::after(&state, &mut res, &before);
    assert!(
        super::super::subjects::source(&state, &res)
            .is_some_and(|reference| reference.version != version),
        "ordinary resolution subjects followed the actual public return"
    );
    register(
        &mut state,
        &res,
        TargetSpec::ThisObject,
        DelayedObservation::YouLoseControl,
        TargetSpec::Object(&baylee_cards_dsl::Filter::Any),
        &[Effect::SacrificeEvent],
    );
    assert!(
        state.delayed.is_empty(),
        "a source observer still refers to the original source"
    );
}
