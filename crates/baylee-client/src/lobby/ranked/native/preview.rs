//! Explicitly requested private deck-Elo previews. Edits invalidate the
//! displayed result, while moving between choices supersedes outstanding reads.
use super::{Error, Format, LobbyState, Worker, central, deck};
use baylee_account::ranked::{DeckPreview, PoolEntry};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone, Copy)]
struct Query {
    generation: u64,
    format: Format,
    source: deck::Source,
}

type Inbox = Arc<Mutex<Option<(Query, Result<(u64, DeckPreview), Error>)>>>;

#[derive(Default)]
pub(super) struct Jobs {
    generation: u64,
    wanted: Option<(Instant, Query)>,
    live: Option<Inbox>,
    shown: Option<(Uuid, u64)>,
}

pub(crate) fn clear_preview(state: &mut LobbyState) {
    clear(&mut state.ranked_net);
}

fn clear(worker: &mut Worker) {
    worker.preview_jobs.generation = worker.preview_jobs.generation.wrapping_add(1);
    worker.preview_jobs.shown = None;
    worker.preview_jobs.wanted = None;
    if worker.preview.take().is_some() || worker.preview_busy || worker.preview_error.is_some() {
        worker.preview_busy = false;
        worker.preview_error = None;
        worker.changed();
    }
}

pub(crate) fn preview(state: &mut LobbyState, format: Format, id: &str) {
    let handle = state.central_handle().filter(central::Handle::is_active);
    state.ranked_net.session(handle);
    clear(&mut state.ranked_net);
    if !state.central_ready() || state.central_terms_pending() {
        return;
    }
    let source = deck::Source::selected(state, id, format);
    let worker = &mut state.ranked_net;
    match source {
        Ok(source) => queue(worker, format, source),
        Err(error) => worker.preview_error = Some(error),
    }
    worker.changed();
}

fn queue(worker: &mut Worker, format: Format, source: deck::Source) {
    worker.preview_busy = true;
    worker.preview_jobs.wanted = Some((
        Instant::now() + Duration::from_millis(180),
        Query {
            generation: worker.preview_jobs.generation,
            format,
            source,
        },
    ));
}

pub(super) fn poll(worker: &mut Worker, desk_open: bool) {
    let Some(handle) = worker.session.as_ref() else {
        return;
    };
    if let Some((id, revision)) = worker.preview_jobs.shown
        && handle.deck_revision_changed(id, revision)
    {
        let refresh = if desk_open
            && let (Some((format, _, _)), Ok(head)) =
                (&worker.preview, handle.listed_deck_revision(id))
        {
            Some((*format, deck::Source::at_revision(id, head)))
        } else {
            None
        };
        clear(worker);
        if let Some((format, source)) = refresh {
            queue(worker, format, source);
            worker.changed();
        }
        return;
    }
    if let Some(inbox) = &worker.preview_jobs.live {
        let Some((query, result)) = inbox.try_lock().ok().and_then(|mut slot| slot.take()) else {
            return;
        };
        worker.preview_jobs.live = None;
        if query.generation == worker.preview_jobs.generation {
            worker.preview_busy = false;
            let id = query.source.id();
            match result {
                Ok((revision, value))
                    if value.format == query.format
                        && !handle.deck_revision_changed(id, revision) =>
                {
                    worker.preview = Some((query.format, id, value));
                    worker.preview_jobs.shown = Some((id, revision));
                }
                Ok(_) => worker.preview_error = Some(Error::Conflict),
                Err(error) => worker.preview_error = Some(error),
            }
            worker.changed();
        }
        return;
    }
    let Some((at, query)) = worker.preview_jobs.wanted else {
        return;
    };
    if at > Instant::now() {
        return;
    }
    worker.preview_jobs.wanted = None;
    let inbox: Inbox = Arc::default();
    let writer = inbox.clone();
    let accepted = handle.submit(move |account| {
        let result = query
            .source
            .read(account, query.format)
            .and_then(|snapshot| {
                let pool = baylee_cards::ranked::rating_pool(&snapshot.registration)
                    .into_iter()
                    .map(|(card, copies)| PoolEntry { card, copies })
                    .collect();
                let value = account.preview_ranked_deck(query.format, pool)?;
                Ok((snapshot.revision, value))
            });
        if let Ok(mut slot) = writer.lock() {
            *slot = Some((query, result));
        }
    });
    if accepted {
        worker.preview_jobs.live = Some(inbox);
    } else {
        worker.preview_error = Some(Error::Busy);
        worker.preview_busy = false;
        worker.changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_account::ranked::{League, PreviewSource, Rank};

    #[test]
    fn clearing_a_preview_discards_the_result_from_a_superseded_deck_choice() {
        let (session, _jobs) = central::Handle::queued_for_test();
        let mut worker = Worker::default();
        worker.session(Some(session));
        let format = Format::Commander;
        let query = Query {
            generation: worker.preview_jobs.generation,
            format,
            source: deck::Source::at_revision(Uuid::now_v7(), 1),
        };
        let unseen = DeckPreview {
            format,
            source: PreviewSource::New,
            elo: 1000,
            edit_loss: 0,
            rank: Rank {
                league: League::Paper,
                division: Some(4),
                champion_place: None,
            },
            placement_games: 0,
            changed_copies: 0,
            compared_copies: 0,
        };
        worker.preview_busy = true;
        worker.preview_jobs.live = Some(Arc::new(Mutex::new(Some((query, Ok((1, unseen)))))));
        clear(&mut worker);
        poll(&mut worker, true);
        assert!(worker.preview.is_none());
        assert!(worker.preview_error.is_none());
        assert!(!worker.preview_busy);
    }
}
