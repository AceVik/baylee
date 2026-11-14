use super::*;
use baylee_engine::event::Cause;
use baylee_engine::object::{Rider, Status};
use baylee_engine::zone::{Zone, ZonePosition};
use baylee_view::{LogEvent, LogObject};

#[test]
fn hidden_exile_views_and_logs_require_explicit_permission_even_for_the_owner() {
    let (owner, other) = (PlayerId::new(0), PlayerId::new(1));
    for seen in [
        SeatSet::new(),
        [owner].into_iter().collect(),
        [other].into_iter().collect(),
    ] {
        let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
        let mut log = crate::log::GameLog::new(&state);
        let from = log.len();
        let card = *state
            .zones
            .list(ZoneLocation::Library(owner))
            .last()
            .unwrap();
        let name = state
            .names
            .get(state.object(card).unwrap().base.name)
            .to_string();
        state
            .move_object(
                card,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let blank = state.bare_base(baylee_core::ids::NameRef::new(0));
        let object = state.object_mut(card).unwrap();
        object.original_base = Some(std::sync::Arc::clone(&object.base));
        object.base = blank;
        object.cache.clear();
        object.status.insert(Status::FACE_DOWN);
        object.riders.push(Rider::ExileVisibility {
            seen,
            follow_source: false,
        });
        state.refresh_characteristics();
        log.consume(&state);
        for viewer in [owner, other] {
            let shown = player_view(&state, viewer, 0, None, &SeatContext::default(), &[]);
            let object = shown.exile.iter().flatten().find(|o| o.id == card).unwrap();
            assert_eq!(object.card.is_some(), seen.contains(viewer));
            assert_eq!(
                object.name,
                if seen.contains(viewer) {
                    &name
                } else {
                    "Face-down"
                }
            );
            assert!(object.rules.is_none());
            assert!(object.types.is_empty());
            let lines = log.told(viewer, from, log.len());
            assert!(lines.iter().all(|entry| match &entry.event {
                LogEvent::Moved {
                    object: LogObject::Known { .. },
                    ..
                } => seen.contains(viewer),
                LogEvent::Moved {
                    object: LogObject::FaceDown { .. },
                    ..
                } => !seen.contains(viewer),
                _ => false,
            }));
            assert!(!lines.is_empty());
        }
        // Controlling a player grants that player's inspection rights, even if
        // neither they nor their controller owns the exiled card (CR 724.4).
        let ctx = SeatContext {
            controlled_players: [other].into_iter().collect(),
            ..SeatContext::default()
        };
        let controlled = player_view(&state, owner, 0, None, &ctx, &[]);
        let object = controlled
            .exile
            .iter()
            .flatten()
            .find(|o| o.id == card)
            .unwrap();
        assert_eq!(object.card.is_some(), !seen.is_empty());
        assert_eq!(state.object(card).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn linked_exile_lists_only_face_up_exiled_cards_while_source_is_face_up() {
    let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
    let owner = PlayerId::new(0);
    let lib = state.zones.list(ZoneLocation::Library(owner)).clone();
    let (source, exiled) = (lib[0], lib[1]);
    state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state.object_mut(source).unwrap().kind = baylee_engine::object::ObjectKind::Permanent;
    state
        .move_object(
            exiled,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let ver = u16::try_from(state.object(source).unwrap().version).unwrap();
    state
        .object_mut(exiled)
        .unwrap()
        .riders
        .push(Rider::ExiledWith {
            host: source,
            version: ver,
        });
    state.refresh_characteristics();

    let shown = player_view(
        &state,
        PlayerId::new(1),
        0,
        None,
        &SeatContext::default(),
        &[],
    );
    assert_eq!(shown.object(source).unwrap().linked_exile, vec![exiled]);

    state
        .object_mut(exiled)
        .unwrap()
        .status
        .insert(Status::FACE_DOWN);
    let hidden_card = player_view(
        &state,
        PlayerId::new(1),
        0,
        None,
        &SeatContext::default(),
        &[],
    );
    assert!(
        hidden_card.object(source).unwrap().linked_exile.is_empty(),
        "CR 406.3: face-down exiled cards are omitted from linked_exile"
    );

    state
        .object_mut(exiled)
        .unwrap()
        .status
        .remove(Status::FACE_DOWN);
    state
        .object_mut(source)
        .unwrap()
        .status
        .insert(Status::FACE_DOWN);
    let hidden_source = player_view(
        &state,
        PlayerId::new(1),
        0,
        None,
        &SeatContext::default(),
        &[],
    );
    assert!(
        hidden_source
            .object(source)
            .unwrap()
            .linked_exile
            .is_empty(),
        "CR 708.2: face-down permanent hides its linked exile"
    );
}
