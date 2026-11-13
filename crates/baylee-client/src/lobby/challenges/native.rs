//! Network effects run on the Central account's existing bounded worker.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::challenges::{Work, api};

pub(super) struct Finished {
    work: Work,
    responses: Vec<api::Response>,
    failed: bool,
    seat: Option<(
        api::Attempt,
        baylee_account::Admission,
        baylee_account::ChallengeSeat,
    )>,
}

fn session(state: &mut LobbyState) -> Option<central::Handle> {
    let handle = state.central_handle();
    let unchanged = match (&handle, &state.challenges.session) {
        (Some(a), Some(b)) => a.same_session(b),
        (None, None) => true,
        _ => false,
    };
    if !unchanged {
        // Old jobs retain the old mailbox, so they cannot populate this account's screen.
        state.challenges = Ui {
            session: handle.clone(),
            ..Ui::default()
        };
    }
    handle
}

pub(in crate::lobby) fn ask(state: &mut LobbyState, action: Action) {
    let Some(handle) = session(state) else {
        return;
    };
    let Some(work) = state
        .challenges
        .model
        .act(action, *uuid::Uuid::now_v7().as_bytes())
    else {
        return;
    };
    let retry = work.clone();
    let inbox = Arc::clone(&state.challenges.inbox);
    if !handle.submit(move |account| {
        let mut finished = Finished {
            work,
            responses: Vec::new(),
            failed: false,
            seat: None,
        };
        for request in &finished.work.requests {
            if let Ok(response) = account.challenges(request.clone()) {
                finished.responses.push(response);
            } else {
                finished.failed = true;
                break;
            }
        }
        if finished.work.play && !finished.failed {
            let attempt = finished.work.resume.as_ref().or_else(|| {
                finished.responses.iter().find_map(|response| {
                    if let api::Response::Attempt { attempt } = response {
                        Some(attempt)
                    } else {
                        None
                    }
                })
            });
            match attempt
                .filter(|attempt| attempt.state.active())
                .map(|attempt| account.play_challenge(attempt.id))
            {
                Some(Ok(seat)) => finished.seat = Some(seat),
                _ => finished.failed = true,
            }
        }
        if let Ok(mut slot) = inbox.lock() {
            *slot = Some(finished);
        }
    }) {
        state.challenges.model.receive(&retry, Vec::new(), true);
    }
}

pub(in crate::lobby) fn tick(
    time: Res<Time>,
    mut state: ResMut<LobbyState>,
    mut clock: Local<(f64, bool)>,
) {
    // Avoid marking the entire lobby changed on idle frames.
    let changed_session = match (state.central_handle(), &state.challenges.session) {
        (Some(a), Some(b)) => !a.same_session(b),
        (None, None) => false,
        _ => true,
    };
    if changed_session {
        session(&mut state);
    }
    let reply = state
        .challenges
        .inbox
        .lock()
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some(reply) = reply {
        state
            .challenges
            .model
            .receive(&reply.work, reply.responses, reply.failed);
        clock.0 = time.elapsed_secs_f64();
        if !state.challenges.model.busy
            && !state.challenges.model.failed
            && state.challenges.model.attempt().is_none()
        {
            state.challenges.pending_result = false;
        }
        if let Some((attempt, admission, seat)) = reply.seat {
            enter(&mut state, attempt, admission, seat);
            return;
        }
    }
    let showing = state.hub == Hub::Challenges && state.central_ready();
    let opened = showing && !clock.1;
    clock.1 = showing;
    if showing
        && !state.challenges.model.busy
        && state.challenges.model.confirmation.is_none()
        && (opened
            || (!state.challenges.model.loaded && !state.challenges.model.failed)
            || time.elapsed_secs_f64() - clock.0
                >= if state.challenges.model.active.is_empty() {
                    60.0
                } else {
                    10.0
                })
    {
        clock.0 = time.elapsed_secs_f64();
        ask(&mut state, Action::Refresh);
    }
}

fn enter(
    state: &mut LobbyState,
    attempt: api::Attempt,
    mut admission: baylee_account::Admission,
    mut seat: baylee_account::ChallengeSeat,
) {
    state.challenges.playing = Some((attempt.challenge, seat.game_id.clone()));
    let lang = state.lobby.lang();
    state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
    state.gateway = attempt.host;
    state.gateway_selected = true;
    state.art_cache = false;
    state.offline = None;
    state.local_decks = None;
    state.connected = false;
    state.room_away = false;
    state.lobby = Lobby::new();
    state.lobby.set_lang(lang);
    state.lobby.set_gateway_ready(true);
    let _ = state.lobby.apply(LobbyEvent::LoggedIn {
        token: std::mem::take(&mut admission.token),
        username: admission.username.take(),
    });
    state.lobby.set_central_bound(true);
    if let Some(renew_in) = admission.renew_in {
        let token = state.lobby.token().unwrap_or_default().to_string();
        let _ = state
            .lobby
            .apply(LobbyEvent::BearerRenewed { token, renew_in });
    }
    state
        .lobby
        .enter_challenge(client_core::lobby::SeatHandover {
            game_id: seat.game_id,
            seat: seat.seat,
            seat_token: std::mem::take(&mut *seat.seat_token),
            local: false,
        });
}

pub(in crate::lobby) fn returned(mut state: ResMut<LobbyState>, duel: Option<Res<crate::Duel>>) {
    if matches!(state.lobby.screen(), Screen::Replay(_)) {
        return;
    }
    let Some((task, _)) = state.challenges.playing.as_ref() else {
        return;
    };
    let task = *task;
    state.challenges.playing = None;
    state.challenges.pending_result = duel.as_deref().is_some_and(|duel| duel.ending().is_some());
    state.hub = Hub::Challenges;
    state.challenges.model.selected = Some(task);
    ask(&mut state, Action::Refresh);
}
