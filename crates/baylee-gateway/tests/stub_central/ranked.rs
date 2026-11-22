//! Minimal Central host peer for the real gateway + engine integration test.
use super::{Shared, Stub};
use central_api::ranked::{Format, HostAssignment, HostRequest, HostResponse, Outcome};
use uuid::Uuid;

pub struct Reservation {
    assignment: HostAssignment,
    claimed: [bool; 2],
    finished: Option<Outcome>,
    attempts: usize,
}

impl Stub {
    pub fn game_bearer(&self, port: u16, account: u8, id: Uuid) -> String {
        let mut claims = self.claims(port, account);
        claims.sid = super::sid_of(account + 64);
        claims.purpose = central_crypto::jwt::Purpose::Ranked;
        claims.ranked_match = Some(*id.as_bytes());
        let signed = self.shared.key.sign(&claims).unwrap();
        let body = serde_json::json!({"token":signed.as_str()}).to_string();
        assert_eq!(
            super::common::http(port, "POST", "/auth/central/login", None, &body).0,
            401
        );
        let (status, response) =
            super::common::http(port, "POST", "/auth/ranked/login", None, &body);
        assert_eq!(status, 200, "ranked login");
        let session: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert!(
            session["renew_in"]
                .as_i64()
                .is_some_and(|s| s > 0 && s <= 1800)
        );
        assert_eq!(
            super::common::http(port, "POST", "/auth/central/refresh", None, &body).0,
            401
        );
        super::common::json_field(&response, "token").to_owned()
    }
    pub fn ranked_capacity(&self) -> u64 {
        self.shared
            .ranked_capacity
            .load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn reserve_ranked(&self, keys: [u8; 2], decks: [[u8; 32]; 2]) -> Uuid {
        let accounts = self.shared.accounts.lock().unwrap();
        let id = Uuid::now_v7();
        *self.shared.ranked.lock().unwrap() = Some(Reservation {
            assignment: HostAssignment {
                match_id: *id.as_bytes(),
                format: Format::Standard,
                accounts: keys.map(|k| *accounts[&k].as_bytes()),
                decks,
                game: None,
            },
            claimed: [false; 2],
            finished: None,
            attempts: 0,
        });
        id
    }
    pub fn ranked_result(&self) -> Option<(Outcome, usize)> {
        let held = self.shared.ranked.lock().unwrap();
        let r = held.as_ref()?;
        Some((r.finished?, r.attempts))
    }
}

pub(super) fn reply(shared: &Shared, request: HostRequest) -> Option<HostResponse> {
    if let HostRequest::Ready { capacity } = request {
        shared
            .ranked_capacity
            .store(u64::from(capacity), std::sync::atomic::Ordering::Release);
        return Some(HostResponse::Ready);
    }
    let mut held = shared.ranked.lock().unwrap();
    let r = held.as_mut()?;
    match request {
        HostRequest::Claim {
            match_id,
            account,
            deck,
        } if match_id == r.assignment.match_id => {
            let seat = r.assignment.accounts.iter().position(|a| *a == account)?;
            if r.assignment.decks[seat] != deck {
                return None;
            }
            r.claimed[seat] = true;
            Some(HostResponse::Assignment {
                assignment: r.assignment.clone(),
            })
        }
        HostRequest::Start {
            match_id,
            game,
            pools,
        } if match_id == r.assignment.match_id && r.claimed == [true, true] => {
            if r.assignment.game.is_some_and(|old| old != game) {
                return None;
            }
            assert!(
                pools
                    .iter()
                    .all(|pool| pool.len() == 1 && pool[0].copies == 60)
            );
            r.assignment.game = Some(game);
            Some(HostResponse::Done)
        }
        HostRequest::Finish {
            match_id,
            game,
            outcome,
        } if match_id == r.assignment.match_id && Some(game) == r.assignment.game => {
            r.attempts += 1;
            if r.attempts == 1 {
                return None;
            }
            assert!(r.finished.is_none_or(|old| old == outcome));
            r.finished = Some(outcome);
            Some(HostResponse::Done)
        }
        _ => None,
    }
}
