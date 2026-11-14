//! Synchronous projection shared by the owned worker and explicit in-process callers.
use baylee_client_core::replay::{
    Header, Perspective, StopLine, StopReason, Tape, TapeMoment, Verify,
};
use baylee_gamehost::replay as projection;
use std::{
    ops::ControlFlow,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

/// Projects a record synchronously. The desktop UI uses [`contained`] instead;
/// this seam remains for deterministic tests and experimental mobile hosts.
///
/// # Errors
/// A malformed record, unsupported policy or invalid projected payload.
pub fn project(
    record: &[u8],
    names: Vec<String>,
    mut header: Header,
    perspective: Perspective,
    verify: Verify,
    progress: &AtomicU8,
    cancel: &AtomicBool,
) -> Result<Tape, String> {
    let summary = projection::summary(record).map_err(|e| format!("{e:?}"))?;
    header.recorded_by.clone_from(&summary.build);
    let inputs = summary.inputs.max(1);
    let mut moments = Vec::new();
    let stop = execute(record, names, perspective, verify, |moment| {
        if let Some(n) = moment.input {
            progress.store(percent(n, inputs), Ordering::Relaxed);
        }
        moments.push(moment);
        if cancel.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })?;
    Tape::from_moments(header, moments, stop).map_err(|e| e.to_string())
}

/// The desktop host's projector, isolated behind a killable process boundary.
pub(crate) fn contained(
    record: &[u8],
    names: Vec<String>,
    mut header: Header,
    perspective: Perspective,
    verify: Verify,
    progress: &AtomicU8,
    cancel: &AtomicBool,
) -> Result<Tape, String> {
    header.perspective = baylee_client_core::replay::tape_perspective(perspective);
    header.seat = perspective.seat().get();
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        crate::host::process::projection::Projection {
            record: record.to_vec(),
            names,
            header,
            verify,
        }
        .project(progress, cancel)
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    project(record, names, header, perspective, verify, progress, cancel)
}

/// Emits complete tape lines in the child. No kept files are read or written.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) fn stream(
    record: &[u8],
    names: Vec<String>,
    mut header: Header,
    verify_note: bool,
    mut emit: impl FnMut(Vec<u8>, u8, bool) -> Result<(), String>,
) -> Result<(), String> {
    use baylee_protocol::tape::{self, Line};
    let summary = projection::summary(record).map_err(|e| format!("{e:?}"))?;
    header.recorded_by = summary.build;
    let perspective = baylee_client_core::replay::perspective_of(&header);
    let mut line = Vec::new();
    tape::write_line(&mut line, &Line::Header(header));
    let mut total = line.len();
    emit(line, 0, false)?;
    let mut failure = None;
    let mut progress = 0;
    let stop = execute(
        record,
        names,
        perspective,
        if verify_note {
            Verify::Note
        } else {
            Verify::Stop
        },
        |moment| {
            if let Some(n) = moment.input {
                progress = percent(n, summary.inputs.max(1));
            }
            let result = baylee_client_core::replay::tape::encode_moment(&moment)
                .map_err(|e| e.to_string())
                .and_then(|lines| {
                    total = total.saturating_add(lines.len());
                    if total > baylee_client_core::bugreport::MAX_INFLATED_BYTES {
                        return Err("the local replay exceeds its size limit".into());
                    }
                    emit(lines, progress, false)
                });
            match result {
                Ok(()) => ControlFlow::Continue(()),
                Err(why) => {
                    failure = Some(why);
                    ControlFlow::Break(())
                }
            }
        },
    )?;
    if let Some(why) = failure {
        return Err(why);
    }
    let mut line = Vec::new();
    tape::write_line(&mut line, &Line::Stop(stop));
    emit(line, 100, true)
}

fn percent(n: u64, inputs: u64) -> u8 {
    (n.saturating_add(1).saturating_mul(100) / inputs).min(100) as u8
}

fn execute(
    record: &[u8],
    names: Vec<String>,
    perspective: Perspective,
    verify: Verify,
    mut emit: impl FnMut(TapeMoment) -> ControlFlow<()>,
) -> Result<StopLine, String> {
    use baylee_core::preset::LoopPolicy;
    use baylee_engine::engine::{DetectLoops, NoLoopDetection};
    let perspective = match perspective {
        Perspective::Seat(seat) => projection::Perspective::Seat(seat),
        Perspective::Public => projection::Perspective::Public,
        Perspective::AllHands(seat) => projection::Perspective::AllHands(seat),
    };
    let verify = match verify {
        Verify::Stop => projection::Verify::Stop,
        Verify::Note => projection::Verify::Note,
    };
    let emit = |moment: projection::Moment| {
        emit(TapeMoment {
            input: moment.input,
            at: moment.at,
            check: moment.check,
            frames: moment.frames,
        })
    };
    let policy = super::local::preset(record)
        .ok_or("the record has no valid preset")?
        .house_rules
        .loop_policy;
    let made = match policy {
        LoopPolicy::Off => projection::project_with_detection::<NoLoopDetection>(
            record,
            names,
            perspective,
            verify,
            emit,
        ),
        LoopPolicy::RunOnceThenBreak | LoopPolicy::CompRulesDraw => {
            projection::project_with_detection::<DetectLoops>(
                record,
                names,
                perspective,
                verify,
                emit,
            )
        }
    }
    .map_err(|e| format!("{e:?}"))?;
    Ok(StopLine {
        stop: match made.stop {
            projection::Stop::End => StopReason::End,
            projection::Stop::Cut => StopReason::Cut,
            projection::Stop::Diverged { .. } => StopReason::Diverged,
            projection::Stop::Refused { .. } => StopReason::Refused,
            projection::Stop::Cancelled => StopReason::Cancelled,
        },
        inputs: made.inputs,
        exact_to: made.exact_to,
        seen_to: made.seen_to,
        diverged: made.diverged,
    })
}
