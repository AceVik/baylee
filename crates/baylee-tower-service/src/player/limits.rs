//! Fixed, bounded admission counters; neither arbitrary bearers nor IPs allocate entries.
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use uuid::Uuid;

pub(super) struct Limits {
    global: Window,
    accounts: BTreeMap<Uuid, Window>,
    swept: Instant,
}

struct Window {
    since: Instant,
    used: u16,
}
impl Window {
    fn admit(&mut self, now: Instant, length: Duration, maximum: u16) -> bool {
        if now.saturating_duration_since(self.since) >= length {
            self.since = now;
            self.used = 0;
        }
        if self.used >= maximum {
            return false;
        }
        self.used += 1;
        true
    }
}

impl Limits {
    pub(super) fn new() -> Self {
        let now = Instant::now();
        Self {
            global: Window {
                since: now,
                used: 0,
            },
            accounts: BTreeMap::new(),
            swept: now,
        }
    }
    pub(super) fn global(&mut self, now: Instant) -> bool {
        self.global.admit(now, Duration::from_secs(1), 64)
    }
    pub(super) fn account(&mut self, account: Uuid, now: Instant) -> bool {
        let length = Duration::from_secs(60);
        if !self.accounts.contains_key(&account) && self.accounts.len() >= 4096 {
            if now.saturating_duration_since(self.swept) >= Duration::from_secs(1) {
                self.accounts
                    .retain(|_, window| now.saturating_duration_since(window.since) < length);
                self.swept = now;
            }
            if self.accounts.len() >= 4096 {
                return false;
            }
        }
        self.accounts
            .entry(account)
            .or_insert(Window {
                since: now,
                used: 0,
            })
            .admit(now, length, 120)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_refresh_does_not_reset_account_budget_or_grow_an_unbounded_map() {
        let mut limits = Limits::new();
        let now = Instant::now();
        let account = Uuid::now_v7();
        for _ in 0..120 {
            assert!(limits.account(account, now));
        }
        assert!(!limits.account(account, now));
        for _ in 1..4096 {
            assert!(limits.account(Uuid::now_v7(), now));
        }
        assert!(!limits.account(Uuid::now_v7(), now));
        assert!(limits.account(account, now + Duration::from_secs(60)));
        assert!(limits.account(Uuid::now_v7(), now + Duration::from_secs(61)));
        assert_eq!(limits.accounts.len(), 2);
        for _ in 0..64 {
            assert!(limits.global(now));
        }
        assert!(!limits.global(now));
        assert!(limits.global(now + Duration::from_secs(1)));
    }
}
