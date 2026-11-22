//! Central chooses the host and both seats. A pending gateway admission can
//! only be reused for the same account, destination, assignment and queue entry.
use super::{Account, Error};
use crate::{
    CentralHttp, Endpoint, GatewayHttp, RankedDeck, RankedSeat,
    ranked::{Assignment, Id, QueueState, Request, Response},
};

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
    gateway: GatewayHttp,
    bearer: zeroize::Zeroizing<String>,
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
    /// Fresh Central assignment for the exact account and deck.
    pub assignment: Assignment,
    /// Private game seat at that gateway.
    pub seat: RankedSeat,
    /// Monotonic instant to refresh the host's live session during a long duel.
    pub renew_at: std::time::Instant,
}

impl Account {
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

    /// Submit a frozen snapshot using an account-scoped gateway admission.
    /// `None` means a pending host needs consent or is waiting for its peer.
    /// When [`RankedHost::consent`] is present, show its document. Drop the
    /// pending admission after successful acceptance to acquire fresh standing.
    /// Keep `pending` and the same snapshot when polling again.
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
        let own = self.session.identity().account;
        let selected = self.ranked_assignment(key)?;
        if selected.account != own
            || selected.seat > 1
            || selected.game.is_none()
                && snapshot.is_none_or(|(digest, deck)| {
                    selected.format != deck.format || selected.deck != digest
                })
        {
            return Err(Error::Conflict);
        }
        let reused = pending.as_ref().is_some_and(|host| {
            host.endpoint == *self.http.endpoint()
                && host.key == key
                && same_match(&selected, &host.assignment)
        });
        if !reused {
            *pending = None;
            let (gateway, ticket) =
                self.gateway_token_for(&selected.origin, crate::wire::GatewayPurpose::Ranked)?;
            let (bearer, stale, renew_in) = gateway.ranked_admit(&ticket).map_err(gateway_error)?;
            let consent = if stale {
                let document = central_client::gateway_terms(&self.http, &selected.origin)?
                    .ok_or(Error::Conflict)?;
                if !same_match(&selected, &self.ranked_assignment(key)?) {
                    return Err(Error::Conflict);
                }
                Some(RankedConsent {
                    key,
                    assignment: selected.clone(),
                    document,
                })
            } else {
                None
            };
            *pending = Some(RankedHost {
                endpoint: self.http.endpoint().clone(),
                key,
                assignment: selected.clone(),
                gateway,
                bearer,
                consent,
                renew_at: std::time::Instant::now() + renew_in,
            });
        }
        let host = pending.as_ref().ok_or(Error::Conflict)?;
        if host.consent.is_some() {
            return Ok(None);
        }
        // The live game already holds the frozen decks. Account-bound seat
        // recovery works on another device even if the shelf has since changed.
        let reply = if let Some(game) = selected.game {
            host.gateway
                .ranked_reclaim(&host.bearer, selected.match_id, game)
        } else if let Some((_, deck)) = snapshot
            .filter(|(digest, deck)| selected.deck == *digest && selected.format == deck.format)
        {
            host.gateway
                .ranked_play(&host.bearer, selected.match_id, deck)
        } else {
            return Err(Error::Conflict);
        };
        let seat = match reply {
            Ok(Some(seat)) => seat,
            Ok(None) => return Ok(None),
            Err(error) => {
                if matches!(error, Error::Rejected | Error::TermsRequired) {
                    *pending = None;
                    // The short HTTP bearer or the host's standing may have
                    // changed while waiting for the other player. Refresh
                    // once. A fresh admission's refusal retains its meaning.
                    if reused {
                        return Err(Error::Conflict);
                    }
                }
                return Err(gateway_error(error));
            }
        };
        let after = self.ranked_assignment(key)?;
        if !confirmed(&selected, &after, &seat) {
            *pending = None;
            return Err(Error::Conflict);
        }
        let renew_at = host.renew_at;
        *pending = None;
        Ok(Some(RankedPlay {
            assignment: after,
            seat,
            renew_at,
        }))
    }

    /// Extend the live game's ranked SID without rotating its seat or opening a lobby.
    ///
    /// # Errors
    /// Refuses to renew a changed assignment, closed game or changed standing.
    pub fn renew_ranked_game(
        &self,
        key: Id,
        game: &Assignment,
    ) -> Result<std::time::Instant, Error> {
        let current = self.ranked_assignment(key)?;
        if game.game.is_none()
            || current != *game
            || current.account != self.session.identity().account
        {
            return Err(Error::Conflict);
        }
        let (gateway, ticket) =
            self.gateway_token_for(&current.origin, crate::wire::GatewayPurpose::Ranked)?;
        let (_, stale, renew_in) = gateway.ranked_admit(&ticket).map_err(gateway_error)?;
        if stale {
            return Err(Error::Rejected);
        }
        Ok(std::time::Instant::now() + renew_in)
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

fn gateway_error(error: Error) -> Error {
    match error {
        Error::TermsRequired => Error::Rejected,
        other => other,
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
