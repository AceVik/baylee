//! Disposable presentation of confirmed equipment. No prices, stock or ownership originate here.
use super::domain;
use domain::v2::{
    catalog::{Definition, Rarity},
    economy::Recipe,
    inventory::{Inventory, Layout, Relic, Slot},
    run::Command,
};

/// A bounded rendered page, independent of how many items the run owns.
pub const PAGE_SIZE: usize = 18;

/// Stable inventory order, with the instance ordinal as its deterministic tie-breaker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Order {
    /// Most recently purchased or fused first.
    #[default]
    Newest,
    /// Localized or canonical display name, highest rarity, then most recent.
    Name,
    /// Highest resale refund (then paid basis), highest rarity, family, then most recent.
    Value,
    /// Family, highest rarity, then most recent.
    Family,
    /// Highest rarity, family, then most recent.
    Rarity,
}

impl Order {
    /// Canonical toolbar order for inventory sorting controls.
    pub const ALL: [Self; 5] = [
        Self::Newest,
        Self::Name,
        Self::Value,
        Self::Family,
        Self::Rarity,
    ];
}

/// Authoritative resale gold for an owned relic instance, derived from its conserved basis.
#[must_use]
pub fn relic_refund(item: &Relic) -> u16 {
    (item.basis / 2).min(item.definition.rarity.sale_cap())
}

/// Borrowed entries sorted without a localized name callback (`Order::Name` falls back to family ID).
#[must_use]
pub fn inventory(inventory: &Inventory, order: Order) -> Vec<&Relic> {
    inventory_by(inventory, order, |definition| (definition.id() - 1) / 4)
}

/// Borrowed entries sorted deterministically, using `name_of` when `order == Order::Name`.
/// The underlying run inventory is never reordered or copied.
#[must_use]
pub fn inventory_by<K: Ord>(
    inventory: &Inventory,
    order: Order,
    mut name_of: impl FnMut(Definition) -> K,
) -> Vec<&Relic> {
    let mut rows: Vec<_> = inventory
        .relics
        .iter()
        .filter(|item| item.slot == Slot::Inventory)
        .collect();
    if order == Order::Name {
        let mut keyed: Vec<_> = rows
            .into_iter()
            .map(|item| {
                let name = name_of(item.definition);
                let tier = 4 - item.definition.rarity.tier();
                let family = (item.definition.id() - 1) / 4;
                (name, tier, family, std::cmp::Reverse(item.id), item)
            })
            .collect();
        keyed.sort_unstable_by(|a, b| {
            (a.0.cmp(&b.0)).then_with(|| (a.1, a.2, a.3).cmp(&(b.1, b.2, b.3)))
        });
        return keyed.into_iter().map(|entry| entry.4).collect();
    }
    rows.sort_unstable_by_key(|item| {
        let family = (item.definition.id() - 1) / 4;
        let tier = u16::from(4 - item.definition.rarity.tier());
        let (first, second, third, fourth) = match order {
            Order::Newest | Order::Name => (0, 0, 0, 0),
            Order::Value => (
                u16::MAX - relic_refund(item),
                u16::MAX - item.basis,
                tier,
                family,
            ),
            Order::Family => (family, tier, 0, 0),
            Order::Rarity => (tier, family, 0, 0),
        };
        (first, second, third, fourth, std::cmp::Reverse(item.id))
    });
    rows
}

/// Clamp a stale page after a sale or automatic fusion; an empty bag has one page.
#[must_use]
pub fn page(total: usize, requested: usize) -> (usize, usize) {
    let pages = total.div_ceil(PAGE_SIZE).max(1);
    (requested.min(pages - 1), pages)
}

/// Right-click toggles equipment and chooses only a genuinely free position.
/// A full loadout refuses; explicit drag/drop can swap the two chosen items instead.
#[must_use]
pub fn quick_move(inventory: &Inventory, id: u32) -> Option<Command> {
    if inventory.layout != Layout::Run {
        return None;
    }
    let item = inventory.relics.iter().find(|item| item.id == id)?;
    let slot = match item.slot {
        Slot::Active(_) => Slot::Inventory,
        Slot::Inventory => Slot::Active((0..inventory.layout.active_slots()).find(|slot| {
            !inventory
                .relics
                .iter()
                .any(|item| item.slot == Slot::Active(*slot))
        })?),
        Slot::Reserve(_) => return None,
    };
    Some(Command::Move { item: id, slot })
}

/// Explicit Rare + owned potion crafting, retaining the selected item's equipment position.
#[must_use]
pub fn mythic(inventory: &Inventory, id: u32, material: u32) -> Option<Command> {
    let item = inventory
        .relics
        .iter()
        .find(|item| item.id == id && item.definition.rarity == Rarity::Rare)?;
    inventory
        .materials
        .iter()
        .find(|potion| potion.id == material)?;
    Some(Command::Fuse {
        recipe: Recipe {
            relics: vec![id],
            mythic: Some(material),
            destination: item.slot,
        },
    })
}

/// Shelf affinity includes equipped and stored copies and predicts cascading upgrades.
#[must_use]
pub fn affinity(inventory: &Inventory, offered: Definition) -> (usize, Option<Rarity>) {
    let count = |definition| {
        inventory
            .relics
            .iter()
            .filter(|item| item.definition == definition)
            .count()
    };
    let owned = count(offered);
    let upgrade = if owned >= 2 {
        match offered.rarity {
            Rarity::Common
                if count(Definition {
                    rarity: Rarity::Uncommon,
                    ..offered
                }) >= 2 =>
            {
                Some(Rarity::Rare)
            }
            Rarity::Common => Some(Rarity::Uncommon),
            Rarity::Uncommon => Some(Rarity::Rare),
            Rarity::Rare | Rarity::MythicRare => None,
        }
    } else {
        None
    };
    (owned, upgrade)
}

#[cfg(test)]
mod tests;
