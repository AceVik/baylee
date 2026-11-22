//! Only one public board read is in flight. Changes of filter replace the
//! queued read, and an old reply can never replace the newly selected board.
use super::{Error, Request, Response, Worker};
use std::sync::{Arc, Mutex};

pub(super) type Inbox = Arc<Mutex<Option<Result<Response, Error>>>>;

pub(super) fn request(worker: &mut Worker, request: Request) {
    if !matches!(request, Request::Board { .. } | Request::Seasons { .. }) {
        return;
    }
    if worker.read_shown.as_ref() != Some(&request) {
        worker.board = None;
        worker.read_shown = Some(request.clone());
    }
    worker.read_wanted = Some(request);
    worker.discard_read = worker.reading.is_some();
    worker.board_busy = true;
    worker.board_error = None;
    worker.changed();
}

pub(super) fn poll(worker: &mut Worker) {
    if let Some(inbox) = &worker.reading {
        let done = inbox.try_lock().ok().and_then(|mut slot| slot.take());
        let Some(done) = done else { return };
        worker.reading = None;
        if !std::mem::take(&mut worker.discard_read) {
            worker.board_busy = false;
            match done {
                Ok(response) => worker.board = Some(response),
                Err(error) => worker.board_error = Some(error),
            }
            worker.changed();
        }
    }
    if let Some(request) = worker.read_wanted.take() {
        let inbox: Inbox = Arc::default();
        let writer = Arc::clone(&inbox);
        match std::thread::Builder::new()
            .name("ranked-leaderboard".into())
            .spawn(move || {
                let result = crate::lobby::central::endpoint()
                    .and_then(|endpoint| baylee_account::ranked_read(&endpoint, request));
                if let Ok(mut slot) = writer.lock() {
                    *slot = Some(result);
                }
            }) {
            Ok(_) => worker.reading = Some(inbox),
            Err(_) => {
                worker.board_error = Some(Error::Unavailable);
                worker.board_busy = false;
                worker.changed();
            }
        }
    }
}
