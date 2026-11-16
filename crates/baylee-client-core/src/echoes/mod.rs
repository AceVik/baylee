//! Renderer-free v2 Tower navigation. Progression comes only from the global authority.
//! The native adapter owns account-scoped mailboxes and all credentials/network effects.
use api::{Cohort, Host, Location, Phase, Request, Response, Run};
pub use baylee_tower as domain;
pub use baylee_tower_api as api;
pub use central_api::Error;
pub use central_api::tower_awards as awards_api;
use domain::v2::run::Command;
use uuid::Uuid;

pub mod draft;
mod receive;
#[cfg(test)]
mod tests;
pub mod workshop;

/// Explicit read/navigation operation, never a progression mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Read {
    /// Global comparison for one exact cohort, with separate house-assisted results.
    Ranking(Uuid, api::ranking::Lane),
    /// Published enrollment choices, using an exclusive cursor.
    Cohorts(Option<Uuid>),
    /// Frozen catalog, including a run whose enrollment period has closed.
    Cohort(Uuid),
    /// Owned retained runs, using an exclusive cursor.
    Runs(Option<Uuid>),
    /// Inspect one owned run without reserving or opening its game.
    Run(Uuid),
    /// Approved available hosts for the selected run.
    Hosts(Option<Uuid>),
    /// Fresh execution location for the selected run.
    Locate,
}

/// One bounded account-worker operation. A retry has a fresh local work identity
/// but preserves the original mutation request byte-for-byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Work {
    id: Uuid,
    request: Option<Request>,
    play: Option<Uuid>,
}

impl Work {
    /// Authoritative read/choice to send first. None for an already reserved game.
    #[must_use]
    pub const fn request(&self) -> Option<&Request> {
        self.request.as_ref()
    }
    /// Only an explicit Play/Reserve/Retry may open a human seat after its request succeeds.
    #[must_use]
    pub const fn play(&self) -> Option<Uuid> {
        self.play
    }
}

/// Account-bound screen state. Dropping it on sign-out also drops pending choices.
#[derive(Default)]
pub struct State {
    ranking: Option<api::ranking::Board>,
    cohorts: Vec<Cohort>,
    publication: Option<Cohort>,
    consent: u16,
    runs: Vec<Run>,
    selected: Option<Uuid>,
    run: Option<Run>,
    hosts: Vec<Host>,
    location: Option<Location>,
    active: Option<Work>,
    pending: Option<Work>,
    failure: Option<Error>,
}

impl State {
    /// Last validated cohort comparison; never locally predicts a score or rank.
    #[must_use]
    pub const fn ranking(&self) -> Option<&api::ranking::Board> {
        self.ranking.as_ref()
    }

    /// Explicit publication choice using the last confirmed privacy revision.
    pub fn listing(&mut self, id: Uuid, listed: bool) -> Option<Work> {
        if self.uncertain() {
            return None;
        }
        let revision = self.ranking.as_ref()?.listing.revision;
        self.begin(id, Some(Request::Listing { revision, listed }), None)
    }
    /// Current enrollment page.
    #[must_use]
    pub fn cohorts(&self) -> &[Cohort] {
        &self.cohorts
    }
    /// Frozen catalog matching the selected run, independent of current enrollment.
    #[must_use]
    pub fn publication(&self) -> Option<&Cohort> {
        let id = self.run.as_ref()?.cohort;
        self.publication
            .iter()
            .chain(self.cohorts.iter())
            .find(|cohort| cohort.id == id)
    }
    /// Exact current explanation revision to display before explicit acceptance.
    #[must_use]
    pub const fn consent(&self) -> u16 {
        self.consent
    }
    /// Current owned run page.
    #[must_use]
    pub fn runs(&self) -> &[Run] {
        &self.runs
    }
    /// Selected owned run, absent while a different selection is loading.
    #[must_use]
    pub const fn run(&self) -> Option<&Run> {
        self.run.as_ref()
    }
    /// Available approved hosts for this selection, never a capacity reservation.
    #[must_use]
    pub fn hosts(&self) -> &[Host] {
        &self.hosts
    }
    /// Last confirmed current location, invalidated when progression changes.
    #[must_use]
    pub const fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }
    /// One account-worker request is outstanding.
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.active.is_some()
    }
    /// An ambiguous mutation must be retried/resolved before another mutation.
    #[must_use]
    pub const fn uncertain(&self) -> bool {
        self.pending.is_some()
    }
    /// Closed refusal category; never remote diagnostic text or an invented result.
    #[must_use]
    pub const fn failure(&self) -> Option<Error> {
        self.failure
    }

    /// Read state without creating, refreshing a shop, reserving or playing.
    pub fn read(&mut self, id: Uuid, read: Read) -> Option<Work> {
        if self.busy() || !uuid7(id) {
            return None;
        }
        let request = match read {
            Read::Ranking(cohort, lane) => {
                if !uuid7(cohort) {
                    return None;
                }
                Request::Ranking { cohort, lane }
            }
            Read::Cohorts(before) => Request::Cohorts { before },
            Read::Cohort(cohort) => {
                if !uuid7(cohort) {
                    return None;
                }
                Request::Cohort { cohort }
            }
            Read::Runs(before) => Request::Runs { before },
            Read::Run(run) => {
                if !uuid7(run) || (self.uncertain() && self.selected != Some(run)) {
                    return None;
                }
                if self.selected != Some(run) {
                    self.selected = Some(run);
                    self.run = None;
                    self.clear_route();
                }
                Request::Run { run }
            }
            Read::Hosts(before) => Request::Hosts {
                run: self.run.as_ref()?.id,
                before,
            },
            Read::Locate => Request::Locate {
                run: self.run.as_ref()?.id,
            },
        };
        self.begin(id, Some(request), None)
    }

    /// Explicitly enroll after displaying and accepting the current echo explanation.
    pub fn start(&mut self, id: Uuid, cohort: Uuid, starter: u8, consent: u16) -> Option<Work> {
        if self.uncertain()
            || consent == 0
            || consent != self.consent
            || !self.cohorts.iter().any(|published| {
                published.id == cohort && usize::from(starter) < published.starters.len()
            })
        {
            return None;
        }
        self.begin(
            id,
            Some(Request::Start {
                request: id,
                cohort,
                starter,
                consent,
            }),
            None,
        )
    }

    /// Shop/deck choice; never applies a local economy delta before the response.
    pub fn choose(&mut self, id: Uuid, command: Command) -> Option<Work> {
        let run = self.run.as_ref()?;
        if self.uncertain() || !matches!(run.phase, Phase::Shop | Phase::Reward) {
            return None;
        }
        self.begin(
            id,
            Some(Request::Command {
                request: id,
                run: run.id,
                revision: run.revision,
                command,
            }),
            None,
        )
    }

    /// Explicit Play commits the shown host choice and then opens that reserved duel.
    pub fn reserve(&mut self, id: Uuid, host: Uuid) -> Option<Work> {
        let run = self.run.as_ref()?;
        if self.uncertain()
            || run.phase != Phase::Shop
            || !self.hosts.iter().any(|shown| shown.id == host)
        {
            return None;
        }
        self.begin(
            id,
            Some(Request::Reserve {
                request: id,
                run: run.id,
                revision: run.revision,
                host,
            }),
            Some(run.id),
        )
    }

    /// Explicitly resume the existing reservation; no new opponent or shop request.
    pub fn play(&mut self, id: Uuid) -> Option<Work> {
        let run = self.run.as_ref()?;
        if self.uncertain() || !matches!(run.phase, Phase::Reserved | Phase::Playing) {
            return None;
        }
        self.begin(id, None, Some(run.id))
    }

    /// Retry precisely the pending mutation; another work UUID cannot reroll its identity.
    pub fn retry(&mut self, id: Uuid) -> Option<Work> {
        let pending = self.pending.as_ref()?;
        self.begin(id, pending.request.clone(), pending.play)
    }

    /// Explicitly withdraw future echo use; repeating this monotonic choice is harmless.
    pub fn withdraw(&mut self, id: Uuid) -> Option<Work> {
        if self.uncertain() {
            return None;
        }
        self.begin(id, Some(Request::WithdrawEchoes {}), None)
    }

    fn begin(&mut self, id: Uuid, request: Option<Request>, play: Option<Uuid>) -> Option<Work> {
        if self.busy() || !uuid7(id) {
            return None;
        }
        let work = Work { id, request, play };
        if work.request.as_ref().is_some_and(mutation) {
            self.pending = Some(work.clone());
        }
        self.active = Some(work.clone());
        self.failure = None;
        Some(work)
    }

    fn clear_route(&mut self) {
        self.hosts.clear();
        self.location = None;
    }
}

fn mutation(request: &Request) -> bool {
    matches!(
        request,
        Request::Start { .. }
            | Request::Command { .. }
            | Request::Reserve { .. }
            | Request::Listing { .. }
    )
}

fn uuid7(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}
