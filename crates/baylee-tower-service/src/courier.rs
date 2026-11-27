//! Durable permanent-fact delivery; no game or replay state crosses this boundary.
use crate::{Error, identity::Central};
use baylee_tower::v2::run::Outcome as DuelOutcome;
use baylee_tower_store::{Award, AwardDelivery, Principal, Role, Store};
use central_api::{
    Endpoint, Token,
    tower_awards::{Delivery, Disposition, Fact, Outcome},
    wire::{Request, Response},
};
use std::{future::Future, time::Duration};

/// A sequential background courier with separate local and Central authority.
/// Independent instances coordinate through expiring database claims.
pub struct Courier {
    store: Store,
    principal: Principal,
    central: Central,
    credential: Token,
}

/// Value-free result; never includes account identifiers, facts or remote diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryStep {
    /// No due event was claimable.
    Idle,
    /// Central durably recorded the exact fact, or had already recorded it.
    Recorded,
    /// Central confirmed that the account is absent; it is never recreated.
    AccountGone,
    /// An unavailable or invalid remote response leaves the event for retry.
    Deferred,
}

/// Counters for the worker lifetime, with no identifying payloads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeliverySummary {
    /// Exact facts confirmed and acknowledged locally.
    pub recorded: u64,
    /// Confirmed absent-account facts acknowledged locally.
    pub account_gone: u64,
    /// Remote attempts retained for later retry.
    pub deferred: u64,
    /// Local storage/role failures; gameplay continues independently.
    pub unavailable: u64,
    /// Leaderboard names withdrawn because Central reported the account gone.
    pub withdrawn: u64,
}

/// When the first daily leaderboard name check runs after the courier starts.
const LISTING_START: Duration = Duration::from_secs(600);
/// How often every listed account is checked against Central.
const LISTING_PERIOD: Duration = Duration::from_secs(24 * 3600);

impl Courier {
    /// Configure only operator-owned credentials and the fixed Central endpoint.
    /// # Errors
    /// Wrong local role or invalid endpoint/origin.
    pub fn new(
        store: Store,
        principal: Principal,
        endpoint: &Endpoint,
        own_origin: &str,
        credential: Token,
    ) -> Result<Self, Error> {
        if principal.role() != Role::Courier {
            return Err(Error::Contract);
        }
        Ok(Self {
            store,
            principal,
            central: Central::new(endpoint, own_origin)?,
            credential,
        })
    }

    /// Deliver one immutable event. A crash, cancelled response or failed local ACK
    /// leaves the same event eligible again; Central's receipt deduplicates it.
    /// # Errors
    /// Cancellation, local role/storage failure or an invalid stored fact.
    pub async fn next(&mut self, stop: impl Future<Output = ()>) -> Result<DeliveryStep, Error> {
        tokio::pin!(stop);
        let claimed = tokio::select! {
            biased;
            () = &mut stop => return Err(Error::Cancelled),
            event = self.store.claim_award(&self.principal) => event?,
        };
        let Some(event) = claimed else {
            return Ok(DeliveryStep::Idle);
        };
        let delivery = envelope(event.lease.event, event.account, &event.award);
        let request = Request::TowerAward { delivery };
        let result = if delivery.validate().is_ok() {
            tokio::select! {
                biased;
                () = &mut stop => return Err(Error::Cancelled),
                response = self.central.exchange(&request, &self.credential) => response.ok(),
            }
        } else {
            None
        };
        if let Some(Response::TowerAward { receipt }) = result
            && receipt.event == delivery.event
            && receipt.account == delivery.account
            && receipt.run == delivery.fact.run
            && receipt.stage == delivery.fact.stage
        {
            // Finish the bounded local transaction even if stop arrives now.
            self.store
                .acknowledge_award(&self.principal, event.lease)
                .await?;
            return Ok(match receipt.disposition {
                Disposition::Recorded => DeliveryStep::Recorded,
                Disposition::AccountGone => {
                    // A deleted account's name leaves the leaderboard; its
                    // pseudonymous run data stays for the season (privacy § 7).
                    self.store.ranking_identity(event.account, None).await?;
                    DeliveryStep::AccountGone
                }
            });
        }
        self.store.defer_award(&self.principal, event.lease).await?;
        Ok(DeliveryStep::Deferred)
    }

    /// Ask Central about every account whose name stands on a Tower leaderboard,
    /// by repeating its newest delivered fact exactly (Central deduplicates it
    /// and changes nothing), and withdraw the name of each account Central no
    /// longer has. Run data stays, pseudonymous, until its season ends
    /// (Central privacy policy § 7). At most ten questions a second; an
    /// unanswered one is asked again the next day.
    /// # Errors
    /// Cancellation or local storage failure.
    pub async fn check_listings(&mut self, stop: impl Future<Output = ()>) -> Result<u64, Error> {
        tokio::pin!(stop);
        let mut withdrawn = 0_u64;
        let mut after = None;
        loop {
            let page = self.store.listed_receipts(&self.principal, after, 64).await?;
            let Some(last) = page.last() else {
                return Ok(withdrawn);
            };
            after = Some(last.account);
            for listed in &page {
                let delivery = envelope(listed.event, listed.account, &listed.award);
                if delivery.validate().is_err() {
                    continue;
                }
                let request = Request::TowerAward { delivery };
                let response = tokio::select! {
                    biased;
                    () = &mut stop => return Err(Error::Cancelled),
                    response = self.central.exchange(&request, &self.credential) => response.ok(),
                };
                if let Some(Response::TowerAward { receipt }) = response
                    && receipt.event == delivery.event
                    && receipt.account == delivery.account
                    && receipt.disposition == Disposition::AccountGone
                {
                    self.store.ranking_identity(listed.account, None).await?;
                    withdrawn += 1;
                }
                tokio::select! {
                    biased;
                    () = &mut stop => return Err(Error::Cancelled),
                    () = tokio::time::sleep(Duration::from_millis(100)) => {},
                }
            }
        }
    }

    /// Retry independently of gameplay. At most ten successful sends per second,
    /// one idle poll per second, and 5–60s pauses for local access/storage failures.
    /// Remote outages defer the event and pause five seconds before another send.
    pub async fn serve(&mut self, stop: impl Future<Output = ()>) -> DeliverySummary {
        tokio::pin!(stop);
        let mut summary = DeliverySummary::default();
        let mut backoff = 5_u64;
        // The first listing check waits for a settled start; then once a day.
        let mut listings = tokio::time::Instant::now() + LISTING_START;
        loop {
            if tokio::time::Instant::now() >= listings {
                match self.check_listings(&mut stop).await {
                    Err(Error::Cancelled) => return summary,
                    Ok(withdrawn) => summary.withdrawn = summary.withdrawn.saturating_add(withdrawn),
                    // A failed check is repeated tomorrow; delivery goes on.
                    Err(_) => eprintln!("Tower leaderboard name check unavailable; retrying tomorrow"),
                }
                listings = tokio::time::Instant::now() + LISTING_PERIOD;
            }
            let delay = match self.next(&mut stop).await {
                Ok(step) => {
                    backoff = 5;
                    match step {
                        DeliveryStep::Idle => Duration::from_secs(1),
                        DeliveryStep::Recorded => {
                            summary.recorded = summary.recorded.saturating_add(1);
                            Duration::from_millis(100)
                        }
                        DeliveryStep::AccountGone => {
                            summary.account_gone = summary.account_gone.saturating_add(1);
                            Duration::from_millis(100)
                        }
                        DeliveryStep::Deferred => {
                            summary.deferred = summary.deferred.saturating_add(1);
                            Duration::from_secs(5)
                        }
                    }
                }
                Err(Error::Cancelled) => return summary,
                Err(_) => {
                    summary.unavailable = summary.unavailable.saturating_add(1);
                    eprintln!("Tower accomplishment delivery unavailable; retrying");
                    let delay = Duration::from_secs(backoff);
                    backoff = (backoff * 2).min(60);
                    delay
                }
            };
            tokio::select! {
                biased;
                () = &mut stop => return summary,
                () = tokio::time::sleep(delay) => {},
            }
        }
    }
}

fn envelope(event: uuid::Uuid, account: uuid::Uuid, award: &Award) -> Delivery {
    Delivery {
        event: *event.as_bytes(),
        account: *account.as_bytes(),
        fact: Fact {
            schema: award.schema,
            run: *award.run.as_bytes(),
            cohort: *award.cohort.as_bytes(),
            stage: award.stage,
            outcome: match award.outcome {
                DuelOutcome::Victory => Outcome::Victory,
                DuelOutcome::Defeat => Outcome::Defeat,
                DuelOutcome::Draw => Outcome::Draw,
            },
            wins: award.wins,
            defeats: award.defeats,
        },
    }
}
