use super::{
    AIProfile, Checker, FormatId, GamePreset, Instant, Line, MAX_EVENTS, MAX_LINE_BYTES,
    MAX_RECORD_BYTES, MAX_RECORD_LINES, Pending, PlayerId, Refused, SeatController, Session,
};

pub(super) fn preflight(
    preset: &GamePreset,
    bytes: &[u8],
    deadline: Instant,
) -> Result<(), Refused> {
    if Instant::now() >= deadline {
        return Err(Refused::Budget);
    }
    if bytes.is_empty() || bytes.len() > MAX_RECORD_BYTES {
        return Err(Refused::Budget);
    }
    if !bytes.ends_with(b"\n")
        || preset.format != FormatId::Custom
        || preset.validate().is_err()
        || !super::super::challenge::valid(preset)
        || preset.house_rules.challenge_policy_seed.is_none()
        || preset.seats[0].controller != SeatController::Open
        || preset.seats[1].controller != SeatController::Ai(AIProfile::TOWER_EXPERT)
        || preset
            .seats
            .iter()
            .any(|seat| seat.capabilities != baylee_core::preset::SeatCapabilities::default())
    {
        return Err(Refused::Invalid);
    }
    for (n, line) in bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        if n >= MAX_RECORD_LINES || line.len() > MAX_LINE_BYTES {
            return Err(Refused::Budget);
        }
        if line.is_empty() {
            return Err(Refused::Invalid);
        }
    }
    Ok(())
}

pub(super) fn budget(session: &Session, deadline: Instant) -> Result<(), Refused> {
    if Instant::now() >= deadline
        || session.engine.journal().len() > MAX_EVENTS
        || session.engine.state().turn.number > 500
    {
        return Err(Refused::Budget);
    }
    Ok(())
}

pub(super) fn apply(
    session: &mut Session,
    checker: &mut Checker,
    line: Line,
    next: u64,
    ended: &mut bool,
) -> Result<u64, Refused> {
    match line {
        Line::Input {
            n,
            at,
            seat,
            by,
            action,
            hash,
            seen,
        } if n == next => {
            if matches!(session.engine.pending(), Pending::GameOver(_)) {
                return Err(Refused::Invalid);
            }
            session.tell_time(at);
            if !session.replayed(PlayerId::new(seat), by, action) {
                return Err(Refused::Invalid);
            }
            checker
                .check(&session.engine, Some(n), hash, seen.as_deref())
                .map_err(|_| Refused::Invalid)?;
            Ok(at)
        }
        // A seat's self-description never changes the policy or the result.
        Line::DeclaredMind { n, at, seat: 0, .. } if n == next => Ok(at),
        Line::End {
            n,
            at,
            winners,
            reason,
            turn,
        } if n == next => {
            let Pending::GameOver(result) = session.engine.pending() else {
                return Err(Refused::Invalid);
            };
            let actual: Vec<u8> = session
                .winning_seats(*result)
                .iter()
                .map(|seat| seat.get())
                .collect();
            if winners != actual
                || reason != format!("{:?}", result.reason)
                || turn != Some(session.engine.state().turn.number)
                || !(1..=500).contains(&session.engine.state().turn.number)
            {
                return Err(Refused::Invalid);
            }
            *ended = true;
            Ok(at)
        }
        _ => Err(Refused::Invalid),
    }
}
