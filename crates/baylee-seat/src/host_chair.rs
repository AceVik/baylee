//! Explicit, local stdin handoff of an already seated host's credentials.
use super::{Chair, Join, Leaving, Lobby, PoisonError, Session, Standing, TICKET_WAIT, Unstarted};
use anyhow::{bail, ensure};
use serde::Deserialize;

/// Never Debug/Serialize: these credentials belong only on stdin and HTTP headers.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Handoff {
    game_id: String,
    seat: u32,
    seat_token: String,
    session_token: String,
}

fn parse(line: &str, room: &str, seat: Option<u32>) -> anyhow::Result<(Session, Chair)> {
    ensure!(
        line.len() <= super::tether::FIRST_LINE_BYTES,
        "host chair handoff is too long"
    );
    // Serde errors may quote attacker-supplied values/field names; never propagate them.
    let value: Handoff = serde_json::from_str(line)
        .map_err(|_| anyhow::anyhow!("invalid host chair handoff JSON on stdin"))?;
    ensure!(
        value.game_id == room && value.seat == 0 && seat == Some(0),
        "host chair handoff must match the requested room and chair 0"
    );
    for token in [&value.seat_token, &value.session_token] {
        ensure!(
            !token.is_empty() && token.len() <= 4096 && token.bytes().all(|c| c.is_ascii_graphic()),
            "invalid host chair credentials on stdin"
        );
    }
    Ok((
        Session::from_token(value.session_token),
        Chair {
            game_id: value.game_id,
            seat: 0,
            seat_token: value.seat_token,
        },
    ))
}

pub(super) async fn sit(
    join: &Join,
    lobby: &Lobby,
    input: tokio::sync::oneshot::Receiver<String>,
    unstarted: &Unstarted,
) -> anyhow::Result<(Standing, Chair)> {
    let line = tokio::time::timeout(TICKET_WAIT, input)
        .await
        .map_err(|_| anyhow::anyhow!("no host chair handoff came on stdin"))?
        .map_err(|_| anyhow::anyhow!("stdin closed before a host chair handoff came"))?;
    let (session, chair) = parse(&line, &join.room, join.chair)?;
    let Some(room) = lobby.room(&session, &chair.game_id).await? else {
        bail!("host chair room is unavailable");
    };
    validate_room(&room)?;
    let by = Standing::Guest(session);
    *unstarted.lock().unwrap_or_else(PoisonError::into_inner) = Some(Leaving {
        lobby: lobby.clone(),
        by: by.clone(),
        room: chair.game_id.clone(),
    });
    if room.clock.decide_secs.is_some_and(super::is_blitz_clock) && !join.allow_blitz {
        bail!("host chair room clock is too short (use --allow-blitz explicitly)");
    }
    Ok((by, chair))
}

fn validate_room(room: &baylee_seat::lobby::Room) -> anyhow::Result<()> {
    ensure!(
        room.yours
            && room.waiting()
            && room
                .seats
                .iter()
                .any(|s| s.seat == 0 && s.you && s.taken && s.kind == "human"),
        "host chair handoff does not own a waiting room's chair 0"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
