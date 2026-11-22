//! Explicit migration of the owner's existing deck collection. Each account-worker
//! job handles a single source step and yields to login, chat and other requests.
use super::{DecksPress, Kit, Live, Weight, controls, orders, parts, tokens};
use crate::lobby::{
    LobbyRequest, LobbyState, Mailbox, Phrase, Press, Reply, central, dispatch, offline,
};
use baylee_account::{Account, Error, GatewayDeckImport};
use bevy::prelude::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Device,
    Gateway,
    Retry,
    Close,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Running,
    Failed,
    Done,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Counts {
    scanned: usize,
    imported: usize,
}

enum Source {
    Device(offline::migrate::Device),
    Gateway(Box<GatewayDeckImport>),
}

impl Source {
    fn step(
        &mut self,
        account: &mut Account,
        host: Option<(String, zeroize::Zeroizing<String>)>,
        epoch: u64,
    ) -> (Result<bool, Error>, Counts) {
        match self {
            Self::Device(d) => (
                d.step(account),
                Counts {
                    scanned: d.scanned,
                    imported: d.imported,
                },
            ),
            Self::Gateway(progress) => {
                let result = host.ok_or(Error::Rejected).and_then(|(origin, bearer)| {
                    account.gateway_deck_import_step(&origin, &bearer, epoch, progress)
                });
                (
                    result,
                    Counts {
                        scanned: progress.scanned_decks(),
                        imported: progress.imported_decks(),
                    },
                )
            }
        }
    }
}

struct Job {
    session: central::Handle,
    epoch: u64,
    origin: Option<String>,
    active: Arc<AtomicBool>,
    source: Arc<Mutex<Source>>,
    counts: Counts,
    status: Status,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
    }
}

#[derive(Default)]
pub(in crate::lobby) struct Ui {
    open: bool,
    job: Option<Job>,
}

impl Ui {
    pub(in crate::lobby) fn stale(&self, state: &LobbyState) -> bool {
        self.job.as_ref().is_some_and(|j| {
            j.epoch != state.gateway_epoch
                || j.origin.as_deref().is_some_and(|origin| {
                    !state.gateway_selected || state.gateway.trim_end_matches('/') != origin
                })
                || !state.has_central_account()
                || !state
                    .central_handle()
                    .is_some_and(|h| h.is_active() && h.same_session(&j.session))
        })
    }
    pub(in crate::lobby) fn reset(&mut self) {
        *self = Self::default();
    }
}

pub(in crate::lobby) struct Update {
    active: Arc<AtomicBool>,
    result: Result<bool, Error>,
    counts: Counts,
}

fn submit(job: &Job, state: &LobbyState, mailbox: &Mailbox) -> bool {
    let host = if let Some(origin) = &job.origin {
        if state.gateway.trim_end_matches('/') != origin || state.gateway_epoch != job.epoch {
            return false;
        }
        let Some(bearer) = state.lobby.token() else {
            return false;
        };
        Some((origin.clone(), zeroize::Zeroizing::new(bearer.to_owned())))
    } else {
        None
    };
    let epoch = job.epoch;
    let active = Arc::clone(&job.active);
    let work = Arc::clone(&job.source);
    let inbox = mailbox.clone();
    job.session.submit(move |account| {
        if !active.load(Ordering::Acquire) {
            return;
        }
        let (result, counts) = work
            .lock()
            .map_or((Err(Error::Unavailable), Counts::default()), |mut s| {
                s.step(account, host, epoch)
            });
        if active.load(Ordering::Acquire)
            && let Ok(mut box_) = inbox.0.lock()
        {
            box_.push(Reply::DeckImport(Update {
                active,
                result,
                counts,
            }));
        }
    })
}

pub(in crate::lobby) fn receive(state: &mut ResMut<LobbyState>, mailbox: &Mailbox, update: Update) {
    if state.deck_import.stale(state) {
        state.deck_import.reset();
        return;
    }
    let Some(job) = state
        .deck_import
        .job
        .as_ref()
        .filter(|j| Arc::ptr_eq(&j.active, &update.active))
    else {
        return;
    };
    let status = match update.result {
        Ok(true) => Status::Done,
        Ok(false) if submit(job, state, mailbox) => Status::Running,
        Ok(false) | Err(_) => Status::Failed,
    };
    if (job.counts != update.counts || job.status != status)
        && let Some(job) = state.deck_import.job.as_mut()
    {
        job.counts = update.counts;
        job.status = status;
    }
    if status == Status::Done {
        dispatch(state, mailbox, Some(LobbyRequest::ListDecks));
    }
}

pub(super) fn press(state: &mut LobbyState, mailbox: &Mailbox, action: Action) {
    if !state.has_central_account() {
        return;
    }
    if state.deck_import.stale(state) {
        state.deck_import.reset();
    }
    match action {
        Action::Open => state.deck_import.open = true,
        Action::Close => {
            let refresh = state.deck_import.job.is_some();
            state.deck_import.reset();
            if refresh {
                dispatch(state, mailbox, Some(LobbyRequest::ListDecks));
            }
        }
        Action::Retry => {
            if let Some(job) = state
                .deck_import
                .job
                .as_ref()
                .filter(|j| j.status == Status::Failed)
                && submit(job, state, mailbox)
                && let Some(job) = state.deck_import.job.as_mut()
            {
                job.status = Status::Running;
            }
        }
        Action::Device | Action::Gateway => {
            if !state.deck_import.open || state.deck_import.job.is_some() {
                return;
            }
            let Some(session) = state.central_handle() else {
                return;
            };
            let source = match action {
                Action::Device => Source::Device(offline::migrate::Device::default()),
                Action::Gateway if state.gateway_selected && state.lobby.token().is_some() => {
                    Source::Gateway(Box::default())
                }
                _ => return,
            };
            let mut job = Job {
                session,
                epoch: state.gateway_epoch,
                origin: matches!(source, Source::Gateway(_))
                    .then(|| state.gateway.trim_end_matches('/').to_owned()),
                active: Arc::new(AtomicBool::new(true)),
                source: Arc::new(Mutex::new(source)),
                counts: Counts::default(),
                status: Status::Running,
            };
            if !submit(&job, state, mailbox) {
                job.status = Status::Failed;
            }
            state.deck_import.job = Some(job);
        }
    }
}

pub(super) fn draw(commands: &mut Commands, panel: Entity, state: &LobbyState, kit: Kit) {
    let row = parts::row(commands, kit, true);
    commands.entity(panel).add_child(row);
    let lang = state.lobby.lang();
    let mut actions = Vec::new();
    if let Some(job) = state
        .deck_import
        .job
        .as_ref()
        .filter(|_| !state.deck_import.stale(state))
    {
        let progress = Phrase::DeckMigrateProgress.fill(
            lang,
            &[
                &job.counts.scanned.to_string(),
                &job.counts.imported.to_string(),
            ],
        );
        let status = match job.status {
            Status::Running => progress,
            Status::Done => format!("{} · {progress}", Phrase::DeckMigrateDone.text(lang)),
            Status::Failed => format!("{} · {progress}", Phrase::DeckMigrateFailed.text(lang)),
        };
        let label = parts::line(commands, kit, &status, kit.m.small, tokens::INK);
        commands.entity(row).add_child(label);
        if job.status == Status::Failed {
            actions.push((Action::Retry, Phrase::ShellRetry, "migrate"));
        }
        actions.push((
            Action::Close,
            if job.status == Status::Running {
                Phrase::SheetCancel
            } else {
                Phrase::SheetClose
            },
            "migrate-close",
        ));
    } else if state.deck_import.open {
        let label = parts::line(
            commands,
            kit,
            Phrase::DeckMigrateHelp.text(lang),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(row).add_child(label);
        actions.push((Action::Device, Phrase::DeckMigrateDevice, "migrate"));
        if state.gateway_selected && state.lobby.token().is_some() {
            actions.push((
                Action::Gateway,
                Phrase::DeckMigrateGateway,
                "migrate-second",
            ));
            let origin = parts::line(commands, kit, &state.gateway, kit.m.small, tokens::MUTED);
            commands.entity(row).add_child(origin);
        }
        actions.push((Action::Close, Phrase::SheetCancel, "migrate-close"));
    } else {
        actions.push((Action::Open, Phrase::DeckMigrateOpen, "migrate"));
    }
    for (action, phrase, stop) in actions {
        let control = controls::button(
            commands,
            kit,
            phrase.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Decks(DecksPress::Migrate(action)),
        );
        orders::stop(commands, control, &orders::DECKS, stop);
        commands.entity(row).add_child(control);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_import_is_explicit_and_cannot_continue_under_another_account_or_gateway() {
        let mut state = LobbyState::new();
        central::restored_for_test(&mut state, vec![]);
        let (handle, jobs) = central::Handle::queued_for_test();
        state.central.as_mut().unwrap().handle = Some(handle);
        let mailbox = Mailbox::default();
        press(&mut state, &mailbox, Action::Device);
        assert!(jobs.try_recv().is_err());
        press(&mut state, &mailbox, Action::Open);
        press(&mut state, &mailbox, Action::Device);
        assert!(jobs.try_recv().is_ok());
        press(&mut state, &mailbox, Action::Device);
        assert!(jobs.try_recv().is_err(), "only one import in flight");
        assert!(!state.deck_import.stale(&state));
        let old = Arc::clone(&state.deck_import.job.as_ref().unwrap().active);
        state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
        assert!(state.deck_import.stale(&state));
        state.deck_import.reset();
        assert!(!old.load(Ordering::Acquire));

        press(&mut state, &mailbox, Action::Open);
        press(&mut state, &mailbox, Action::Gateway);
        assert!(
            jobs.try_recv().is_err(),
            "no gateway exchange without the owner's local bearer"
        );
        press(&mut state, &mailbox, Action::Device);
        assert!(jobs.try_recv().is_ok());
        let (other, _) = central::Handle::queued_for_test();
        state.central.as_mut().unwrap().handle = Some(other);
        assert!(state.deck_import.stale(&state));
    }
}
