//! Central chooses the host and both seats. A pending gateway admission can
//! only be reused for the same account, destination, assignment and queue entry.
use super::{Account, Error};
use crate::ranked::replica::{self, Fence, SeatRequest, SeatResponse};
mod documents;
mod relay;
use crate::{
    CentralHttp, Endpoint, RankedDeck, RankedSeat,
    ranked::{Assignment, Id, QueueState, Request, Response},
};
pub use documents::{RankedHostDocument, RankedHostDocuments};

/// Read a public Ranked leaderboard, season or published profile without logging in.
///
/// # Errors
/// Malformed/unauthorized requests and unavailable Central fail closed.
pub fn ranked_read(endpoint: &Endpoint, request: Request) -> Result<Response, Error> {
    central_client::ranked::read(&CentralHttp::new(endpoint.clone()), request)
}

/// Pending gateway-local admission. No credential can be formatted or
/// forwarded to a caller-supplied destination.
pub struct RankedHost {
    endpoint: Endpoint,
    key: Id,
    assignment: Assignment,
    fence: Fence,
    bearer: replica::relay::Secret,
    consent: Option<RankedConsent>,
    renew_at: std::time::Instant,
}

/// A specific unclaimed ranked seat's published gateway terms. The document
/// and destination come from Central and acceptance uses the same account.
#[derive(Clone)]
pub struct RankedConsent {
    key: Id,
    assignment: Assignment,
    document: crate::terms::TermsDocument,
}

impl RankedHost {
    /// This host requires a decision before the frozen deck may be submitted.
    #[must_use]
    pub fn consent(&self) -> Option<&RankedConsent> {
        self.consent.as_ref()
    }
}

impl RankedConsent {
    /// Central-verified translations and digest of the document shown.
    #[must_use]
    pub fn document(&self) -> &crate::terms::TermsDocument {
        &self.document
    }

    /// The verified match host whose terms would be accepted.
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.assignment.origin
    }

    /// Whether the same authenticated queue entry still needs this decision.
    #[must_use]
    pub fn matches(&self, state: &QueueState) -> bool {
        matches!(state, QueueState::Assigned{key, assignment} if *key == self.key && same_match(assignment, &self.assignment))
    }

    /// Accept the displayed version after a deliberate player decision.
    ///
    /// # Errors
    /// A changed assignment, account, document, or unavailable authority.
    pub fn accept(&self, account: &Account) -> Result<(), Error> {
        let selected = account.ranked_assignment(self.key)?;
        if selected.account != account.session.identity().account
            || !same_match(&selected, &self.assignment)
        {
            return Err(Error::Conflict);
        }
        account.session.accept_terms(
            &account.http,
            crate::terms::TermsSubject::Gateway {
                origin: selected.origin,
            },
            &self.document.version,
        )
    }
}

/// A seated 1v1 whose gateway response was confirmed by a second Central read.
pub struct RankedPlay {
    /// Trusted Central origin of the socket relay.
    pub relay_origin: String,
    /// Leadership observed while acquiring the one-use ticket.
    pub fence: Fence,
    /// Fresh Central assignment for the exact account and deck.
    pub assignment: Assignment,
    /// Private game seat at that gateway.
    pub seat: RankedSeat,
    /// Monotonic instant to refresh the host's live session during a long duel.
    pub renew_at: std::time::Instant,
}

impl Account {
    /// Execute an authenticated ranked seat operation against this account's Central.
    ///
    /// # Errors
    /// Refuses a mismatched authority, invalid request, or expired session.
    pub fn replica(&self, request: SeatRequest) -> Result<SeatResponse, Error> {
        self.session.replica(&self.http, request)
    }

    /// Read the private Elo projection for the deck pool the owner selected.
    ///
    /// # Errors
    /// Invalid pools, refusal by Central or an unavailable account session.
    pub fn preview_ranked_deck(
        &self,
        format: crate::ranked::Format,
        pool: Vec<crate::ranked::PoolEntry>,
    ) -> Result<crate::ranked::DeckPreview, Error> {
        self.session.preview_ranked_deck(&self.http, format, pool)
    }

    /// Work on this account's queue entry, or query the authoritative status.
    ///
    /// # Errors
    /// Refused requests, unavailable Central or a stale account session.
    pub fn ranked(&self, request: Request) -> Result<Response, Error> {
        self.session.ranked(&self.http, request)
    }

    /// Submit a frozen snapshot through Central's authenticated relay.
    /// `None` means play is waiting on a peer, consent or recovery. On
    /// `TermsRequired`, show [`Account::ranked_host_documents`] and obtain
    /// the participant's explicit choice before retrying. Reuse the frozen
    /// snapshot and pending admission while this assignment remains current.
    ///
    /// # Errors
    /// Refuses a replaced queue, changed assignment, stale admission, foreign
    /// seat or unavailable authority. Only Central can choose the destination.
    pub fn play_ranked(
        &self,
        key: Id,
        snapshot: Option<([u8; 32], &RankedDeck)>,
        pending: &mut Option<RankedHost>,
    ) -> Result<Option<RankedPlay>, Error> {
        let id = self.ranked_assignment(key)?.match_id;
        let status = self.replica_status(id)?;
        self.play_ranked_replica(key, &status, snapshot, pending)
    }

    /// Refresh a single-host ranked session through Central's authenticated relay.
    ///
    /// # Errors
    /// Refuses a changed assignment, protected game or closed game.
    pub fn renew_ranked_game(
        &self,
        key: Id,
        game: &Assignment,
    ) -> Result<std::time::Instant, Error> {
        self.renew_ranked_replica(
            key,
            game,
            Fence {
                match_id: game.match_id,
                generation: 0,
            },
        )
    }

    fn replica_status(&self, match_id: Id) -> Result<replica::SeatStatus, Error> {
        let SeatResponse::Status { status } = self.replica(SeatRequest::Status { match_id })?
        else {
            return Err(Error::Rejected);
        };
        Ok(status)
    }

    fn ranked_assignment(&self, key: Id) -> Result<Assignment, Error> {
        match self.ranked(Request::Status)? {
            Response::Queue {
                state:
                    QueueState::Assigned {
                        key: current,
                        assignment,
                    },
            } if current == key => Ok(assignment),
            _ => Err(Error::Conflict),
        }
    }
}

fn same_match(left: &Assignment, right: &Assignment) -> bool {
    left.account == right.account
        && left.match_id == right.match_id
        && left.host_id == right.host_id
        && left.origin == right.origin
        && left.format == right.format
        && left.seat == right.seat
        && left.deck == right.deck
}

fn confirmed(before: &Assignment, after: &Assignment, seat: &RankedSeat) -> bool {
    same_match(before, after)
        && before.game.is_none_or(|id| id == *seat.game_id.as_bytes())
        && after.game == Some(*seat.game_id.as_bytes())
        && u32::from(after.seat) == seat.seat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranked::Format;
    use uuid::Uuid;

    fn id() -> Id {
        *Uuid::now_v7().as_bytes()
    }

    #[test]
    fn handoff_rechecks_every_assignment_binding_and_both_game_observations() {
        let before = Assignment {
            match_id: id(),
            host_id: id(),
            origin: "https://verified.invalid".into(),
            format: Format::Standard,
            account: id(),
            seat: 1,
            deck: [9; 32],
            game: None,
            opponent_ai: None,
        };
        let seat = RankedSeat {
            game_id: Uuid::now_v7(),
            seat: 1,
            seat_token: zeroize::Zeroizing::new("not-a-live-credential".into()),
        };
        let after = Assignment {
            game: Some(*seat.game_id.as_bytes()),
            ..before.clone()
        };
        assert!(confirmed(&before, &after, &seat));
        for changed in [
            Assignment {
                match_id: id(),
                ..after.clone()
            },
            Assignment {
                host_id: id(),
                ..after.clone()
            },
            Assignment {
                origin: "https://second.invalid".into(),
                ..after.clone()
            },
            Assignment {
                format: Format::Highlander,
                ..after.clone()
            },
            Assignment {
                account: id(),
                ..after.clone()
            },
            Assignment {
                seat: 0,
                ..after.clone()
            },
            Assignment {
                deck: [12; 32],
                ..after.clone()
            },
            Assignment {
                game: Some(id()),
                ..after.clone()
            },
            Assignment {
                game: None,
                ..after.clone()
            },
        ] {
            assert!(!confirmed(&before, &changed, &seat));
        }
        let other_initial_game = Assignment {
            game: Some(id()),
            ..before
        };
        assert!(!confirmed(&other_initial_game, &after, &seat));
        let other_chair = RankedSeat { seat: 0, ..seat };
        assert!(!confirmed(&after, &after, &other_chair));
    }

    #[test]
    fn public_transport_rejects_mutations_before_opening_any_connection() {
        let endpoint = Endpoint::development("http://127.0.0.1:1").unwrap();
        let request = Request::Join {
            key: id(),
            format: Format::Standard,
            deck: [1; 32],
        };
        assert_eq!(ranked_read(&endpoint, request), Err(Error::Rejected));
    }

    #[test]
    fn gateway_terms_stay_bound_to_one_queue_entry_host_and_account() {
        use crate::terms::{TermsDocument, TermsVersion};
        let key = id();
        let assignment = Assignment {
            match_id: id(),
            host_id: id(),
            origin: "https://playing.invalid".into(),
            format: Format::Commander,
            account: id(),
            seat: 0,
            deck: [7; 32],
            game: None,
            opponent_ai: None,
        };
        let consent = RankedConsent {
            key,
            assignment: assignment.clone(),
            document: TermsDocument {
                version: TermsVersion {
                    version: "v1".into(),
                    digest: [0; 32],
                },
                texts: Vec::new(),
            },
        };
        let matches = |key, a| consent.matches(&QueueState::Assigned { key, assignment: a });
        assert!(matches(key, assignment.clone()));
        assert!(!matches(id(), assignment.clone()));
        assert!(!matches(
            key,
            Assignment {
                origin: "https://home.invalid".into(),
                ..assignment.clone()
            }
        ));
        assert!(!matches(
            key,
            Assignment {
                account: id(),
                ..assignment.clone()
            }
        ));
        assert!(!matches(
            key,
            Assignment {
                match_id: id(),
                ..assignment
            }
        ));
        assert!(!consent.matches(&QueueState::Idle));
    }
}
