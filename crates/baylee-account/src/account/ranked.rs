//! Central chooses the host and both seats. A pending gateway admission can
//! only be reused for the same account, destination, assignment and queue entry.
use super::{Account, Error};
use crate::{
    Admission, CentralHttp, Endpoint, GatewayHttp, RankedDeck, RankedSeat,
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
    admission: Admission,
}

/// A seated 1v1 whose gateway response was confirmed by a second Central read.
pub struct RankedPlay {
    /// Fresh Central assignment for the exact account and deck.
    pub assignment: Assignment,
    /// Private game seat at that gateway.
    pub seat: RankedSeat,
}

impl Account {
    /// Work on this account's queue entry, or query the authoritative status.
    ///
    /// # Errors
    /// Refused requests, unavailable Central or a stale account session.
    pub fn ranked(&self, request: Request) -> Result<Response, Error> {
        self.session.ranked(&self.http, request)
    }

    /// Submit a frozen snapshot using an account-scoped gateway admission.
    /// `None` means the host has accepted the deck and is waiting for its peer.
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
        if !pending.as_ref().is_some_and(|host| {
            host.endpoint == *self.http.endpoint()
                && host.key == key
                && same_match(&selected, &host.assignment)
        }) {
            *pending = None;
            let (gateway, ticket) = self.gateway_token(&selected.origin)?;
            let admission = gateway.admit(&ticket).map_err(|e| gateway_error(e.error))?;
            if admission.terms_stale == Some(true) {
                return Err(Error::Rejected);
            }
            *pending = Some(RankedHost {
                endpoint: self.http.endpoint().clone(),
                key,
                assignment: selected.clone(),
                gateway,
                admission,
            });
        }
        let host = pending.as_ref().ok_or(Error::Conflict)?;
        // The live game already holds the frozen decks. Account-bound seat
        // recovery works on another device even if the shelf has since changed.
        let reply = if let Some((_, deck)) = snapshot
            .filter(|(digest, deck)| selected.deck == *digest && selected.format == deck.format)
        {
            host.gateway
                .ranked_play(&host.admission.token, selected.match_id, deck)
        } else if let Some(game) = selected.game {
            host.gateway
                .ranked_reclaim(&host.admission.token, game)
                .map(Some)
        } else {
            return Err(Error::Conflict);
        };
        let seat = match reply {
            Ok(Some(seat)) => seat,
            Ok(None) => return Ok(None),
            Err(error) => {
                if matches!(error, Error::Rejected | Error::TermsRequired) {
                    *pending = None;
                }
                return Err(gateway_error(error));
            }
        };
        let after = self.ranked_assignment(key)?;
        if !confirmed(&selected, &after, &seat) {
            *pending = None;
            return Err(Error::Conflict);
        }
        *pending = None;
        Ok(Some(RankedPlay {
            assignment: after,
            seat,
        }))
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
}
