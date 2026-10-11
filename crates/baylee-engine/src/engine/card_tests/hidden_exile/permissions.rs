#[allow(clippy::wildcard_imports)]
use super::*;

fn steal(e: &mut Engine<RegistryLookup>, source: ObjectId, controller: PlayerId) {
    let filter = crate::effects::EffectFilter::object(&e.state, source);
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Control,
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter,
        modifier: baylee_cards_dsl::Modifier::GainControl,
    });
    e.state.refresh_characteristics();
    e.refresh_offer();
}

#[test]
fn hidden_exile_look_permission_persists_after_control_changes_and_source_departure() {
    let (mut e, source, card) = hidden(index::SHELLDOCK_ISLE, index::GRIZZLY_BEARS);
    let before = e.snapshot_hash();
    steal(&mut e, source, P1);
    assert!(e.state.object(card).unwrap().may_look_at_exile(P1));
    assert_ne!(e.snapshot_hash(), before);
    e.state
        .move_object(
            source,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    for player in [P0, P1] {
        assert!(e.state.object(card).unwrap().may_look_at_exile(player));
    }
    e.state
        .move_object(
            card,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            card,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(!e.state.object(card).unwrap().may_look_at_exile(P1));
}

#[test]
fn hidden_exile_phased_out_host_grants_no_new_inspection_until_it_returns() {
    let (mut e, source, card) = hidden(index::SHELLDOCK_ISLE, index::GRIZZLY_BEARS);
    e.state.phase_out(&[source]);
    e.state.object_mut(source).unwrap().set_controller(P1);
    e.state.restart_summoning_sickness(source);
    assert!(!e.state.object(card).unwrap().may_look_at_exile(P1));
    e.state.phase_in(source);
    assert!(e.state.object(card).unwrap().may_look_at_exile(P1));
    assert!(e.state.object(card).unwrap().may_look_at_exile(P0));
}

#[test]
fn hidden_exile_an_activated_link_survives_the_source_leaving_but_a_new_incarnation_cannot_use_it()
{
    for activated_first in [false, true] {
        let (mut e, source, card) = hidden(index::SHELLDOCK_ISLE, index::GRIZZLY_BEARS);
        satisfy(&mut e, index::SHELLDOCK_ISLE);
        if activated_first {
            activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
        }
        e.state
            .move_object(
                source,
                ZoneLocation::Hand(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        if !activated_first {
            e.state
                .move_object(
                    source,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            e.state.set_tapped(source, false);
            e.refresh_offer();
            // Let the new hideaway finish before activating this incarnation.
            let player = e.pending().asked().unwrap();
            e.apply(player, PlayerAction::PassPriority).unwrap();
            finish_hideaway(&mut e);
            e.state.set_tapped(source, false);
            e.refresh_offer();
            activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
        }
        pass_until(&mut e, playing);
        let Pending::ChooseCards { options, .. } = e.pending() else {
            unreachable!()
        };
        assert_eq!(options.contains(&card), activated_first);
        choose(&mut e, vec![]);
        pass_until(&mut e, stack_is_empty);
    }
}

#[test]
fn hidden_exile_land_play_obeys_turn_and_drop_limit_but_not_main_phase_timing() {
    for (own_turn, remaining) in [(true, true), (true, false), (false, true)] {
        let (mut e, _, card) = hidden(index::SHELLDOCK_ISLE, index::ISLAND);
        satisfy(&mut e, index::SHELLDOCK_ISLE);
        e.state.players[0].lands_played_this_turn = u8::from(!remaining);
        e.state.turn.phase = Phase::Combat;
        if !own_turn {
            e.state.turn.active = P1;
        }
        e.refresh_offer();
        activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
        pass_until(&mut e, playing);
        choose(&mut e, vec![card]);
        assert_eq!(
            e.state.object(card).unwrap().zone == Zone::Battlefield,
            own_turn && remaining
        );
        assert_eq!(
            e.state
                .object(card)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN),
            !(own_turn && remaining)
        );
        assert!(crate::casting::play_permission(&e.state, P0, card).is_none());
        assert!(
            !e.state
                .object(card)
                .unwrap()
                .riders
                .contains(&Rider::PlayableFromExileFor(P0))
        );
    }
}

#[test]
fn hidden_exile_optional_play_can_be_declined_and_x_is_zero_when_cast_free() {
    for play in [false, true] {
        let (mut e, _, card) = hidden(index::SHELLDOCK_ISLE, index::WALKING_BALLISTA);
        satisfy(&mut e, index::SHELLDOCK_ISLE);
        activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
        pass_until(&mut e, playing);
        choose(&mut e, if play { vec![card] } else { vec![] });
        if play {
            assert_eq!(e.state.object(card).unwrap().zone, Zone::Stack);
            assert_eq!(e.state.object(card).unwrap().x_value, 0);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.object(card).unwrap().zone,
            if play { Zone::Graveyard } else { Zone::Exile }
        );
    }
}
