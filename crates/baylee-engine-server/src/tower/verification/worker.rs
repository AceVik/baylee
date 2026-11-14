use super::{Refused, ReplayBudget, verify_record};
use baylee_tower::v2::verification::{
    MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, Rejected, Request, Response,
};
use std::{
    io::{self, Read, Write},
    time::{Duration, Instant},
};

/// One private verifier process invocation. No network, database or auth secrets
/// are read. stdout carries one framed result; stderr carries only progress bytes.
/// The owner must continuously drain progress and enforce memory/wall limits.
/// # Errors
/// Private framing/pipe/liveness failure; no result may be committed in that case.
pub fn worker_entry() -> io::Result<()> {
    // Panics must never print hidden record/card values into the progress pipe.
    std::panic::set_hook(Box::new(|_| {}));
    let mut progress = baylee_worker::Reporter::new(Some(io::stderr()))?;
    progress.work(|| {
        let _liveness = baylee_worker::watch_progress_owner()?;
        run(&mut io::stdin().lock(), &mut io::stdout().lock())
    })?
}

pub(super) fn run(input: &mut impl Read, output: &mut impl Write) -> io::Result<()> {
    let header = baylee_worker::frame::read(input, MAX_REQUEST_BYTES)?;
    let response = match Request::decode(&header) {
        Ok(request) => adjudicate(&request, input),
        Err(baylee_tower::Refusal::Capacity) => Response::Rejected {
            reason: Rejected::Budget,
        },
        Err(_) => Response::Rejected {
            reason: Rejected::Invalid,
        },
    };
    let bytes = serde_json::to_vec(&response)
        .map_err(|_| io::Error::other("Tower result encoding failed"))?;
    baylee_worker::frame::write(output, &bytes, MAX_RESPONSE_BYTES)
}

fn adjudicate(request: &Request, source: impl Read) -> Response {
    let allocated = request.resources;
    let resources = usize::try_from(allocated.bytes).and_then(|bytes| {
        Ok(ReplayBudget {
            bytes,
            lines: usize::try_from(allocated.lines)?,
            line_bytes: usize::try_from(allocated.line_bytes)?,
            events: usize::try_from(allocated.events)?,
        })
    });
    let Ok(resources) = resources else {
        return Response::Rejected {
            reason: Rejected::Budget,
        };
    };
    let deadline = Instant::now() + Duration::from_millis(u64::from(allocated.millis));
    match verify_record(
        &request.package,
        &request.catalog,
        request.binding,
        source,
        resources,
        deadline,
    ) {
        Ok(settlement) => Response::Verified { settlement },
        Err(Refused::Invalid) => Response::Rejected {
            reason: Rejected::Invalid,
        },
        Err(Refused::Budget) => Response::Rejected {
            reason: Rejected::Budget,
        },
    }
}
