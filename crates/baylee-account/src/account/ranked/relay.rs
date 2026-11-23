use super::{
    Account, Assignment, Error, Fence, Id, RankedDeck, RankedHost, RankedPlay, RankedSeat,
    SeatRequest, SeatResponse, confirmed, same_match,
};
use crate::ranked::replica::{
    ConsentState, Phase, SeatStatus, SetupBytes,
    relay::{Call, HttpBytes, Reply, Secret},
};
use std::time::{Duration, Instant};

fn same_route(expected: &SeatStatus, actual: &SeatStatus) -> bool {
    expected.fence == actual.fence
        && expected.host_id == actual.host_id
        && expected.origin == actual.origin
        && expected.game.is_none_or(|game| actual.game == Some(game))
}

fn check_assignment(a: &Assignment, status: &SeatStatus, owner: Id) -> Result<(), Error> {
    if a.account != owner
        || a.seat > 1
        || a.match_id != status.fence.match_id
        || a.origin != status.origin
        || a.host_id != status.host_id
        || status.game.is_some_and(|g| a.game != Some(g))
    {
        return Err(Error::Conflict);
    }
    Ok(())
}

fn allowed(status: &SeatStatus) -> Result<bool, Error> {
    match status.phase {
        Phase::Paused | Phase::Disputed => return Ok(false),
        Phase::Finished | Phase::Void => return Err(Error::Conflict),
        Phase::Single | Phase::Preparing | Phase::Playing => {}
    }
    match status.consent {
        ConsentState::Ready => Ok(true),
        ConsentState::Waiting => Ok(false),
        ConsentState::Needed => Err(Error::TermsRequired),
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Admission {
    token: Secret,
    renew_in: u64,
}

impl Account {
    fn call_relay(&self, fence: Fence, call: Call) -> Result<Reply, Error> {
        let SeatResponse::Relayed { reply } = self.replica(SeatRequest::Relay { fence, call })?
        else {
            return Err(Error::Rejected);
        };
        Ok(reply)
    }

    fn relay_http(&self, fence: Fence, call: Call) -> Result<Option<HttpBytes>, Error> {
        match self.call_relay(fence, call)? {
            Reply::Http {
                status: 200,
                body: Some(body),
            } => Ok(Some(body)),
            Reply::Http {
                status: 202,
                body: None,
            } => Ok(None),
            Reply::Http { status, .. } => {
                Err(crate::http::status_error(status).unwrap_or(Error::Rejected))
            }
            Reply::Socket { .. } => Err(Error::Rejected),
        }
    }

    fn relay_admit(&self, fence: Fence) -> Result<(Secret, Instant), Error> {
        let bytes = self
            .relay_http(fence, Call::Admit {})?
            .ok_or(Error::Rejected)?;
        let a: Admission = central_api::decode(bytes.as_slice())?;
        if !a.token.is_capability() || !(1..=1800).contains(&a.renew_in) {
            return Err(Error::Rejected);
        }
        Ok((a.token, Instant::now() + Duration::from_secs(a.renew_in)))
    }

    /// Obtain a game seat and one-use Central socket ticket through the relay.
    /// Pending polls reuse only this account, exact operator and generation.
    ///
    /// # Errors
    /// Refuses stale leadership, missing consent and an unexpected seat or game.
    pub fn play_ranked_replica(
        &self,
        key: Id,
        expected: &SeatStatus,
        snapshot: Option<([u8; 32], &RankedDeck)>,
        pending: &mut Option<RankedHost>,
    ) -> Result<Option<RankedPlay>, Error> {
        let status = self.replica_status(expected.fence.match_id)?;
        if !same_route(expected, &status) {
            *pending = None;
            return Err(Error::Conflict);
        }
        if !allowed(&status)? {
            return Ok(None);
        }
        let owner = self.session.identity().account;
        let a = self.ranked_assignment(key)?;
        check_assignment(&a, &status, owner)?;
        let reused = pending.as_ref().is_some_and(|p| {
            p.fence == status.fence
                && p.endpoint == *self.http.endpoint()
                && p.key == key
                && same_match(&a, &p.assignment)
                && Instant::now() < p.renew_at
        });
        if !reused {
            *pending = None;
            let (bearer, renew_at) = self.relay_admit(status.fence)?;
            *pending = Some(RankedHost {
                fence: status.fence,
                endpoint: self.http.endpoint().clone(),
                key,
                assignment: a.clone(),
                bearer,
                consent: None,
                renew_at,
            });
        }
        let host = pending.as_ref().ok_or(Error::Conflict)?;
        let bearer = host.bearer.clone();
        let call = if let Some(game) = a.game {
            Call::Reclaim { bearer, game }
        } else {
            let (_, deck) = snapshot
                .filter(|(digest, d)| a.deck == *digest && a.format == d.format)
                .ok_or(Error::Conflict)?;
            Call::Play {
                bearer,
                deck: SetupBytes::new(central_api::encode(deck)?)?,
            }
        };
        let bytes = match self.relay_http(status.fence, call) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(None),
            Err(e) => {
                *pending = None;
                return Err(if reused && e == Error::Rejected {
                    Error::Conflict
                } else {
                    e
                });
            }
        };
        let mut seat: RankedSeat = central_api::decode(bytes.as_slice())?;
        seat.validate()?;
        let after = self.ranked_assignment(key)?;
        let final_status = self.replica_status(status.fence.match_id)?;
        check_assignment(&after, &final_status, owner)?;
        if !same_route(&status, &final_status)
            || !confirmed(&a, &after, &seat)
            || !allowed(&final_status)?
        {
            *pending = None;
            return Err(Error::Conflict);
        }
        let renew_at = host.renew_at;
        *pending = None;
        let game = *seat.game_id.as_bytes();
        let Reply::Socket { ticket } = self.call_relay(
            status.fence,
            Call::SocketTicket {
                game,
                seat_token: Secret::new(std::mem::take(&mut *seat.seat_token))?,
            },
        )?
        else {
            return Err(Error::Rejected);
        };
        seat.seat_token = zeroize::Zeroizing::new(ticket.expose().into());
        Ok(Some(RankedPlay {
            fence: status.fence,
            relay_origin: self.http.endpoint().origin().into(),
            assignment: after,
            seat,
            renew_at,
        }))
    }

    /// Extend a game session through the relay while it remains the same authority.
    ///
    /// # Errors
    /// Refuses superseded generations, unapproved operators or any different assignment.
    pub fn renew_ranked_replica(
        &self,
        key: Id,
        game: &Assignment,
        fence: Fence,
    ) -> Result<Instant, Error> {
        let status = self.replica_status(fence.match_id)?;
        let actual = self.ranked_assignment(key)?;
        check_assignment(&actual, &status, self.session.identity().account)?;
        if &actual != game || game.game.is_none() || status.fence != fence || !allowed(&status)? {
            return Err(Error::Conflict);
        }
        self.relay_admit(fence).map(|(_, renew)| renew)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranked::{Format, replica::Head};

    fn status() -> SeatStatus {
        SeatStatus {
            fence: Fence {
                match_id: [1; 16],
                generation: 2,
            },
            host_id: [2; 16],
            origin: "https://ranked.example".into(),
            game: Some([3; 16]),
            head: Head {
                position: 1,
                chain: [4; 32],
            },
            phase: Phase::Playing,
            clock: None,
            consent: ConsentState::Ready,
        }
    }

    #[test]
    fn relayed_seat_requires_same_leader_term_after_restart_even_at_the_same_origin() {
        let first = status();
        let later = SeatStatus {
            fence: Fence {
                generation: first.fence.generation + 2,
                ..first.fence
            },
            ..first.clone()
        };
        assert!(!same_route(&first, &later));
        let changed = SeatStatus {
            host_id: [7; 16],
            ..first.clone()
        };
        assert!(!same_route(&first, &changed));
        let without_game = SeatStatus {
            game: None,
            ..first.clone()
        };
        assert!(same_route(&without_game, &first));
        assert!(!same_route(&first, &without_game));
    }

    #[test]
    fn private_upload_waits_for_both_operator_permissions_and_a_playable_phase() {
        let state = status();
        assert_eq!(allowed(&state), Ok(true));
        for phase in [Phase::Paused, Phase::Disputed] {
            assert_eq!(
                allowed(&SeatStatus {
                    phase,
                    ..state.clone()
                }),
                Ok(false)
            );
        }
        for phase in [Phase::Finished, Phase::Void] {
            assert_eq!(
                allowed(&SeatStatus {
                    phase,
                    ..state.clone()
                }),
                Err(Error::Conflict)
            );
        }
        assert_eq!(
            allowed(&SeatStatus {
                consent: ConsentState::Needed,
                ..state.clone()
            }),
            Err(Error::TermsRequired)
        );
        assert_eq!(
            allowed(&SeatStatus {
                consent: ConsentState::Waiting,
                ..state.clone()
            }),
            Ok(false)
        );
        let a = Assignment {
            match_id: state.fence.match_id,
            host_id: state.host_id,
            origin: state.origin.clone(),
            format: Format::Commander,
            account: [5; 16],
            seat: 0,
            deck: [0; 32],
            game: state.game,
        };
        assert!(check_assignment(&a, &state, a.account).is_ok());
        assert!(check_assignment(&a, &state, [9; 16]).is_err());
        assert!(
            check_assignment(
                &a,
                &SeatStatus {
                    game: Some([8; 16]),
                    ..state
                },
                a.account
            )
            .is_err()
        );
    }
}
