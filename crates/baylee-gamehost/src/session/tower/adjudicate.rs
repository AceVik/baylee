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
    let Pending::GameOver(result) = session.engine.pending() else {
        return Err(Refused::Invalid);
    };
    if !ended {
        return Err(Refused::Invalid);
    }
    Ok(Verified {
        result: *result,
        turn: session.engine.state().turn.number,
        consumed: supplies.consumed(&session)?,
        record_sha256: lines.digest(),
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
