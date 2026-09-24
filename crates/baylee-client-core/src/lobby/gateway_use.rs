//! How often each saved gateway has been used here, and the order the
//! gateway list is drawn in because of it.
//!
//! Kept in the client's own settings file and never sent anywhere: it is a
//! fact about this device, and nobody else's business. A use is a sign-in
//! that worked (and, once guests can play, a guest's entry). Asking an
//! address about itself, choosing it and pointing at it are not uses: the
//! list is ordered by where the player actually went, not where they looked.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One gateway's uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct GatewayUse {
    /// How many sign-ins worked there.
    #[serde(default)]
    pub count: u32,
    /// When the last one was, as a place in the order of all uses on this
    /// device (see [`GatewayUses::record`]), not as a time: two sign-ins only
    /// need to say which came later, and a count cannot be wrong about that
    /// the way a clock can.
    #[serde(default)]
    pub last: u64,
}

/// Every saved gateway's uses, by address.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct GatewayUses(pub BTreeMap<String, GatewayUse>);

impl GatewayUses {
    /// Counts one use of an address.
    pub fn record(&mut self, address: &str) {
        let next = self.0.values().map(|u| u.last).max().unwrap_or(0) + 1;
        let entry = self.0.entry(address.to_string()).or_default();
        entry.count = entry.count.saturating_add(1);
        entry.last = next;
    }

    /// Forgets an address, when it leaves the list.
    pub fn forget(&mut self, address: &str) {
        self.0.remove(address);
    }

    /// One address's uses; none for an address never used.
    #[must_use]
    pub fn of(&self, address: &str) -> GatewayUse {
        self.0.get(address).copied().unwrap_or_default()
    }

    /// Puts addresses in the order the list draws them: the most used first,
    /// then the most recently used, then by address, so that the order is
    /// total and two launches draw the same list.
    pub fn order(&self, addresses: &mut [String]) {
        addresses.sort_by(|a, b| {
            let (ua, ub) = (self.of(a), self.of(b));
            ub.count
                .cmp(&ua.count)
                .then(ub.last.cmp(&ua.last))
                .then_with(|| a.cmp(b))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listed(uses: &GatewayUses, addresses: &[&str]) -> Vec<String> {
        let mut list: Vec<String> = addresses.iter().map(ToString::to_string).collect();
        uses.order(&mut list);
        list
    }

    #[test]
    fn the_most_used_comes_first_then_the_latest_then_the_address() {
        let mut uses = GatewayUses::default();
        for address in [
            "https://b",
            "https://c",
            "https://b",
            "https://a",
            "https://c",
        ] {
            uses.record(address);
        }
        // b and c twice each, c more recently; a once; d and e never.
        assert_eq!(
            listed(
                &uses,
                &[
                    "https://e",
                    "https://a",
                    "https://b",
                    "https://d",
                    "https://c"
                ]
            ),
            [
                "https://c",
                "https://b",
                "https://a",
                "https://d",
                "https://e"
            ]
        );
    }

    #[test]
    fn a_settings_file_from_before_counts_every_gateway_as_unused() {
        let uses: GatewayUses = serde_json::from_str("{}").expect("an empty map");
        assert_eq!(uses.of("https://anything"), GatewayUse::default());
        let partial: GatewayUses =
            serde_json::from_str(r#"{"https://a":{"count":3}}"#).expect("a use without a last");
        assert_eq!(partial.of("https://a"), GatewayUse { count: 3, last: 0 });
        assert_eq!(
            listed(&partial, &["https://b", "https://a"]),
            ["https://a", "https://b"]
        );
    }

    #[test]
    fn a_forgotten_gateway_starts_again_from_nothing() {
        let mut uses = GatewayUses::default();
        uses.record("https://a");
        uses.record("https://a");
        uses.forget("https://a");
        assert_eq!(uses.of("https://a"), GatewayUse::default());
        uses.record("https://a");
        assert_eq!(uses.of("https://a").count, 1);
    }
}
