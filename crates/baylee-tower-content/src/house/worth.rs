//! What a relic is worth to one deck, in rough points of one opposing life.
//!
//! This is the house shopper's taste, not a balance claim: the ladder's
//! strength is measured afterwards in native duels (`baylee-tower-lab`), and
//! a taste that buys badly shows up there as a weak stage, not as a number
//! here. The event rates come from the deck's own composition; the relic
//! levels from the native definitions (`baylee_tower_relics::Level`).
use baylee_cards::deckgen::profile::{
    CardProfile, MASK_INTERACTION, ROLE_BOARD_WIPE, ROLE_CANTRIP, ROLE_CARD_DRAW, ROLE_GY_FILLER,
    ROLE_LIFEGAIN, ROLE_TOKENS,
};
use baylee_core::types::TypeSet;
use baylee_tower::{
    cards::Stack,
    v2::catalog::{Definition, Family, Rarity, RunicRecipe, Sigil, Vessel},
};

/// A duel's own turns, as the lab measures them (about 17 turns in all).
const TURNS: f64 = 8.5;
/// The share of the library a duel draws: seven plus a card a turn, of 39.
const SEEN: f64 = (7.0 + TURNS) / 39.0;

/// What one deck does in a typical duel, counted from its cards.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Deck {
    creatures: f64,
    tokens: f64,
    noncreature: f64,
    spells: f64,
    artifacts: f64,
    enchantments: f64,
    lifegain: f64,
    draws: f64,
    discards: f64,
    removal: f64,
    lands: f64,
}

impl Deck {
    /// Count the library (commander excluded; it is cast about once anyway).
    pub(super) fn of(deck: &[Stack]) -> Self {
        let mut counts = Self::default();
        for row in deck {
            let Some(def) = baylee_cards::by_index(row.card) else {
                continue;
            };
            let n = f64::from(row.copies);
            let types = def.faces[0].types;
            if types.contains(TypeSet::LAND) {
                counts.lands += n;
                continue;
            }
            let profile = CardProfile::from_def(def);
            let role = |mask: u32| profile.roles & mask != 0;
            if types.contains(TypeSet::CREATURE) {
                counts.creatures += n;
            } else {
                counts.noncreature += n;
            }
            if types.intersects(TypeSet::INSTANT.union(TypeSet::SORCERY)) {
                counts.spells += n;
            }
            if types.contains(TypeSet::ARTIFACT) {
                counts.artifacts += n;
            }
            if types.contains(TypeSet::ENCHANTMENT) {
                counts.enchantments += n;
            }
            if role(ROLE_TOKENS) {
                counts.tokens += n;
            }
            if role(ROLE_LIFEGAIN) {
                counts.lifegain += n;
            }
            if role(ROLE_CARD_DRAW | ROLE_CANTRIP) {
                counts.draws += n;
            }
            if role(ROLE_GY_FILLER) {
                counts.discards += n;
            }
            if role(MASK_INTERACTION) {
                counts.removal += n * if role(ROLE_BOARD_WIPE) { 2.0 } else { 1.0 };
            }
        }
        counts
    }

    /// Expected matching events in one duel.
    fn events(&self, sigil: Sigil) -> f64 {
        let entering = (self.creatures + 1.0) * SEEN;
        let tokens = self.tokens * SEEN * 2.0;
        let attacking = (self.creatures / 2.0).min(TURNS * 0.6);
        match sigil {
            Sigil::Dawn | Sigil::Dusk => TURNS - 0.5,
            Sigil::Root => (self.lands * SEEN).min(TURNS),
            Sigil::Hearth => entering,
            Sigil::Bloom => tokens,
            Sigil::Ash => (entering + tokens) * 0.45,
            Sigil::Ember => self.noncreature * SEEN,
            Sigil::Lore => self.draws * SEEN * 1.5,
            Sigil::Pulse => self.lifegain * SEEN * 2.0,
            Sigil::Forge => self.artifacts * SEEN,
            Sigil::Veil => self.enchantments * SEEN,
            Sigil::Valor => attacking,
            Sigil::Fang => attacking * 0.6,
            Sigil::Echo => self.discards * SEEN,
            Sigil::Wraith => self.removal * SEEN + 2.0,
            Sigil::Star => self.spells * SEEN,
        }
    }

    /// Creatures on the battlefield when a team-wide effect resolves.
    fn board(&self) -> f64 {
        ((self.creatures + self.tokens * 2.0) / 8.0).clamp(1.0, 4.0)
    }
}

/// The worth of one equipped definition to `deck`.
pub(super) fn of(definition: Definition, deck: &Deck) -> f64 {
    let tier = usize::from(definition.rarity.tier() - 1);
    match definition.family {
        Family::Runic(recipe) => runic(recipe, tier, deck),
        family => original(family, tier, deck),
    }
}

fn runic(recipe: RunicRecipe, tier: usize, deck: &Deck) -> f64 {
    let Some(level) = recipe.level(tier) else {
        return 0.0;
    };
    let charges = deck.events(recipe.sigil) * f64::from(level.gathered);
    let mut uses = charges / f64::from(level.charges.max(1));
    if level.activations > 0 {
        uses = uses.min(f64::from(level.activations) * TURNS * 0.8);
    }
    let amount = f64::from(level.amount);
    let each = match recipe.vessel {
        Vessel::Chalice => amount * 0.45,
        Vessel::Scepter => amount,
        Vessel::Prism => amount * 0.35,
        Vessel::Seed => amount * 1.4,
        Vessel::Tome => 2.4,
        Vessel::Horn => amount * 1.4,
        Vessel::Flask => 0.9,
        Vessel::Bulwark => amount * deck.board() * 0.25,
        Vessel::Banner => amount * deck.board() * 0.55,
        Vessel::Lens => 0.8,
    } - f64::from(level.mana) * 0.7;
    let bonus = if tier >= 2 {
        match recipe.vessel {
            Vessel::Chalice | Vessel::Tome => deck.events(Sigil::Lore).max(TURNS) * 0.4,
            Vessel::Scepter => deck.events(Sigil::Valor),
            Vessel::Prism => TURNS * 0.3,
            Vessel::Seed | Vessel::Horn | Vessel::Banner => deck.tokens * SEEN,
            Vessel::Flask => deck.events(Sigil::Ember) * 0.4,
            Vessel::Bulwark => deck.board() * 0.8,
            Vessel::Lens => deck.events(Sigil::Echo) * 0.4,
        }
    } else {
        0.0
    };
    uses * each.max(0.0) + bonus
}

/// Common-tier worth of an original family; higher tiers scale it.
fn original(family: Family, tier: usize, deck: &Deck) -> f64 {
    const SCALE: [f64; 4] = [1.0, 2.0, 3.3, 4.8];
    let base = match family {
        Family::Heartwood => return [4.0, 7.0, 11.0, 16.0][tier] * 0.45,
        Family::Satchel => return [0.6, 3.0, 5.5, 8.0][tier],
        Family::Compass => return [0.8, 1.5, 2.2, 2.9][tier],
        // Income, worth what it buys in the duels still to come.
        Family::GildedAcorn => return [1.0, 2.0, 3.0, 4.0][tier] * 1.2,
        Family::Grove => 3.0,
        Family::SpringChalice => TURNS * 0.4,
        Family::Hearthstone => (deck.events(Sigil::Hearth) + deck.events(Sigil::Bloom)) * 0.4,
        Family::BloodAmber => (deck.events(Sigil::Pulse) + 1.0) * 0.7,
        Family::LifeCoil => 2.2,
        Family::SeedCradle => deck.events(Sigil::Pulse) * 0.6,
        Family::SwarmBanner => deck.events(Sigil::Bloom) * 0.9,
        Family::HarvestWheel => deck.events(Sigil::Bloom) * 0.3,
        Family::RootReservoir => deck.events(Sigil::Bloom) * 0.5,
        Family::EmberLantern => deck.events(Sigil::Ember),
        Family::TideglassLens => deck.events(Sigil::Lore) * 0.3,
        Family::GraveBell => deck.events(Sigil::Ash) * 0.6,
        Family::StormSpindle => deck.events(Sigil::Ember) * 0.6,
        Family::DawnAegis => deck.board() * 0.9,
        Family::Runic(_) => 0.0,
    };
    base * SCALE[tier]
}

/// The best six of `items` by worth, the loadout a duel would carry.
pub(super) fn best_six(items: &[Definition], deck: &Deck) -> f64 {
    let mut values: Vec<f64> = items.iter().map(|item| of(*item, deck)).collect();
    values.sort_by(|a, b| b.total_cmp(a));
    values.iter().take(6).sum()
}

/// The rarity a fusion of three of `rarity` makes, if any.
pub(super) const fn fused(rarity: Rarity) -> Option<Rarity> {
    match rarity {
        Rarity::Common => Some(Rarity::Uncommon),
        Rarity::Uncommon => Some(Rarity::Rare),
        Rarity::Rare | Rarity::MythicRare => None,
    }
}
