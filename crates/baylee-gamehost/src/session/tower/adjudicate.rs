use super::{
    Checker, GamePreset, Instant, Line, Pending, RECORD_VERSION, Refused, Session, Sources,
    Verified, budget::Limits, record, replay, stream::Lines,
};
use crate::session::SessionRegistry;
use baylee_engine::engine::LoopDetection;
use baylee_protocol::execution::Identity;
use std::io::BufRead;

pub(super) fn verify<D: LoopDetection, R: SessionRegistry>(
    preset: &GamePreset,
    identity: Option<&Identity>,
    source: impl BufRead,
    limits: Limits,
    deadline: Instant,
) -> Result<Verified, Refused> {
    let rebuilt = rebuild::<D, R>(preset, identity, source, limits, deadline)?;
    let Pending::GameOver(result) = rebuilt.session.engine.pending() else {
        return Err(Refused::Invalid);
    };
    if !rebuilt.ended {
        return Err(Refused::Invalid);
    }
    Ok(Verified {
        result: *result,
        turn: rebuilt.session.engine.state().turn.number,
        consumed: rebuilt.supplies.consumed(&rebuilt.session)?,
        record_sha256: rebuilt.digest,
    })
}

pub(super) struct Rebuilt<D: LoopDetection, R: SessionRegistry> {
    pub session: Session<D, R>,
    pub ended: bool,
    pub next: u64,
    pub at: u64,
    supplies: Sources,
    digest: [u8; 32],
}

/// The same strict reconstruction serves terminal adjudication and live recovery.
pub(super) fn rebuild<D: LoopDetection, R: SessionRegistry>(
    preset: &GamePreset,
    identity: Option<&Identity>,
    source: impl BufRead,
    limits: Limits,
    deadline: Instant,
) -> Result<Rebuilt<D, R>, Refused> {
    if Instant::now() >= deadline {
        return Err(Refused::Budget);
    }
    let mut lines = Lines::new(source, limits.resources);
    let (mut session, mut checker) =
        begin::<D, R>(preset, identity, lines.next()?.ok_or(Refused::Invalid)?)?;
    let supplies = Sources::of(&session);
    let (mut ended, mut previous_time, mut next) = (false, 0, 0);
    while let Some(line) = lines.next()? {
        if ended {
            return Err(Refused::Invalid);
        }
        limits.check(&session, deadline)?;
        let at = replay::apply(&mut session, &mut checker, line, next, &mut ended, limits)?;
        if at < previous_time {
            return Err(Refused::Invalid);
        }
        previous_time = at;
        next += 1;
    }
    limits.check(&session, deadline)?;
    // A terminal input and its End belong to one committed operation. Restoring
    // an input without its terminal metadata would accept a torn operation.
    if ended != matches!(session.engine.pending(), Pending::GameOver(_)) {
        return Err(Refused::Invalid);
    }
    Ok(Rebuilt {
        session,
        ended,
        next,
        at: previous_time,
        supplies,
        digest: lines.digest(),
    })
}

fn begin<D: LoopDetection, R: SessionRegistry>(
    preset: &GamePreset,
    expected: Option<&Identity>,
    header: Line,
) -> Result<(Session<D, R>, Checker), Refused> {
    let Line::Header {
        record: RECORD_VERSION,
        build,
        preset: recorded,
        hash,
        seen,
        execution,
    } = header
    else {
        return Err(Refused::Invalid);
    };
    if build != record::this_build()
        || recorded.as_ref() != preset
        || expected.is_some_and(|expected| execution.as_ref() != Some(expected))
    {
        return Err(Refused::Invalid);
    }
    let session = Session::with_detection(preset).ok_or(Refused::Invalid)?;
    let mut checker = Checker::for_build(&build);
    checker
        .check(&session.engine, None, hash, seen.as_deref())
        .map_err(|_| Refused::Invalid)?;
    Ok((session, checker))
}
