use super::*;
use std::time::Duration;

fn placement() -> Placement {
    let game = Uuid::now_v7();
    Placement {
        match_id: game,
        game,
        format: Format::Standard,
        accounts: [[1; 16], [2; 16]],
        decks: [[3; 32], [4; 32]],
        generation: 1,
        primary: false,
    }
}

fn preset() -> GamePreset {
    let deck = baylee_cards::ranked::registration("standard", &["60 Forest".into()], &[], &[])
        .unwrap()
        .deck;
    super::super::deck::duel(Format::Standard, [&deck, &deck], 17)
}

#[test]
fn replay_requires_the_ranked_format_and_both_normal_seat_projections() {
    let mut preset = preset();
    assert!(super::super::deck::safe_setup(Format::Standard, &preset));
    assert!(!super::super::deck::safe_setup(Format::Commander, &preset));
    preset.seats[1].capabilities.see_hidden = true;
    assert!(!super::super::deck::safe_setup(Format::Standard, &preset));
}

#[test]
fn changing_generations_revokes_capabilities_without_restarting_the_mirror() {
    let old = placement();
    let fresh = Placement {
        primary: true,
        generation: 2,
        ..old
    };
    let preset = preset();
    let mut lobby = Lobby::default();
    let id = fresh.game.to_string();
    assert!(matches!(install(&mut lobby, old, &preset), Ok(true)));
    let game = lobby.games.get_mut(&id).unwrap();
    assert!(!game.allow_spectators);
    assert!(game.ranked_seat(0).is_none());
    game.agent_id = Some("replica-agent".to_owned());
    game.seats[0].socket_owner = Some(Uuid::now_v7());
    game.seats[0].socket_pending = Some(Uuid::now_v7());
    game.seats[0].seat_token_hash = Some(auth::token_hash("stale-seat"));
    game.seats[0].central_session = Some(vec![4; 32]);
    game.seats[0].account_id = Some("previous-seat-alias".to_owned());
    let mut previous_views = game.outbox(0).subscribe();
    assert!(matches!(install(&mut lobby, fresh, &preset), Ok(false)));
    assert!(install(&mut lobby, old, &preset).is_err());
    let game = lobby.games.get_mut(&id).unwrap();
    assert!(game.ranked_seat(0).is_none());
    assert!(matches!(
        previous_views.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Closed)
    ));
    let chair = &game.seats[0];
    assert!(
        chair.account_id.is_none()
            && chair.central_session.is_none()
            && chair.seat_token_hash.is_none()
    );
    assert!(chair.socket_owner.is_none() && chair.socket_pending.is_none());
    let role = game.ranked.as_mut().unwrap().replica.as_mut().unwrap();
    role.lease = Some(Instant::now() + Duration::from_secs(5));
    assert!(game.ranked_seat(0).is_some());
    game.ranked
        .as_mut()
        .unwrap()
        .replica
        .as_mut()
        .unwrap()
        .lease = Some(Instant::now());
    assert!(game.ranked_seat(0).is_none());
}
