//! Account-bound expedition navigation. Every progression decision remains Central-owned.
use api::{Board, Phase, Request, Response, Run, Season, Track};
use central_api::challenges::Attempt;
pub use central_api::tower as api;
#[cfg(test)]
mod tests;

/// Stable expedition control identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open the expedition, refreshing its authoritative progress.
    Open,
    /// Open a particular public week from a website link; no mutation.
    Season([u8; 16]),
    /// Begin an expedition, or reserve/resume its next duel.
    Play,
    /// Inspect an improvement before committing it.
    Upgrade(Track),
    /// Confirm the currently inspected improvement.
    Choose,
    /// Ask before retiring the active expedition.
    Abandon,
    /// Confirm retirement.
    Confirm,
    /// Dismiss an improvement or retirement confirmation.
    Cancel,
    /// Toggle public-name consent explicitly.
    Visibility,
    /// Select an independent ranking.
    Board(Board),
    /// Page the ranking.
    Page(bool),
    /// Inspect permanent reward tiers.
    Badge(api::BadgeKind),
}

/// Expedition metadata and exact retry identities, discarded on account changes.
#[expect(
    clippy::struct_excessive_bools,
    reason = "navigation, consent, confirmation and loaded state are independent UI properties"
)]
pub struct State {
    /// Whether the expedition detail occupies the challenges pane.
    pub opened: bool,
    /// A specifically requested archive week; ordinary Lobby visits use the current week.
    pub selected: Option<[u8; 16]>,
    /// Latest published expedition.
    pub season: Option<Season>,
    /// The latest run, preferring an unfinished run from any week.
    pub run: Option<Run>,
    /// Current server clock.
    pub now: i64,
    /// Independent ranking selection.
    pub board: Board,
    /// Page offset, bounded by Central.
    pub offset: u32,
    /// Public scores for the selected expedition.
    pub standings: Vec<api::Standing>,
    /// Permanent, authoritative rewards.
    pub badges: Vec<api::Badge>,
    /// Selected upgrade, not committed until Choose.
    pub upgrade: Option<Track>,
    /// Explicit retirement confirmation.
    pub abandoning: bool,
    /// Public-name listing is off until explicitly chosen.
    pub listed: bool,
    /// Inspected reward, without claiming it is earned.
    pub badge: Option<api::BadgeKind>,
    /// A read completed, even when no expedition is available.
    pub loaded: bool,
    pending: Option<Request>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            opened: false,
            selected: None,
            season: None,
            run: None,
            now: 0,
            board: Board::Ascent,
            offset: 0,
            standings: Vec::new(),
            badges: Vec::new(),
            upgrade: None,
            abandoning: false,
            listed: false,
            badge: None,
            loaded: false,
            pending: None,
        }
    }
}

/// One expedition request; play is separate from reading or choosing an upgrade.
pub struct Work {
    /// Ordered authoritative operations.
    pub requests: Vec<Request>,
    /// Only explicit Play may admit a game.
    pub play: bool,
    /// An already reserved attempt needs no second reservation.
    pub resume: Option<Attempt>,
}

impl State {
    /// Read the visible expedition, preserving a past week's unfinished run.
    #[must_use]
    pub fn read(&self) -> Request {
        Request::Read {
            season: self.selected.or_else(|| {
                self.run
                    .as_ref()
                    .filter(|run| run.progress.phase.active())
                    .map(|run| run.season.id)
            }),
            board: self.board,
            offset: self.offset,
        }
    }

    /// The run's week may differ from the current public expedition.
    #[must_use]
    pub fn visible_season(&self) -> Option<&Season> {
        self.run
            .as_ref()
            .map(|run| &run.season)
            .or(self.season.as_ref())
    }

    /// Create an operation while retaining the exact identity of an uncertain mutation.
    pub fn act(&mut self, action: Action, request_id: [u8; 16]) -> Option<Work> {
        if self.abandoning && !matches!(action, Action::Confirm | Action::Cancel) {
            return None;
        }
        if self.pending.is_some()
            && matches!(
                action,
                Action::Upgrade(_) | Action::Cancel | Action::Abandon
            )
        {
            return None;
        }
        if self.preview(action) {
            return None;
        }
        let mut requests = Vec::new();
        let mut play = false;
        let mut resume = None;
        if self.pending.is_some() && matches!(action, Action::Play | Action::Choose) {
            let request = self.pending.clone()?;
            play = matches!(request, Request::Start { .. });
            requests.push(request);
        } else {
            match action {
                Action::Open => {
                    self.opened = true;
                    self.selected = None;
                }
                Action::Season(id) => {
                    self.opened = true;
                    self.selected = Some(id);
                }
                Action::Cancel | Action::Upgrade(_) | Action::Abandon | Action::Badge(_) => {
                    return None;
                }
                Action::Confirm => {
                    if !self.abandoning {
                        return None;
                    }
                    let run = self.run.as_ref()?;
                    requests.push(Request::Abandon {
                        run: run.id,
                        revision: run.revision,
                    });
                }
                Action::Choose => {
                    requests.push(self.choice(request_id)?);
                }
                Action::Play => {
                    let work = self.reservation(request_id)?;
                    requests = work.requests;
                    play = work.play;
                    resume = work.resume;
                }
                Action::Visibility => {
                    if self.run.is_none() {
                        self.listed = !self.listed;
                        return None;
                    }
                    requests.push(Request::Visibility {
                        season: self.visible_season()?.id,
                        listed: !self.listed,
                    });
                }
                Action::Board(board) => {
                    self.board = board;
                    self.offset = 0;
                    self.standings.clear();
                }
                Action::Page(next) => {
                    self.offset = if next {
                        (self.offset + 30).min(3000)
                    } else {
                        self.offset.saturating_sub(30)
                    };
                }
            }
            if let Some(
                request @ (Request::Begin { .. } | Request::Start { .. } | Request::Choose { .. }),
            ) = requests.first()
            {
                self.pending = Some(request.clone());
            }
        }
        if !play {
            requests.push(self.read());
        }
        Some(Work {
            requests,
            play,
            resume,
        })
    }

    fn preview(&mut self, action: Action) -> bool {
        match action {
            Action::Cancel => {
                self.upgrade = None;
                self.abandoning = false;
            }
            Action::Upgrade(track) => {
                if self
                    .run
                    .as_ref()
                    .is_some_and(|run| run.progress.phase == Phase::Choosing)
                {
                    self.upgrade = Some(track);
                }
            }
            Action::Abandon => {
                self.abandoning = self
                    .run
                    .as_ref()
                    .is_some_and(|run| run.progress.phase.active());
            }
            Action::Badge(kind) => {
                self.badge = (self.badge != Some(kind)).then_some(kind);
            }
            _ => return false,
        }
        true
    }

    /// An uncertain progression mutation must be resolved before offering another choice.
    #[must_use]
    pub fn uncertain(&self) -> bool {
        self.pending.is_some()
    }

    fn reservation(&self, request_id: [u8; 16]) -> Option<Work> {
        let season = self.visible_season()?;
        if season.withdrawn {
            return None;
        }
        let mut work = Work {
            requests: Vec::new(),
            play: false,
            resume: None,
        };
        if let Some(run) = self.run.as_ref().filter(|run| run.progress.phase.active()) {
            match run.progress.phase {
                Phase::Playing => {
                    work.resume = Some(run.attempt.clone()?);
                    work.play = true;
                }
                Phase::Ready => {
                    work.requests.push(Request::Start {
                        run: run.id,
                        revision: run.revision,
                        request_id,
                    });
                    work.play = true;
                }
                _ => return None,
            }
        } else {
            work.requests.push(Request::Begin {
                season: season.id,
                request_id,
                listed: self.listed,
            });
        }
        Some(work)
    }

    fn choice(&self, request_id: [u8; 16]) -> Option<Request> {
        let run = self.run.as_ref()?;
        if run.progress.phase != Phase::Choosing {
            return None;
        }
        Some(Request::Choose {
            run: run.id,
            revision: run.revision,
            track: self.upgrade?,
            request_id,
        })
    }

    /// Apply only responses received through the current account worker.
    pub fn receive(&mut self, response: Response) {
        match response {
            Response::Read {
                now,
                season,
                standings,
                run,
                active,
                badges,
            } => {
                self.now = now;
                if self.season.as_ref().map(|season| season.id)
                    != season.as_ref().map(|season| season.id)
                {
                    self.listed = false;
                }
                self.season = season.map(|season| *season);
                self.standings = standings;
                self.badges = badges;
                let next = active.or(run).map(|run| *run);
                if self.run.as_ref().map(|run| (run.id, run.revision))
                    != next.as_ref().map(|run| (run.id, run.revision))
                {
                    self.upgrade = None;
                    self.abandoning = false;
                }
                self.run = next;
                if let Some(run) = &self.run {
                    self.listed = run.listed;
                    let resolved = match &self.pending {
                        Some(Request::Begin { season, .. }) => *season == run.season.id,
                        Some(
                            Request::Start {
                                run: id, revision, ..
                            }
                            | Request::Choose {
                                run: id, revision, ..
                            },
                        ) => *id == run.id && run.revision > *revision,
                        _ => false,
                    };
                    if resolved {
                        self.pending = None;
                    }
                }
                self.loaded = true;
            }
            Response::Run { run } => {
                self.listed = run.listed;
                self.run = Some(*run);
                self.pending = None;
                self.upgrade = None;
                self.abandoning = false;
            }
            Response::Updated => {
                self.listed = !self.listed;
            }
            Response::Export { .. } => {}
        }
    }

    /// A definitive refusal permits a fresh revision after reloading; an uncertain
    /// transport result keeps the exact mutation identity for a safe retry.
    pub fn refused(&mut self, error: central_api::Error) {
        if matches!(
            error,
            central_api::Error::Conflict | central_api::Error::Rejected
        ) {
            self.pending = None;
        }
    }
}
