//! Network effects run on the Central account's existing bounded worker.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::challenges::{Work, api};

pub(super) struct Finished {
    work: Work,
    responses: Vec<api::Response>,
    failed: bool,
    refusal: Option<baylee_account::Error>,
    ordinary: bool,
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

/// Restore UI context only after the ordinary gateway reply passed its epoch check.
pub(in crate::lobby) fn seated(state: &mut LobbyState, event: &LobbyEvent) {
    match event {
        LobbyEvent::ChallengeSeated { seat, challenge } => {
            session(state);
            state.challenges.playing = Some((*challenge, seat.game_id.clone()));
        }
        LobbyEvent::Seated(_) => state.challenges.playing = None,
        _ => {}
    }
}

pub(in crate::lobby) fn ask(state: &mut LobbyState, action: Action) {
    if !ready(state) {
        return;
    }
    let Some(handle) = session(state) else {
        return;
    };
    let action = if action == Action::Refresh {
        state
            .challenges
            .returning
            .map_or(action, |(_, game)| Action::Restore(game))
    } else {
        action
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
            refusal: None,
            ordinary: false,
            seat: None,
        };
        for request in &finished.work.requests {
            match account.challenges(request.clone()) {
                Ok(response) => finished.responses.push(response),
                Err(baylee_account::Error::Rejected) if finished.work.restoring => {
                    finished.ordinary = true;
                    break;
                }
                Err(error) => {
                    finished.failed = true;
                    finished.refusal = Some(error);
                    break;
                }
            }
        }
        if finished.work.play && !finished.failed {
            let attempt = finished.work.resume.as_ref().or_else(|| {
                finished
                    .responses
                    .iter()
                    .find_map(|response| match response {
                        api::Response::Attempt { attempt } => Some(attempt),
                        api::Response::Tower { response } => match &**response {
                            baylee_client_core::tower::api::Response::Run { run } => {
                                run.attempt.as_ref()
                            }
                            _ => None,
                        },
                        _ => None,
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
        if let Some(error) = reply.refusal {
            state.challenges.model.tower.refused(error);
        }
        state
            .challenges
            .model
            .receive(&reply.work, reply.responses, reply.failed);
        clock.0 = time.elapsed_secs_f64();
        if reply.work.restoring && !reply.failed {
            let returning = state.challenges.returning.take();
            if reply.ordinary
                && let Some((task, _)) = returning
            {
                ask(&mut state, Action::Select(task));
            } else {
                ask(&mut state, Action::Refresh);
            }
            return;
        }
        if !pending_result(&state) {
            state.challenges.pending_result = false;
        }
        if let Some((attempt, admission, seat)) = reply.seat {
            enter(&mut state, attempt, admission, seat);
            return;
        }
    }
    let showing = matches!(state.hub, Hub::Play | Hub::Challenges) && ready(&state);
    let opened = showing && !clock.1;
    clock.1 = showing;
    if showing
        && !state.challenges.model.busy
        && state.challenges.model.confirmation.is_none()
        && !state.challenges.model.tower.abandoning
        && state.challenges.model.tower.upgrade.is_none()
        && (opened
            || (!state.challenges.model.loaded && !state.challenges.model.failed)
            || time.elapsed_secs_f64() - clock.0
                >= if state.challenges.model.active.is_empty()
                    && !state
                        .challenges
                        .model
                        .tower
                        .run
                        .as_ref()
                        .is_some_and(|run| run.progress.phase.active())
                {
                    60.0
                } else {
                    10.0
                })
    {
        clock.0 = time.elapsed_secs_f64();
        ask(&mut state, Action::Refresh);
    }
}

/// Acceptance may still be finishing the account migration after its sheet closes.
fn ready(state: &LobbyState) -> bool {
    state.central_ready()
        && !state.central_terms_pending()
        && state.terms_subject != crate::lobby::front::terms::Subject::Central
}

fn pending_result(state: &LobbyState) -> bool {
    let ui = &state.challenges;
    ui.model.busy
        || ui.model.failed
        || ui
            .model
            .active
            .iter()
            .any(|attempt| Some(attempt.challenge) == ui.pending_task)
        || ui.model.tower.run.as_ref().is_some_and(|run| {
            run.progress.phase == baylee_client_core::tower::api::Phase::Playing
                && Some(run.encounter.id) == ui.pending_task
        })
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
    let Some((task, game)) = state.challenges.playing.as_ref() else {
        return;
    };
    let task = *task;
    let game = uuid::Uuid::parse_str(game)
        .ok()
        .map(|game| *game.as_bytes());
    state.challenges.playing = None;
    state.challenges.pending_result = duel.as_deref().is_some_and(|duel| duel.ending().is_some());
    state.challenges.pending_task = Some(task);
    state.challenges.returning = game.map(|game| (task, game));
    state.hub = Hub::Challenges;
    if state
        .challenges
        .model
        .tower
        .run
        .as_ref()
        .is_some_and(|run| run.encounter.id == task)
    {
        state.challenges.model.tower.opened = true;
        state.challenges.model.selected = None;
    } else {
        state.challenges.model.selected = Some(task);
    }
    ask(&mut state, Action::Refresh);
}
