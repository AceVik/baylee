//! Finite merchandise and explicit purchase destinations.
use super::{Refusal, Stock};
use crate::{
    cards::Catalog,
    v2::{
        catalog::{Definition, MYTHIC_PRICE, Potion, Rarity},
        economy::Recipe,
        inventory::Slot,
    },
};
use baylee_core::ids::CardIndex;
use serde::{Deserialize, Serialize};

/// Exact offered goods, with price resolved only from the frozen catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "item",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Goods {
    /// One owned nonbasic card; free basics never enter the shop.
    Card(CardIndex),
    /// One specific family and rarity.
    Relic(Definition),
    /// One table charge.
    Potion(Potion),
    /// One crafting potion; never usable at the table.
    Mythic,
}

/// A relic purchase always names its intended destination or complete fusion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "target",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Delivery {
    /// Cards, charges and crafting materials need no equipment slot.
    Inventory,
    /// Buy one relic into a free active or reserve slot.
    Slot(Slot),
    /// Buy one recipe input and fuse it with these owned inputs atomically.
    Fuse(Recipe),
}

impl Goods {
    /// Fixed authoritative price; callers never submit a price operand.
    /// # Errors
    /// Unknown/basic card or directly offered Mythic equipment.
    pub fn price(self, catalog: &Catalog) -> Result<u16, Refusal> {
        match self {
            Self::Card(id) => catalog
                .card(id)
                .filter(|card| !card.basic_land)
                .map(|card| u16::from(card.price))
                .ok_or(Refusal::Unavailable),
            Self::Relic(definition) => definition.rarity.price().ok_or(Refusal::Unavailable),
            Self::Potion(potion) => Ok(potion.price()),
            Self::Mythic => Ok(MYTHIC_PRICE),
        }
    }

    pub(super) const fn admitted(self, stage: u8) -> bool {
        match self {
            Self::Relic(Definition {
                rarity: Rarity::Uncommon,
                ..
            }) => stage >= 2,
            Self::Relic(Definition {
                rarity: Rarity::Rare,
                ..
            }) => stage >= 4,
            Self::Relic(Definition {
                rarity: Rarity::MythicRare,
                ..
            }) => false,
            Self::Mythic => stage >= 5,
            _ => true,
        }
    }

    pub(super) fn usable(self, stock: Stock<'_>) -> bool {
        let economy = stock.economy;
        if !self
            .price(stock.catalog)
            .is_ok_and(|price| price <= economy.gold)
        {
            return false;
        }
        match self {
            Self::Card(card) => stock.cards.bought(stock.catalog, card).is_ok(),
            Self::Potion(potion) => economy.bought_potion(potion).is_ok(),
            Self::Mythic => economy.bought_material().is_ok(),
            Self::Relic(definition) => {
                if (0..3)
                    .map(Slot::Active)
                    .chain((0..6).map(Slot::Reserve))
                    .any(|slot| economy.bought(definition, slot).is_ok())
                {
                    return true;
                }
                if definition.rarity == Rarity::Rare {
                    return economy.inventory.materials.iter().any(|material| {
                        (0..3)
                            .map(Slot::Active)
                            .chain((0..6).map(Slot::Reserve))
                            .any(|destination| {
                                economy
                                    .bought_and_fused(
                                        definition,
                                        &Recipe {
                                            relics: Vec::new(),
                                            mythic: Some(material.id),
                                            destination,
                                        },
                                    )
                                    .is_ok()
                            })
                    });
                }
                let matching: Vec<_> = economy
                    .inventory
                    .relics
                    .iter()
                    .filter(|item| item.definition == definition)
                    .collect();
                matching.iter().enumerate().any(|(index, first)| {
                    matching[index + 1..].iter().any(|second| {
                        [first.slot, second.slot].into_iter().any(|destination| {
                            economy
                                .bought_and_fused(
                                    definition,
                                    &Recipe {
                                        relics: vec![first.id, second.id],
                                        mythic: None,
                                        destination,
                                    },
                                )
                                .is_ok()
                        })
                    })
                })
            }
        }
    }
}
