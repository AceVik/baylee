//! Central's global tasks: renderer-free navigation, explicit consent and retry identity.
use api::{Attempt, Badge, Board, Challenge, Request, Response, TaskProgress, Words};
pub use central_api::challenges as api;

/// A stable control identity, independent of a rebuilt UI tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Current tasks and account progress.
    Refresh,
    /// Next archive page.
    Older,
    /// Open a task, including one addressed by a website link.
    Select([u8; 16]),
    /// Return to the catalogue.
    Back,
    /// Choose a ranking.
    Board(Board),
    /// Page a ranking, keeping the current board.
    Page(bool),
    /// Begin or resume; a retry retains the reservation identity.
    Play,
    /// Toggle public listing; never implicit in playing.
    Visibility,
    /// Ask before revealing a hint.
    Hint,
    /// Ask before abandoning a started attempt.
    Abandon,
    /// Perform exactly the confirmed operation.
    Confirm,
    /// Dismiss the confirmation.
    Cancel,
    /// Expand or fold the supplied deck list.
    Deck,
}

/// The consequence the player is asked to accept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirmation {
    /// Hints permanently mark this task assisted for this account.
    Hint(u8),
    /// A deliberate abandonment counts as a failure after the host started it.
    Abandon([u8; 16]),
}

/// One bounded worker job. Its serial guards against late replies after navigation.
#[derive(Clone, Debug)]
pub struct Work {
    /// UI generation; never an authentication identity.
    pub serial: u64,
    /// Ordered Central requests.
    pub requests: Vec<Request>,
    /// An existing attempt to reconnect to, without reserving again.
    pub resume: Option<Attempt>,
    /// Only an explicit Play opens a game.
    pub play: bool,
    append: bool,
}

/// Account-bound challenge state, discarded when its Central session changes.
#[expect(
    clippy::struct_excessive_bools,
    reason = "listing consent, deck folding and request freshness are independent UI properties"
)]
pub struct State {
    /// Latest catalogue page(s), bounded to 120 tasks.
    pub catalogue: Vec<Challenge>,
    /// Central's time, refreshed with every public read.
    pub now: i64,
    /// Next older page.
    pub before: Option<i64>,
    /// Selected identity, even while its detail is loading.
    pub selected: Option<[u8; 16]>,
    /// Public selected task.
    pub detail: Option<Challenge>,
    /// Selected ranking.
    pub board: Board,
    /// Current ranking page.
    pub offset: u32,
    /// Current page of standings.
    pub standings: Vec<api::Standing>,
    /// Own durable task progress.
    pub personal: Option<TaskProgress>,
    /// Permanent badge tracks.
    pub badges: Vec<Badge>,
    /// At most two Central-owned active attempts.
    pub active: Vec<Attempt>,
    /// Last attempt result, when read directly.
    pub last: Option<Attempt>,
    /// A hint Central has already marked as seen.
    pub hint: Option<Words>,
    /// A second explicit action is needed before a spoiler or abandonment.
    pub confirmation: Option<Confirmation>,
    /// New reservations default to private scores.
    pub listed: bool,
    /// Worker request in flight.
    pub busy: bool,
    /// Retry is offered without pretending the operation succeeded.
    pub failed: bool,
    /// Whether a first read has completed.
    pub loaded: bool,
    /// Whether the supplied deck text is expanded.
    pub show_deck: bool,
    serial: u64,
    begin: std::collections::BTreeMap<[u8; 16], [u8; 16]>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            catalogue: Vec::new(),
            now: 0,
            before: None,
            selected: None,
            detail: None,
            board: Board::FirstWin,
            offset: 0,
            standings: Vec::new(),
            personal: None,
            badges: Vec::new(),
            active: Vec::new(),
            last: None,
            hint: None,
            confirmation: None,
            listed: false,
            busy: false,
            failed: false,
            loaded: false,
            show_deck: false,
            serial: 0,
            begin: std::collections::BTreeMap::new(),
        }
    }
}

impl State {
    /// The selected task's resumable attempt.
    #[must_use]
    pub fn attempt(&self) -> Option<&Attempt> {
        self.active
            .iter()
            .find(|attempt| Some(attempt.challenge) == self.selected)
    }

    /// Prepare one action. The edge supplies a fresh `UUIDv7` only when Play needs one.
    pub fn act(&mut self, action: Action, request_id: [u8; 16]) -> Option<Work> {
        if self.busy {
            return None;
        }
        if self.confirmation.is_some()
            && !matches!(action, Action::Confirm | Action::Cancel | Action::Back)
        {
            return None;
        }
        let mut requests = Vec::new();
        let mut resume = None;
        let mut play = false;
        let mut append = false;
        match action {
            Action::Back => {
                self.select(None);
                self.failed = false;
                return None;
            }
            Action::Cancel => {
                self.confirmation = None;
                return None;
            }
            Action::Deck => {
                self.show_deck = !self.show_deck;
                return None;
            }
            Action::Hint => {
                self.selected?;
                let level = self.personal.as_ref()?.hint_level.saturating_add(1).min(2);
                self.confirmation = Some(Confirmation::Hint(level));
                return None;
            }
            Action::Abandon => {
                self.confirmation = Some(Confirmation::Abandon(self.attempt()?.id));
                return None;
            }
            Action::Confirm => match self.confirmation.take()? {
                Confirmation::Hint(level) => requests.push(Request::Hint {
                    challenge: self.selected?,
                    level,
                }),
                Confirmation::Abandon(attempt) => requests.push(Request::Abandon { attempt }),
            },
            Action::Play => {
                play = true;
                (resume, requests) = self.reservation(request_id)?;
            }
            Action::Visibility => requests.push(Request::Visibility {
                challenge: self.selected?,
                listed: !self.listed,
            }),
            Action::Select(id) => self.select(Some(id)),
            Action::Board(board) => {
                self.board = board;
                self.offset = 0;
                self.standings.clear();
            }
            Action::Page(forward) => {
                self.offset = if forward {
                    (self.offset + 30).min(3000)
                } else {
                    self.offset.saturating_sub(30)
                };
            }
            Action::Older => {
                let before = self.before?;
                append = true;
                requests.push(Request::Catalogue {
                    before: Some(before),
                });
            }
            Action::Refresh => {
                requests.push(Request::Catalogue { before: None });
            }
        }
        if !play {
            requests.push(Request::Progress);
            if let Some(challenge) = self.selected {
                requests.push(Request::Detail {
                    challenge,
                    board: self.board,
                    offset: self.offset,
                });
                requests.push(Request::Personal { challenge });
            }
        }
        self.serial = self.serial.wrapping_add(1);
        self.failed = false;
        self.busy = true;
        Some(Work {
            serial: self.serial,
            requests,
            resume,
            play,
            append,
        })
    }

    fn select(&mut self, id: Option<[u8; 16]>) {
        self.selected = id;
        self.detail = None;
        self.personal = None;
        self.hint = None;
        self.last = None;
        self.confirmation = None;
        self.board = Board::FirstWin;
        self.offset = 0;
        self.listed = false;
        self.show_deck = false;
        self.standings.clear();
    }

    fn reservation(&mut self, request_id: [u8; 16]) -> Option<(Option<Attempt>, Vec<Request>)> {
        let challenge = self.detail.as_ref()?;
        if challenge.withdrawn {
            return None;
        }
        if let Some(attempt) = self.attempt() {
            return Some((Some(attempt.clone()), Vec::new()));
        }
        if self.begin.len() >= 120 && !self.begin.contains_key(&challenge.id) {
            self.failed = true;
            return None;
        }
        // Keep uncertain reservations even when visiting a different task.
        let key = *self.begin.entry(challenge.id).or_insert(request_id);
        Some((
            None,
            vec![Request::Begin {
                challenge: challenge.id,
                request_id: key,
                listed: self.listed,
            }],
        ))
    }

    /// Apply an ordered reply batch. A failed call preserves all last confirmed data.
    /// Successes preceding a later failure are still applied (notably a disclosed hint).
    pub fn receive(&mut self, work: &Work, responses: Vec<Response>, failed: bool) {
        if work.serial != self.serial || !self.busy {
            return;
        }
        for response in responses {
            match response {
                Response::Catalogue {
                    now,
                    challenges,
                    before,
                } => {
                    self.now = now;
                    self.before = before;
                    self.loaded = true;
                    if !work.append {
                        self.catalogue.clear();
                    }
                    for task in challenges {
                        if !self.catalogue.iter().any(|old| old.id == task.id) {
                            self.catalogue.push(task);
                        }
                    }
                    if self.catalogue.len() > 120 {
                        self.catalogue.drain(..30);
                    }
                }
                Response::Detail {
                    now,
                    challenge,
                    standings,
                } if Some(challenge.id) == self.selected => {
                    self.now = now;
                    self.detail = Some(challenge);
                    self.standings = standings;
                }
                Response::Progress { badges, active } => {
                    self.badges = badges;
                    self.active = active;
                }
                Response::Personal { progress } if Some(progress.challenge) == self.selected => {
                    self.listed = progress.listed;
                    self.personal = Some(progress);
                }
                Response::Attempt { attempt } => {
                    self.active.retain(|old| old.challenge != attempt.challenge);
                    if attempt.state.active() {
                        self.active.push(attempt.clone());
                    }
                    self.begin.remove(&attempt.challenge);
                    self.last = Some(attempt);
                }
                Response::Hint { text } => {
                    self.hint = Some(text);
                }
                _ => {}
            }
        }
        self.busy = false;
        self.failed = failed;
    }
}

#[cfg(test)]
mod tests;
