//! Synchronous local replay projection, independent of its worker transport.
use std::sync::atomic::{AtomicBool, AtomicU8};
use baylee_client_core::replay::{Header, Perspective, Tape, Verify};

/// Plays `record` (JSON Lines, inflated) again from `perspective` and reads
/// what the perspective is shown into a tape, reporting progress in
/// percent and stopping when `cancel` is set. Nothing is written to disk.
///
/// # Errors
/// The record cannot be played again on this build, in words a player may
/// read.
pub fn project(
    record: &[u8],
    names: Vec<String>,
    mut header: Header,
    perspective: Perspective,
    verify: Verify,
    progress: &AtomicU8,
    cancel: &AtomicBool,
) -> Result<Tape, String> {
    use baylee_client_core::replay::{StopLine, StopReason, TapeMoment};
    use baylee_core::preset::LoopPolicy;
    use baylee_engine::engine::{DetectLoops, NoLoopDetection};
    use baylee_gamehost::replay::{self as projection, Stop};
    use std::ops::ControlFlow;
    use std::sync::atomic::Ordering;
    let summary = projection::summary(record).map_err(|e| format!("{e:?}"))?;
    header.recorded_by.clone_from(&summary.build);
    let inputs = summary.inputs.max(1);
    let mut moments = Vec::new();
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
        if let Some(n) = moment.input {
            let percent = (n.saturating_add(1).saturating_mul(100) / inputs).min(100);
            progress.store(percent as u8, Ordering::Relaxed);
        }
        moments.push(TapeMoment {
            input: moment.input,
            at: moment.at,
            check: moment.check,
            frames: moment.frames,
        });
        if cancel.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
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
    let stop = StopLine {
        stop: match made.stop {
            Stop::End => StopReason::End,
            Stop::Cut => StopReason::Cut,
            Stop::Diverged { .. } => StopReason::Diverged,
            Stop::Refused { .. } => StopReason::Refused,
            Stop::Cancelled => StopReason::Cancelled,
        },
        inputs: made.inputs,
        exact_to: made.exact_to,
        seen_to: made.seen_to,
        diverged: made.diverged,
    };
    Tape::from_moments(header, moments, stop).map_err(|e| e.to_string())
}

