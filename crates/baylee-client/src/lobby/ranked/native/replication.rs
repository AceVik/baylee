//! Account-authenticated seat routing. Only the two account reads below select
//! a playing origin. A host socket cannot nominate its own successor.
use super::{NetworkHost, RankedSocket, Reply, Worker, ticket};
use baylee_account::{
    Account, Error, RankedHostDocuments, RankedPlay,
    ranked::{
        QueueState, Request, Response,
        replica::{ConsentState, Fence, Phase, SeatRequest, SeatResponse, SeatStatus},
    },
};
use std::sync::Arc;
use std::time::Instant;

pub(super) struct Route {
    queue: QueueState,
    authority: Option<SeatStatus>,
    documents: Option<Arc<RankedHostDocuments>>,
}

pub(super) enum Submission {
    Receipt(u64),
    Status(SeatStatus),
}

pub(super) fn answered(
    table: &super::Table,
    id: baylee_account::ranked::Id,
    result: Result<Submission, Error>,
    now: Instant,
) {
    match result {
        Ok(Submission::Receipt(position)) => table.answered(id, Ok(position), now),
        Ok(Submission::Status(status)) => {
            table.observe(status);
            table.answered(id, Err(Error::Busy), now);
        }
        Err(error) => table.answered(id, Err(error), now),
    }
}

/// One account read. A join carries `ai`, whom its key was minted to accept
/// (the person's explicit choice, people only when they made none).
pub(super) fn read(
    account: &mut Account,
    request: Request,
    ai: baylee_account::ranked::ai::AiOpponents,
    presence: Option<(Fence, bool)>,
    cached: Option<Arc<RankedHostDocuments>>,
) -> Result<Route, Error> {
    let queue = match request {
        Request::Join { key, format, deck } => {
            match account.replica(SeatRequest::Join {
                key,
                format,
                deck,
                ai,
            })? {
                SeatResponse::Queue { state } => state,
                _ => return Err(Error::Rejected),
            }
        }
        other => match account.ranked(other)? {
            Response::Queue { state } => state,
            _ => return Err(Error::Rejected),
        },
    };
    let authority = if let QueueState::Assigned { assignment, .. } = &queue {
        let ask = match presence {
            Some((fence, connected)) if fence.match_id == assignment.match_id => {
                SeatRequest::Presence { fence, connected }
            }
            _ => SeatRequest::Status {
                match_id: assignment.match_id,
            },
        };
        // A stale presence heartbeat is expected on a host transition. Learn
        // the current fence immediately instead of retrying a revoked one.
        let result = match account.replica(ask) {
            Err(Error::Conflict) => account.replica(SeatRequest::Status {
                match_id: assignment.match_id,
            }),
            reply => reply,
        }?;
        let SeatResponse::Status { status } = result else {
            return Err(Error::Rejected);
        };
        if status.fence.match_id != assignment.match_id
            || status.host_id != assignment.host_id
            || status.origin != assignment.origin
        {
            return Err(Error::Conflict);
        }
        Some(status)
    } else {
        None
    };
    let documents = if let Some(status) = &authority
        && status.consent == ConsentState::Needed
    {
        Some(super::host_documents::read(account, status.fence, cached)?)
    } else {
        None
    };
    Ok(Route {
        queue,
        authority,
        documents,
    })
}

pub(super) fn observe(worker: &mut Worker, route: Route) -> QueueState {
    worker.documents = route.documents;
    if let Some(status) = route.authority {
        if !matches!(&worker.queue, QueueState::Assigned{assignment,..} if assignment.match_id == status.fence.match_id)
        {
            worker.table.unavailable();
            worker.table = super::Table::default();
        }
        if worker.table.observe(status) {
            worker.play_paused = false;
            worker.changed();
        }
    } else {
        worker.table.unavailable();
        worker.table = super::Table::default();
    }
    route.queue
}

pub(super) fn connect(
    worker: &Worker,
    play: RankedPlay,
    previous_protection: Option<bool>,
) -> Result<(NetworkHost, RankedSocket), Error> {
    let authority = worker
        .table
        .expected(&play.assignment)
        .ok_or(Error::Conflict)?;
    if play.fence != authority.fence {
        return Err(Error::Conflict);
    }
    let marker = RankedSocket {
        generation: authority.fence.generation,
        look: worker
            .frozen
            .as_ref()
            .and_then(|(_, frozen)| (frozen.digest == play.assignment.deck).then_some(frozen.look)),
        match_id: authority.fence.match_id,
        game_id: *play.seat.game_id.as_bytes(),
        protected: authority.phase != Phase::Single,
        seat: play.assignment.seat,
        opponent_ai: play.assignment.opponent_ai.clone(),
    };
    if previous_protection.is_some_and(|was| was != marker.protected) {
        return Err(Error::Conflict);
    }
    let binding = worker.table.attach(&authority).ok_or(Error::Conflict)?;
    let host = NetworkHost::ranked(ticket(play), binding).map_err(|_| Error::Unavailable)?;
    Ok((host, marker))
}

pub(super) fn new_assignment(worker: &Worker, reply: &Reply) -> bool {
    let queue = match reply {
        Reply::Authority(_, Ok(route)) => &route.queue,
        Reply::Queue(_, Ok(queue)) => queue,
        _ => return false,
    };
    matches!(queue, QueueState::Assigned{assignment,..} if
        !matches!(&worker.queue, QueueState::Assigned{assignment: old,..} if old.match_id == assignment.match_id))
}

pub(super) fn send_pending(worker: &mut Worker) -> bool {
    let Some(request @ SeatRequest::Submit { command_id, .. }) =
        worker.table.submission(Instant::now())
    else {
        return false;
    };
    worker.submit(move |account| {
        let receipt = account.replica(request).and_then(|reply| match reply {
            SeatResponse::Receipt {
                command_id: acknowledged,
                position,
            } if acknowledged == command_id => Ok(Submission::Receipt(position)),
            SeatResponse::Status { status } => Ok(Submission::Status(status)),
            _ => Err(Error::Rejected),
        });
        Reply::Submitted(command_id, receipt)
    });
    true
}
