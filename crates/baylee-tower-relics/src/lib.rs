//! Original runic relic recipes. No engine, transport, callbacks or caller scripts.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

mod tuning;
pub use tuning::Level;

/// Event affinity. Order is append-only: it is part of the saved definition ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Sigil {
    /// Your upkeep begins.
    Dawn,
    /// Your end step begins.
    Dusk,
    /// Your land enters.
    Root,
    /// Your nontoken creature enters.
    Hearth,
    /// Your creature token enters.
    Bloom,
    /// Your creature dies.
    Ash,
    /// You cast a noncreature spell.
    Ember,
    /// You draw beyond the first draw-step card.
    Lore,
    /// You gain life.
    Pulse,
    /// Your artifact enters.
    Forge,
    /// You cast an enchantment.
    Veil,
    /// One or more of your creatures attack.
    Valor,
    /// Your creature deals combat damage to an opponent.
    Fang,
    /// You discard a card.
    Echo,
    /// An opponent's creature dies.
    Wraith,
    /// You cast or copy an instant or sorcery.
    Star,
}

impl Sigil {
    /// Frozen identity order.
    pub const ALL: [Self; 16] = [
        Self::Dawn,
        Self::Dusk,
        Self::Root,
        Self::Hearth,
        Self::Bloom,
        Self::Ash,
        Self::Ember,
        Self::Lore,
        Self::Pulse,
        Self::Forge,
        Self::Veil,
        Self::Valor,
        Self::Fang,
        Self::Echo,
        Self::Wraith,
        Self::Star,
    ];
    /// Original English title used by native content and its source generator.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Dawn => "Dawn",
            Self::Dusk => "Dusk",
            Self::Root => "Rootbound",
            Self::Hearth => "Hearth",
            Self::Bloom => "Bloom",
            Self::Ash => "Ashen",
            Self::Ember => "Ember",
            Self::Lore => "Lore",
            Self::Pulse => "Lifeblood",
            Self::Forge => "Forge",
            Self::Veil => "Veil",
            Self::Valor => "Valor",
            Self::Fang => "Fang",
            Self::Echo => "Echo",
            Self::Wraith => "Wraith",
            Self::Star => "Starfall",
        }
    }
    /// Original event sentence, without its reward clause.
    pub const fn event(self) -> &'static str {
        match self {
            Self::Dawn => "At the beginning of your upkeep",
            Self::Dusk => "At the beginning of your end step",
            Self::Root => "Whenever a land you control enters",
            Self::Hearth => "Whenever a nontoken creature you control enters",
            Self::Bloom => "Whenever a creature token you control enters",
            Self::Ash => "Whenever a creature you control dies",
            Self::Ember => "Whenever you cast a noncreature spell",
            Self::Lore => {
                "Whenever you draw a card except the first one in each of your draw steps"
            }
            Self::Pulse => "Whenever you gain life",
            Self::Forge => "Whenever an artifact you control enters",
            Self::Veil => "Whenever you cast an enchantment spell",
            Self::Valor => "Whenever one or more creatures you control attack",
            Self::Fang => "Whenever a creature you control deals combat damage to an opponent",
            Self::Echo => "Whenever you discard a card",
            Self::Wraith => "Whenever a creature an opponent controls dies",
            Self::Star => "Whenever you cast or copy an instant or sorcery spell",
        }
    }
}

/// Ten mechanically different uses of gathered charges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Vessel {
    /// Life gain.
    Chalice,
    /// Opponent life loss.
    Scepter,
    /// Scry.
    Prism,
    /// Plant creatures.
    Seed,
    /// Card advantage at a mana cost.
    Tome,
    /// White Soldier creatures at a mana cost.
    Horn,
    /// Mana of any colour.
    Flask,
    /// Toughness until end of turn.
    Bulwark,
    /// Power until end of turn.
    Banner,
    /// Draw, then discard.
    Lens,
}

impl Vessel {
    /// Frozen identity order.
    pub const ALL: [Self; 10] = [
        Self::Chalice,
        Self::Scepter,
        Self::Prism,
        Self::Seed,
        Self::Tome,
        Self::Horn,
        Self::Flask,
        Self::Bulwark,
        Self::Banner,
        Self::Lens,
    ];
    /// Original English noun.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Chalice => "Chalice",
            Self::Scepter => "Scepter",
            Self::Prism => "Prism",
            Self::Seed => "Seed",
            Self::Tome => "Tome",
            Self::Horn => "Horn",
            Self::Flask => "Flask",
            Self::Bulwark => "Bulwark",
            Self::Banner => "Banner",
            Self::Lens => "Lens",
        }
    }
}

/// A closed combination; arbitrary effects, prices and event names cannot be decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    /// What charges the item.
    pub sigil: Sigil,
    /// What its paid activation does.
    pub vessel: Vessel,
}

impl Recipe {
    /// Number of distinct event/action recipes, excluding rarity upgrades.
    pub const COUNT: usize = 160;
    /// Stable zero-based family ordinal within the runic expansion.
    pub const fn index(self) -> usize {
        self.sigil as usize * Vessel::ALL.len() + self.vessel as usize
    }
    /// Decode only the closed catalog.
    pub const fn from_index(index: usize) -> Option<Self> {
        if index >= Self::COUNT {
            return None;
        }
        Some(Self {
            sigil: Sigil::ALL[index / 10],
            vessel: Vessel::ALL[index % 10],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_single_mythic_relic_self_loops_for_zero_mana_or_snowballs_surplus_charges() {
        for index in 0..Recipe::COUNT {
            let recipe = Recipe::from_index(index).unwrap();
            for tier in 0..4 {
                let level = recipe.level(tier).unwrap();
                assert!((1..=7).contains(&level.charges));
                if level.activations != 0 {
                    continue;
                }
                // Count charges generated on `recipe.sigil` by one activation of `recipe.vessel`
                // plus its own Rare/Mythic `bonus` trigger when `tier >= 2`.
                let events: u16 = match (recipe.sigil, recipe.vessel) {
                    (Sigil::Bloom, Vessel::Seed | Vessel::Horn) => level.amount,
                    (Sigil::Pulse, Vessel::Chalice | Vessel::Tome | Vessel::Lens)
                    | (Sigil::Lore, Vessel::Tome | Vessel::Lens)
                    | (Sigil::Echo, Vessel::Lens) => 1,
                    _ => 0,
                };
                let refunded = events * level.gathered;
                if level.mana == 0 {
                    assert!(
                        refunded < level.charges,
                        "{recipe:?} tier {tier} refunds {refunded} >= {} for 0 mana",
                        level.charges
                    );
                } else {
                    assert!(
                        refunded <= level.charges,
                        "{recipe:?} tier {tier} snowballs {refunded} > {} charges",
                        level.charges
                    );
                }
            }
        }
    }
}
