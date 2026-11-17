//! Cosmetic transitions between confirmed revisions, never predicted mutations.
use super::domain::v2::{
    economy::Economy,
    inventory::Relic,
    shop::{Goods, Offer},
};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Stable render anchors, independent of disposable UI entity identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Anchor {
    /// A retained offer slot, including its empty sold state.
    Offer(u32),
    /// A specific equipped or stored instance.
    Item(u32),
    /// Fallback for items outside the visible inventory page.
    Bag,
    /// The shared supply tray.
    Supplies,
    /// The coin balance.
    Wallet,
    /// Origin for round income; destination for a paid reroll.
    Shelf,
}

/// One icon moving between confirmed positions; fusion travels more slowly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flight {
    /// Icon displayed along the path.
    pub goods: Goods,
    /// Previous position.
    pub from: Anchor,
    /// New position.
    pub to: Anchor,
    /// Converging ingredients, followed by the upgraded item's glow.
    pub fusion: bool,
}

/// Bounded visual response. The wallet always displays the server's actual value.
#[derive(Debug, Default)]
pub struct Change {
    /// At most sixteen icon flights, even after a bulk state correction.
    pub flights: Vec<Flight>,
    /// Signed authoritative wallet change.
    pub gold: i32,
    /// The purchased offer, if any.
    pub purchase: Option<u32>,
    /// The sold relic instance, if any.
    pub sold: Option<u32>,
    /// Upgraded instance and its finished appearance.
    pub fused: Option<(u32, Goods)>,
}

/// Retains only equipment, stock and wallet data, never deck lists or credentials.
#[derive(Default)]
pub struct Trace(Option<Snapshot>);

/// Remembers the latest visibly presented reward, independently of socket
/// revision gaps during a duel. Cosmetic only; never credits the wallet.
#[derive(Default)]
pub struct Income(Option<(Uuid, usize)>);

impl Income {
    /// The first presentation of a pending reward may celebrate its settled
    /// income. Navigating away, repeated replies and stale rounds cannot replay it.
    #[must_use]
    pub fn observe(&mut self, id: Uuid, rounds: usize, pending: bool, gold: u16) -> Option<u16> {
        if self
            .0
            .is_some_and(|(old, seen)| old == id && seen >= rounds)
        {
            return None;
        }
        self.0 = Some((id, rounds));
        (pending && rounds > 0).then_some(gold)
    }
}

struct Snapshot {
    id: Uuid,
    revision: u64,
    gold: u16,
    relics: Vec<Relic>,
    offers: Vec<Offer>,
}

impl Trace {
    /// An unchanged/stale reply does no allocation and cannot replay an animation.
    /// First loads, run switches and missed revisions establish a new baseline.
    #[must_use]
    pub fn observe(
        &mut self,
        id: Uuid,
        revision: u64,
        economy: &Economy,
        offers: &[Offer],
    ) -> Option<Change> {
        if self
            .0
            .as_ref()
            .is_some_and(|old| old.id == id && revision <= old.revision)
        {
            return None;
        }
        let changes = self
            .0
            .as_ref()
            .filter(|old| old.id == id && old.revision.checked_add(1) == Some(revision))
            .map(|old| changes(old, economy, offers));
        self.0 = Some(Snapshot {
            id,
            revision,
            gold: economy.gold,
            relics: economy.inventory.relics.clone(),
            offers: offers.to_vec(),
        });
        changes
    }
}

fn changes(old: &Snapshot, economy: &Economy, offers: &[Offer]) -> Change {
    let relics = &economy.inventory.relics;
    let purchased = offers.iter().find(|offer| {
        offer.sold
            && old
                .offers
                .iter()
                .any(|prior| prior.id == offer.id && prior.goods == offer.goods && !prior.sold)
    });
    let mut result = Change {
        gold: i32::from(economy.gold) - i32::from(old.gold),
        purchase: purchased.map(|offer| offer.id),
        ..Change::default()
    };
    // Validated inventories have ascending, never-reused instance IDs. Index only
    // removed families once, avoiding a quadratic scan when a large bag changes.
    let mut removed: BTreeMap<u16, Vec<&Relic>> = BTreeMap::new();
    for item in &old.relics {
        if relics
            .binary_search_by_key(&item.id, |item| item.id)
            .is_err()
        {
            removed
                .entry((item.definition.id() - 1) / 4)
                .or_default()
                .push(item);
        }
    }
    for item in relics {
        if result.flights.len() >= 16 {
            break;
        }
        let to = Anchor::Item(item.id);
        let goods = Goods::Relic(item.definition);
        if let Ok(index) = old.relics.binary_search_by_key(&item.id, |item| item.id) {
            if old.relics[index].slot != item.slot {
                result.flights.push(Flight {
                    goods,
                    from: to,
                    to,
                    fusion: false,
                });
            }
            continue;
        }
        let ingredients = removed
            .remove(&((item.definition.id() - 1) / 4))
            .unwrap_or_default();
        let fusion = ingredients
            .iter()
            .any(|prior| prior.definition.rarity.tier() < item.definition.rarity.tier());
        if fusion {
            result.fused = Some((item.id, goods));
            for prior in ingredients.into_iter().take(16 - result.flights.len()) {
                result.flights.push(Flight {
                    goods: Goods::Relic(prior.definition),
                    from: Anchor::Item(prior.id),
                    to,
                    fusion: true,
                });
            }
        }
        if let Some(offer) = purchased.filter(|offer| matches!(offer.goods, Goods::Relic(definition) if definition.family == item.definition.family))
            && result.flights.len() < 16 {
            result.flights.push(Flight { goods: offer.goods, from: Anchor::Offer(offer.id), to, fusion });
        }
    }
    if let Some(offer) = purchased.filter(|offer| !matches!(offer.goods, Goods::Relic(_)))
        && result.flights.len() < 16
    {
        result.flights.push(Flight {
            goods: offer.goods,
            from: Anchor::Offer(offer.id),
            to: Anchor::Supplies,
            fusion: false,
        });
    }
    if result.gold > 0 && result.purchase.is_none() && result.fused.is_none() {
        result.sold = removed
            .values()
            .flat_map(|items| items.iter())
            .next()
            .map(|item| item.id);
    }
    result
}

#[cfg(test)]
mod tests;
