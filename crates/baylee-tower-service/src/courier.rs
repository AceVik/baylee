//! Durable permanent-fact delivery; no game or replay state crosses this boundary.
use crate::{Error, identity::Central};
use baylee_tower::v2::run::Outcome as DuelOutcome;
use baylee_tower_store::{AwardDelivery, Principal, Role, Store};
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
}

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
        let delivery = envelope(&event);
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
                Disposition::AccountGone => DeliveryStep::AccountGone,
            });
        }
        self.store.defer_award(&self.principal, event.lease).await?;
        Ok(DeliveryStep::Deferred)
    }

    /// Retry independently of gameplay. At most ten successful sends per second,
    /// one idle poll per second, and 5–60s pauses for local access/storage failures.
    /// Remote outages defer the event and pause five seconds before another send.
    pub async fn serve(&mut self, stop: impl Future<Output = ()>) -> DeliverySummary {
        tokio::pin!(stop);
        let mut summary = DeliverySummary::default();
        let mut backoff = 5_u64;
        loop {
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

fn envelope(event: &AwardDelivery) -> Delivery {
    Delivery {
        event: *event.lease.event.as_bytes(),
        account: *event.account.as_bytes(),
        fact: Fact {
            schema: event.award.schema,
            run: *event.award.run.as_bytes(),
            cohort: *event.award.cohort.as_bytes(),
            stage: event.award.stage,
            outcome: match event.award.outcome {
                DuelOutcome::Victory => Outcome::Victory,
                DuelOutcome::Defeat => Outcome::Defeat,
                DuelOutcome::Draw => Outcome::Draw,
            },
            wins: event.award.wins,
            defeats: event.award.defeats,
        },
    }
}
