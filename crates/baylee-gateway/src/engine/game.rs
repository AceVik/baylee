//! Existing ordinary engine attachment and routing.
use super::{
    Envelope, Incoming, LobbyState, Shared, Uuid, WebSocket, auth, end_game, execution, hello_of,
    mpsc, next_envelope, outbox_of, refusal, run_job_socket, send, send_queued,
    spectator_outbox_of, stop_engine, tower, v1,
};

#[allow(clippy::too_many_lines)] // the attach, in the order it happens
pub(super) async fn run_engine_socket(
    state: Shared,
    mut socket: WebSocket,
    place: Option<auth::Held>,
) {
    let hello = hello_of(&mut socket).await;
    drop(place);
    let Some(v1::envelope::Msg::EngineHello(hello)) = hello else {
        return;
    };
    if let Some(authority) = state.tower.clone()
        && let Some(game) = authority.game(&hello.game_id)
    {
        tower::run(&state, socket, &hello, authority, game).await;
        return;
    }
    if hello.game_id.starts_with(crate::replays::JOB_PREFIX) {
        run_job_socket(&state, socket, &hello).await;
        return;
    }
    let game_id = hello.game_id.clone();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let setup = {
        let names = crate::seat_names(&state, &game_id).await;
        let guests = crate::seat_guests(&state, &game_id).await;
        let mut lobby = state.lobby.lock();
        // The id is a stranger's until it names a game, so it is not logged.
        let Some(game) = lobby.games.get_mut(&game_id) else {
            tracing::warn!("engine attached to a game that is not here");
            return;
        };
        // The token is the whole authorization on this socket: it was issued
        // for this game and handed to one agent. A second engine for the same
        // game is refused rather than allowed to take over — the game's state
        // lives in the process that already has it.
        let authorized = game
            .engine_token_hash
            .as_ref()
            .is_some_and(|h| auth::ct_eq(h, &auth::token_hash(&hello.token)));
        if !authorized || game.state != LobbyState::Playing || game.engine.is_some() {
            tracing::warn!(game_id, "engine attach refused");
            return;
        }
        // The engine this game was given cannot play it, and the agent
        // would start the same binary again, so the game ends here with the
        // reason said and logged, rather than waiting on an engine that
        // never comes (#271). After the token, so only the engine this game
        // ordered can end it.
        if let Some(why) = baylee_protocol::version_refusal("This engine", hello.protocol_version) {
            tracing::error!(game_id, why, "engine refused");
            Err(why)
        } else if !execution::matches(game.execution.as_ref(), &hello) {
            Err("engine does not match the reserved execution identity".to_owned())
        } else if game
            .challenge_executor
            .as_ref()
            .is_some_and(|required| hello.build != *required)
        {
            Err("challenge engine build does not match its certificate".to_owned())
        } else {
            let Some(preset) = game.preset.as_ref() else {
                return;
            };
            let Ok(preset_json) = serde_json::to_vec(preset) else {
                return;
            };
            game.engine = Some(tx);
            game.engine_pid = Some(hello.pid).filter(|pid| *pid != 0 && game.engine_local);
            let seats = game
                .seats
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let id = |a: &str| Uuid::parse_str(a).ok();
                    let spec = preset.seats.get(i);
                    (
                        baylee_db::records::Seat {
                            account: s.account_id.as_deref().and_then(id),
                            delegated_by: s.delegate.as_ref().and_then(|d| id(&d.by)),
                        },
                        baylee_db::records::SeatFacts {
                            seated: if s.delegate.is_some() {
                                baylee_db::records::Seated::Model
                            } else if s.kind == crate::lobby::SeatKind::Ai {
                                baylee_db::records::Seated::House
                            } else {
                                baylee_db::records::Seated::Person
                            },
                            colours: spec.map(crate::replays::deck_colours),
                            commander: spec
                                .and_then(|spec| spec.commanders.first())
                                .map(|entry| entry.card.get()),
                        },
                    )
                })
                .collect::<Vec<_>>();
            // An engine that says its build is of this release or later,
            // and writes `seen` into the record (`docs/protocol.md`
            // §"Replays").
            let opening = baylee_db::records::Opening {
                build: Some(hello.build.clone()).filter(|b| !b.is_empty()),
                checked: !hello.build.is_empty(),
                allow_spectators: Some(game.allow_spectators),
            };
            Ok((
                (opening, seats),
                Envelope {
                    msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                        game_id: game_id.clone(),
                        preset_json,
                        seat_names: names,
                        seat_guests: guests,
                        seat_accounts: game.seat_accounts(),
                        execution: game
                            .execution
                            .as_ref()
                            .expect("reserved identity")
                            .to_metadata(),
                    })),
                },
            ))
        }
    };
    let ((opening, seats), setup) = match setup {
        Ok(setup) => setup,
        Err(why) => {
            let _ = send(&mut socket, &refusal(&why)).await;
            end_game(&state, &game_id);
            return;
        }
    };
    if send(&mut socket, &setup).await.is_err() {
        end_game(&state, &game_id);
        return;
    }
    // Only now: a seat socket that sees this waits no longer, and everything
    // it then says has somewhere to go.
    //
    // `send_replace`, never `send`: a `watch::Sender` with no receivers left
    // refuses `send` *and keeps the old value*, and this channel routinely
    // has none — the engine can attach before the player who ordered it has
    // finished dialling their own seat. That socket then subscribes to a
    // channel still reading `false` and waits out the full timeout for an
    // engine that arrived before it did.
    {
        let lobby = state.lobby.lock();
        if let Some(game) = lobby.games.get(&game_id) {
            game.ready.send_replace(true);
        }
    }
    tracing::info!(game_id, "engine attached");
    let mut record = crate::record::Sink::open(state.db.clone(), &game_id, opening, seats);
    pump_engine(&state, &game_id, &mut socket, &mut rx, &mut record).await;
    // The socket is the game's lifeline: its state lives in that process and
    // nowhere else, so a link that closes before `GameEnded` is a game that
    // cannot be continued. Marking it over is the honest answer.
    let finished = {
        let mut lobby = state.lobby.lock();
        match lobby.games.get_mut(&game_id) {
            Some(game) => {
                let was_playing = game.state == LobbyState::Playing;
                game.finish(auth::now_secs());
                was_playing
            }
            None => false,
        }
    };
    if finished {
        tracing::warn!(game_id, "engine link lost; game closed");
    }
    state.lobby_moved();
    stop_engine(&state, &game_id);
}

/// One game's routing: seat frames both ways, until the game ends.
async fn pump_engine(
    state: &Shared,
    game_id: &str,
    socket: &mut WebSocket,
    rx: &mut mpsc::UnboundedReceiver<Envelope>,
    record: &mut crate::record::Sink,
) {
    // Each seat's channel (and the public spectator channel), looked up under
    // the lobby lock once and kept: a frame the engine sends then takes no
    // lock at all on its way out.
    let mut outboxes = Vec::new();
    let mut spectator_outbox = None;
    loop {
        tokio::select! {
            outbound = rx.recv() => {
                let Some(outbound) = outbound else { return };
                if send_queued(socket, &outbound, rx).await.is_err() {
                    return;
                }
            }
            frame = socket.recv() => {
                match next_envelope(frame) {
                    Incoming::Msg(v1::envelope::Msg::SeatFrame(seat_frame)) => {
                        let Some(outbox) = outbox_of(state, game_id, &mut outboxes, seat_frame.seat) else {
                            continue;
                        };
                        // No receivers is fine: a seat with no live socket
                        // is a seat nobody is waiting at.
                        let _ = outbox.send(seat_frame.envelope);
                    }
                    // The public table, to every spectator at once
                    // (`docs/protocol.md` §"Spectators").
                    Incoming::Msg(v1::envelope::Msg::SpectatorFrame(frame)) => {
                        if let Some(outbox) = spectator_outbox_of(state, game_id, &mut spectator_outbox) {
                            let _ = outbox.send(frame.envelope);
                        }
                    }
                    // Never forwarded: the record is omniscient (#315).
                    Incoming::Msg(v1::envelope::Msg::GameRecordChunk(piece)) => record.take(piece),
                    // A report's ask answered (#323): the piece it asked for
                    // came before this on the socket and is already handed
                    // to the writer, so the report is told once that is
                    // stored. This game's own asks only, whatever the frame
                    // says it is about. Never forwarded either.
                    Incoming::Msg(v1::envelope::Msg::RecordFlushed(ack)) => {
                        if let Some(done) = state.record_flushes.answered(game_id, ack.nonce) {
                            record.flushed(done);
                        }
                    }
                    Incoming::Msg(v1::envelope::Msg::SeatParticipationEnded(ended)) => {
                        // This socket proved the engine capability for this game.
                        // Never consume this evidence from a player-facing socket.
                        let changed = {
                            let mut lobby = state.lobby.lock();
                            lobby.games.get_mut(game_id).is_some_and(|game| {
                                game.end_participation(&ended.seats)
                            })
                        };
                        if changed {
                            state.lobby_moved();
                        }
                    }
                    Incoming::Msg(v1::envelope::Msg::GameEnded(ended)) => {
                        if ended.game_id != game_id { continue; }
                        tracing::info!(game_id, winners = ?ended.winners, reason = ended.reason, "game over");
                        let ranked = state.lobby.lock().games.get(game_id).is_some_and(|g| g.ranked.is_some());
                        let valid_duel = record.complete() && matches!(ended.winners.as_slice(), [] | [0 | 1]);
                        record.ended(crate::record::Ending {
                            winners: ended
                                .winners
                                .iter()
                                .filter_map(|&w| u8::try_from(w).ok())
                                .collect(),
                            turn: Some(ended.turn).filter(|&t| t != 0 && (!ranked || valid_duel)),
                            reason: ended.reason.clone(),
                        });
                        if let Some(result) = crate::challenges::completion(state, game_id, &ended)
                            && let Some(stored) = record.challenge(result) {
                            crate::record::finalizing(state, game_id, stored);
                        }
                        if ranked {
                            let (done, stored) = tokio::sync::oneshot::channel();
                            record.flushed(done);
                            crate::record::finalizing(state, game_id, stored);
                        }
                        end_game(state, game_id);
                        return;
                    }
                    Incoming::Msg(_) | Incoming::Ignored => {}
                    Incoming::Closed => return,
                }
            }
        }
    }
}
