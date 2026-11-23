use super::*;

#[test]
fn only_the_original_worker_and_successful_admission_can_bind_a_frozen_look() {
    let mut state = LobbyState::default();
    central::restored_for_test(&mut state, Vec::new());
    let game = Uuid::now_v7();
    let handover = SeatHandover {
        game_id: game.to_string(),
        seat: 1,
        seat_token: "b".repeat(64),
        local: false,
    };
    let look = Equipment {
        sleeve: Some(*Uuid::now_v7().as_bytes()),
        playmat: None,
    };
    let selected = Selection {
        account: state.central_handle().unwrap(),
        look: Some(look),
        dest: Destination::Join(game.to_string(), Some(1)),
    };
    let granted = Reply::Event(LobbyEvent::Seated(handover.clone()));
    assert!(
        selected
            .clone()
            .admit(&mut state, &Reply::Event(LobbyEvent::GatewayLost))
    );
    assert!(state.casual_worn.is_none());
    assert!(selected.clone().admit(&mut state, &granted));
    let bound = state.casual_worn.as_ref().unwrap();
    assert_eq!((bound.look, bound.game, bound.chair), (Some(look), game, 1));
    // The room brought back by SetSeat is not evidence that a specific player owns it.
    let _ = state.lobby.apply(LobbyEvent::Seated(handover));
    let mut change = selected.clone();
    change.look = Some(Equipment::default());
    change.dest = Destination::Change(game.to_string(), 0);
    assert!(
        change
            .clone()
            .admit(&mut state, &Reply::Event(LobbyEvent::Moved))
    );
    assert!(state.casual_worn.is_some());
    change.dest = Destination::Change(game.to_string(), 1);
    assert!(change.admit(&mut state, &Reply::Event(LobbyEvent::Moved)));
    assert_eq!(
        state.casual_worn.as_ref().unwrap().look,
        Some(Equipment::default())
    );
    state.casual_worn = None;
    state.central.as_mut().unwrap().handle = Some(central::Handle::unavailable_for_test());
    assert!(!selected.admit(&mut state, &granted));
    assert!(state.casual_worn.is_none());
}

#[test]
fn restarted_authenticated_seat_asks_for_its_confirmed_pair_without_reading_a_deck() {
    let mut state = LobbyState::default();
    let game = Uuid::now_v7();
    let take = LobbyRequest::TakeSeat {
        game_id: game.to_string(),
    };
    assert!(Selection::recover(&state, &take).is_none());
    central::restored_for_test(&mut state, Vec::new());
    let _ = state.lobby.apply(LobbyEvent::LoggedIn {
        token: "1".repeat(64),
        username: None,
    });
    let recovery = Selection::recover(&state, &take).unwrap();
    let mut seat = SeatHandover {
        game_id: Uuid::now_v7().to_string(),
        seat: 2,
        seat_token: "b".repeat(64),
        local: false,
    };
    assert!(
        recovery
            .clone()
            .admit(&mut state, &Reply::Event(LobbyEvent::Seated(seat.clone())))
    );
    assert!(state.casual_worn.is_none());
    seat.game_id = game.to_string();
    assert!(recovery.admit(&mut state, &Reply::Event(LobbyEvent::Seated(seat))));
    let binding = state.casual_worn.as_ref().unwrap();
    assert!(
        matches!(binding.request(false),table::Request::Resume{game:id,seat:2,..} if id==*game.as_bytes())
    );
    assert!(matches!(
        binding.request(true),
        table::Request::Resume { .. }
    ));
}

#[test]
fn changing_a_house_chair_or_another_room_never_changes_my_look() {
    let game = Uuid::now_v7().to_string();
    let mut seat = SeatHandover {
        game_id: game.clone(),
        seat: 0,
        seat_token: "a".repeat(64),
        local: false,
    };
    assert!(Destination::Change(game.clone(), 0).accepts(&seat));
    assert!(!Destination::Change(game.clone(), 1).accepts(&seat));
    assert!(!Destination::Join(Uuid::now_v7().to_string(), None).accepts(&seat));
    assert!(ordinary(&seat).is_some());
    seat.seat = SeatHandover::WATCH;
    assert!(ordinary(&seat).is_none());
    seat.seat = 0;
    seat.local = true;
    assert!(ordinary(&seat).is_none());
}
