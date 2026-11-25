//! What stands in front of the token routes now that the intake is public
//! for good (owner, 25.11.2026: the route is needed and stays open, #408;
//! it is guarded instead). `docs/feedback.md` §"Intake" is normative.
//!
//! - [`Failures`]: a wrong credential on a token route is counted against
//!   the address it came from (an IPv6 address by its /64), and an address
//!   with [`FAILURES_PER_ADDRESS`] of them in [`FAILURE_WINDOW`] is refused
//!   before its next credential is even looked at. A process on this
//!   machine that comes without a proxy's `X-Forwarded-For` (the official
//!   gateway over loopback) is neither counted nor refused, so nobody on
//!   the internet can lock it out by sending wrong tokens.
//! - [`Budget`]: what one gateway's token may file in [`BUDGET_WINDOW`]:
//!   a number of reports and a number of bytes, both the operator's
//!   (`FEEDBACK_INTAKE_PER_HOUR`, `FEEDBACK_INTAKE_MIB_PER_HOUR`). A token
//!   that leaks can no longer fill the disk at the speed of its line.
//! - [`Slots`]: how many intake bodies are read and stored at once, in all
//!   and for one gateway, since each may hold about three times its 88 MiB
//!   in memory while it is decoded and stored.
//! - [`INTAKE_READ_TIME`]: the time a body has to arrive in, so a token
//!   cannot hold its slots with a body it never finishes.
//!
//! Memory only: a restart forgets every count, and no address is ever
//! written down or logged. A lock a panic poisoned is taken as it is: the
//! counts are only pushed, popped and summed, so whatever a panic left
//! behind is still a count, and a guard never takes the service down.

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, Ipv6Addr};
use std::sync::{Arc, Mutex, PoisonError};

use axum::http::HeaderMap;
use time::{Duration, OffsetDateTime};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// How long a wrong credential is held against its address.
pub const FAILURE_WINDOW: Duration = Duration::minutes(15);

/// Wrong credentials an address may send in [`FAILURE_WINDOW`] before its
/// requests to the token routes are refused unread. An operator who mistypes
/// a token a few times is not shut out; guessing a token of 16 characters or
/// more at 80 an hour from one address is hopeless.
pub const FAILURES_PER_ADDRESS: usize = 20;

/// The window an intake budget counts in.
pub const BUDGET_WINDOW: Duration = Duration::HOUR;

/// Reports one gateway's token may file an hour unless the operator says
/// otherwise (`FEEDBACK_INTAKE_PER_HOUR`). The language-model collector files
/// at most 25 reports a game, so this is room for 80 such games an hour.
pub const DEFAULT_PER_HOUR: usize = 2000;

/// Mebibytes of intake bodies one gateway's token may send an hour unless
/// the operator says otherwise (`FEEDBACK_INTAKE_MIB_PER_HOUR`).
pub const DEFAULT_MIB_PER_HOUR: u64 = 1024;

/// Intake bodies read and stored at once, from every gateway together.
pub const SLOTS: usize = 4;

/// Intake bodies read and stored at once for one gateway: so one token,
/// leaked or busy, never holds every slot.
pub const SLOTS_PER_GATEWAY: usize = 2;

/// How long a report waits for a slot before it is refused as busy. A
/// gateway gives the whole request 20 seconds (`FORWARD_TIMEOUT`), so the
/// wait leaves it most of them.
pub const SLOT_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// The time an intake body has to arrive in, once it has its slot. A gateway
/// on this machine sends one within its 20 seconds; this leaves a remote one
/// room for a large record over a slow line, and no more.
pub const INTAKE_READ_TIME: std::time::Duration = std::time::Duration::from_secs(120);

/// Whether a request comes from a process on this machine and not through
/// the proxy: a loopback peer that sends no `X-Forwarded-For`. The proxy in
/// front of the service always sets that header, so a request from the
/// internet never passes for local, however the service is configured.
#[must_use]
pub fn is_local(peer: Option<IpAddr>, headers: &HeaderMap) -> bool {
    peer.is_some_and(|p| p.is_loopback()) && !headers.contains_key("x-forwarded-for")
}

/// The key an address's failures are counted under: an IPv4 address (an
/// IPv4 address mapped into IPv6 too) as it is, an IPv6 address by its /64,
/// since one host usually has the whole /64; anything that is no address
/// (`unknown`, a peer the service could not see) as it is.
#[must_use]
pub fn counted_as(address: &str) -> String {
    let Ok(ip) = address.parse::<IpAddr>() else {
        return address.to_owned();
    };
    match ip.to_canonical() {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            let s = v6.segments();
            format!("{}/64", Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0))
        }
    }
}

/// Wrong credentials on the token routes in the last [`FAILURE_WINDOW`],
/// per address ([`counted_as`]).
#[derive(Default)]
pub struct Failures {
    by_address: HashMap<String, VecDeque<OffsetDateTime>>,
}

impl Failures {
    /// The most addresses it holds before it sweeps out every stale one.
    const SWEEP_AT: usize = 4096;

    /// Whether `address` has sent too many wrong credentials to be heard
    /// at `now`.
    pub fn shut(&mut self, address: &str, now: OffsetDateTime) -> bool {
        let Some(times) = self.by_address.get_mut(&counted_as(address)) else {
            return false;
        };
        while times.front().is_some_and(|t| now - *t >= FAILURE_WINDOW) {
            times.pop_front();
        }
        times.len() >= FAILURES_PER_ADDRESS
    }

    /// `address` sent a wrong credential at `now`.
    pub fn fail(&mut self, address: &str, now: OffsetDateTime) {
        if self.by_address.len() >= Self::SWEEP_AT {
            self.by_address
                .retain(|_, times| times.back().is_some_and(|t| now - *t < FAILURE_WINDOW));
        }
        let times = self.by_address.entry(counted_as(address)).or_default();
        // Never more than the bound is kept: a flood of wrong tokens from
        // one address costs one short queue, not one entry per request.
        if times.len() >= FAILURES_PER_ADDRESS {
            times.pop_front();
        }
        times.push_back(now);
    }
}

/// What each gateway's token filed in the last [`BUDGET_WINDOW`]: one entry
/// per report it was let in for, with the bytes its body had.
pub struct Budget {
    per_hour: usize,
    bytes_per_hour: u64,
    by_gateway: HashMap<String, VecDeque<(OffsetDateTime, u64)>>,
}

impl Default for Budget {
    fn default() -> Self {
        Self::new(DEFAULT_PER_HOUR, DEFAULT_MIB_PER_HOUR)
    }
}

impl Budget {
    /// `per_hour` reports and `mib_per_hour` MiB of bodies a gateway an hour.
    #[must_use]
    pub fn new(per_hour: usize, mib_per_hour: u64) -> Self {
        Self {
            per_hour,
            bytes_per_hour: mib_per_hour.saturating_mul(1024 * 1024),
            by_gateway: HashMap::new(),
        }
    }

    /// Whether `gateway` may send another report at `now`; counted if so.
    /// Its bytes are added once they are read ([`Budget::spent`]), so a
    /// gateway at the edge may go over by the bodies it has in its slots,
    /// and no further.
    pub fn take(&mut self, gateway: &str, now: OffsetDateTime) -> bool {
        // Gateways are the configured names, a handful: nothing to sweep.
        let sent = self.by_gateway.entry(gateway.to_owned()).or_default();
        while sent.front().is_some_and(|(t, _)| now - *t >= BUDGET_WINDOW) {
            sent.pop_front();
        }
        let bytes: u64 = sent.iter().map(|(_, b)| b).sum();
        if sent.len() >= self.per_hour || bytes >= self.bytes_per_hour {
            return false;
        }
        sent.push_back((now, 0));
        true
    }

    /// `gateway`'s latest report brought `bytes`.
    pub fn spent(&mut self, gateway: &str, bytes: u64) {
        if let Some((_, b)) = self
            .by_gateway
            .get_mut(gateway)
            .and_then(VecDeque::back_mut)
        {
            *b = b.saturating_add(bytes);
        }
    }
}

/// The intake's slots: [`SLOTS`] in all, [`SLOTS_PER_GATEWAY`] for one
/// gateway.
pub struct Slots {
    all: Arc<Semaphore>,
    per_gateway: Mutex<HashMap<String, Arc<Semaphore>>>,
}

impl Default for Slots {
    fn default() -> Self {
        Self {
            all: Arc::new(Semaphore::new(SLOTS)),
            per_gateway: Mutex::new(HashMap::new()),
        }
    }
}

/// A slot, held while one intake body is read and stored; dropping it gives
/// it back, however the request ends.
pub struct Slot {
    _gateway: OwnedSemaphorePermit,
    _all: OwnedSemaphorePermit,
}

impl Slots {
    /// A slot for `gateway`, waiting at most `wait` in all; `None` when
    /// none came free in that time.
    pub async fn take(&self, gateway: &str, wait: std::time::Duration) -> Option<Slot> {
        let mine = self
            .per_gateway
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(gateway.to_owned())
            .or_insert_with(|| Arc::new(Semaphore::new(SLOTS_PER_GATEWAY)))
            .clone();
        let deadline = tokio::time::Instant::now() + wait;
        // The gateway's own first: a gateway that holds its two waits on
        // itself and takes none of the others' slots meanwhile.
        let gateway = tokio::time::timeout_at(deadline, mine.acquire_owned())
            .await
            .ok()?
            .ok()?;
        let all = tokio::time::timeout_at(deadline, self.all.clone().acquire_owned())
            .await
            .ok()?
            .ok()?;
        Some(Slot {
            _gateway: gateway,
            _all: all,
        })
    }
}

/// The guards together, as the service holds them.
#[derive(Default)]
pub struct Guard {
    failures: Mutex<Failures>,
    budget: Mutex<Budget>,
    slots: Slots,
}

impl Guard {
    /// The guards with an intake budget of `per_hour` reports and
    /// `mib_per_hour` MiB a gateway.
    #[must_use]
    pub fn with_budget(per_hour: usize, mib_per_hour: u64) -> Self {
        Self {
            budget: Mutex::new(Budget::new(per_hour, mib_per_hour)),
            ..Self::default()
        }
    }

    /// See [`Failures::shut`].
    pub fn shut(&self, address: &str, now: OffsetDateTime) -> bool {
        self.failures
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .shut(address, now)
    }

    /// See [`Failures::fail`].
    pub fn fail(&self, address: &str, now: OffsetDateTime) {
        self.failures
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .fail(address, now);
    }

    /// See [`Budget::take`].
    pub fn take(&self, gateway: &str, now: OffsetDateTime) -> bool {
        self.budget
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take(gateway, now)
    }

    /// See [`Budget::spent`].
    pub fn spent(&self, gateway: &str, bytes: u64) {
        self.budget
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .spent(gateway, bytes);
    }

    /// See [`Slots::take`], waiting at most [`SLOT_WAIT`].
    pub async fn slot(&self, gateway: &str) -> Option<Slot> {
        self.slots.take(gateway, SLOT_WAIT).await
    }
}

/// A whole number of at least 1 from the environment variable `name`, or
/// `default` when it is unset or empty.
///
/// # Errors
///
/// When it is set to anything else: a budget mistyped would quietly be no
/// budget, so the service refuses to start instead.
pub fn positive_from_env<T: std::str::FromStr + PartialOrd + From<u8>>(
    name: &str,
    default: T,
) -> anyhow::Result<T> {
    positive(name, std::env::var(name).ok().as_deref(), default)
}

/// [`positive_from_env`] on the value `raw` the variable `name` has.
fn positive<T: std::str::FromStr + PartialOrd + From<u8>>(
    name: &str,
    raw: Option<&str>,
    default: T,
) -> anyhow::Result<T> {
    match raw.map(str::trim).filter(|v| !v.is_empty()) {
        None => Ok(default),
        Some(raw) => match raw.parse::<T>() {
            Ok(n) if n >= T::from(1) => Ok(n),
            _ => anyhow::bail!("{name}: `{raw}` is not a whole number of at least 1"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn at(minutes: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::days(20_000) + Duration::minutes(minutes)
    }

    #[test]
    fn only_a_loopback_peer_without_a_proxy_header_is_local() {
        let loopback: Option<IpAddr> = Some("127.0.0.1".parse().unwrap());
        let mut proxied = HeaderMap::new();
        proxied.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.7"));
        assert!(is_local(loopback, &HeaderMap::new()));
        assert!(is_local(Some("::1".parse().unwrap()), &HeaderMap::new()));
        assert!(!is_local(loopback, &proxied), "through the proxy");
        assert!(!is_local(
            Some("203.0.113.7".parse().unwrap()),
            &HeaderMap::new()
        ));
        assert!(!is_local(None, &HeaderMap::new()), "a peer nobody saw");
    }

    #[test]
    fn an_ipv6_address_is_counted_by_its_64() {
        assert_eq!(counted_as("2001:db8:1:2:aaaa::1"), "2001:db8:1:2::/64");
        assert_eq!(
            counted_as("2001:db8:1:2:ffff:ffff:ffff:ffff"),
            counted_as("2001:db8:1:2::9")
        );
        assert_ne!(counted_as("2001:db8:1:2::1"), counted_as("2001:db8:1:3::1"));
        assert_eq!(counted_as("203.0.113.7"), "203.0.113.7");
        assert_eq!(counted_as("::ffff:203.0.113.7"), "203.0.113.7");
        assert_eq!(counted_as("unknown"), "unknown");
    }

    #[test]
    fn an_address_is_shut_after_its_failures_until_they_age() {
        let mut failures = Failures::default();
        for _ in 0..FAILURES_PER_ADDRESS - 1 {
            failures.fail("203.0.113.7", at(0));
        }
        assert!(!failures.shut("203.0.113.7", at(1)));
        failures.fail("203.0.113.7", at(1));
        assert!(failures.shut("203.0.113.7", at(1)));
        assert!(!failures.shut("203.0.113.8", at(1)), "only that address");
        // the first nineteen age out after the window; the last still counts
        assert!(!failures.shut("203.0.113.7", at(15)));
        // the same /64 shares its count
        for _ in 0..FAILURES_PER_ADDRESS {
            failures.fail("2001:db8::1", at(20));
        }
        assert!(failures.shut("2001:db8::ffff", at(20)));
    }

    #[test]
    fn a_flood_of_failures_keeps_one_short_queue() {
        let mut failures = Failures::default();
        for _ in 0..10_000 {
            failures.fail("203.0.113.7", at(0));
        }
        assert_eq!(
            failures.by_address["203.0.113.7"].len(),
            FAILURES_PER_ADDRESS
        );
        assert!(failures.shut("203.0.113.7", at(0)));
    }

    #[test]
    fn a_gateway_s_budget_counts_reports_and_bytes_per_hour() {
        let mut budget = Budget::new(3, 1);
        assert!(budget.take("collector", at(0)));
        assert!(budget.take("collector", at(1)));
        assert!(budget.take("collector", at(2)));
        assert!(!budget.take("collector", at(3)), "three an hour");
        assert!(budget.take("sanctuary", at(3)), "each gateway its own");
        // an hour after the first, one is free again
        assert!(budget.take("collector", at(60)));
        // a mebibyte of bodies is all, however few the reports
        let mut budget = Budget::new(100, 1);
        assert!(budget.take("collector", at(0)));
        budget.spent("collector", 1024 * 1024);
        assert!(!budget.take("collector", at(1)), "the bytes are spent");
        assert!(budget.take("collector", at(60)));
    }

    #[tokio::test]
    async fn slots_are_bounded_in_all_and_per_gateway_and_come_back() {
        let slots = Slots::default();
        let wait = std::time::Duration::from_millis(20);
        let a1 = slots.take("a", wait).await.expect("a's first");
        let a2 = slots.take("a", wait).await.expect("a's second");
        assert!(slots.take("a", wait).await.is_none(), "two for one gateway");
        let b1 = slots.take("b", wait).await.expect("b's first");
        let b2 = slots.take("b", wait).await.expect("b's second");
        assert!(slots.take("c", wait).await.is_none(), "four in all");
        drop(a1);
        let c1 = slots.take("c", wait).await.expect("a slot given back");
        drop((a2, b1, b2, c1));
        assert_eq!(slots.all.available_permits(), SLOTS);
    }

    #[test]
    fn a_budget_from_the_environment_is_a_whole_number_of_at_least_one() {
        let name = "FEEDBACK_INTAKE_PER_HOUR";
        assert_eq!(positive(name, None, 7_usize).unwrap(), 7);
        assert_eq!(
            positive(name, Some(" "), 7_usize).unwrap(),
            7,
            "empty is unset"
        );
        for (raw, fine) in [
            ("12", Some(12_usize)),
            (" 12 ", Some(12)),
            ("0", None),
            ("-1", None),
            ("1.5", None),
            ("x", None),
        ] {
            assert_eq!(positive(name, Some(raw), 7_usize).ok(), fine, "{raw}");
        }
        let refused = positive(name, Some("0"), 7_usize).unwrap_err().to_string();
        assert!(
            refused.starts_with("FEEDBACK_INTAKE_PER_HOUR: `0`"),
            "{refused}"
        );
    }
}
