//! Regression: assertion failures and diagnostics must not reveal credentials.
use super::*;

const SECRET: &str = "private-credential-must-never-appear";

fn redacted(value: &impl std::fmt::Debug) {
    for shown in [format!("{value:?}"), format!("{value:#?}")] {
        assert!(!shown.contains(SECRET));
        assert!(!shown.is_empty());
    }
}

#[test]
fn requests_events_and_nested_state_redact_authentication_secrets() {
    for request in [
        LobbyRequest::Register {
            username: SECRET.into(),
            display_name: SECRET.into(),
            password: SECRET.into(),
            invite_key: Some(SECRET.into()),
        },
        LobbyRequest::LogIn {
            username: SECRET.into(),
            password: SECRET.into(),
        },
        LobbyRequest::ChangePassword {
            username: SECRET.into(),
            password: SECRET.into(),
            new_password: SECRET.into(),
        },
        LobbyRequest::PlayAsGuest {
            display_name: None,
            invite_key: Some(SECRET.into()),
        },
        LobbyRequest::DeleteAccount {
            password: Some(SECRET.into()),
        },
        LobbyRequest::LogOut {
            token: SECRET.into(),
        },
        LobbyRequest::Depart {
            token: SECRET.into(),
        },
        LobbyRequest::JoinGame {
            game_id: "game".into(),
            deck_id: "deck".into(),
            seat: None,
            password: SECRET.into(),
            invite: Some(SECRET.into()),
        },
    ] {
        redacted(&request);
    }
    let seat = SeatHandover {
        seat_token: SECRET.into(),
        ..Default::default()
    };
    redacted(&seat);
    redacted(&Screen::Seated(seat.clone()));
    redacted(&LobbyEvent::Seated(seat));
    redacted(&LobbyEvent::LoggedIn {
        token: SECRET.into(),
        username: None,
    });
    redacted(&Performer::Gateway(SECRET.into()));
    redacted(&Performer::Guest(SECRET.into()));
    let mut lobby = Lobby::new();
    for field in [Field::Password, Field::PasswordAgain, Field::RoomPassword] {
        lobby.set_field(field, SECRET);
    }
    lobby.password_change = Some(SECRET.into());
    lobby.performer = Performer::Gateway(SECRET.into());
    redacted(&lobby);
}
