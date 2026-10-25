//! Semantic card profile for synergistic deck construction, mana-base pip math,
//! and functional replacement.

use baylee_cards::decks::leader_of;
use baylee_cards::dsl::{CardDef, EnterModifier};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaCost, ManaSymbol};
use baylee_core::types::{SupertypeSet, TypeSet};

use super::Archetype;
use crate::carddsl::encode_card_face;

/// Role bit 0: non-land mana producer (mana dork, mana rock, treasure generator).
pub const ROLE_MANA_PRODUCER: u32 = 1 << 0;
/// Role bit 1: land ramp or land tutor (`cmc <= 4`).
pub const ROLE_LAND_RAMP: u32 = 1 << 1;
/// Role bit 2: card draw engine.
pub const ROLE_CARD_DRAW: u32 = 1 << 2;
/// Role bit 3: cheap cantrip / card selection (`cmc <= 2`).
pub const ROLE_CANTRIP: u32 = 1 << 3;
/// Role bit 4: single-target removal (destroy, exile, or `>= 2` any-target damage).
pub const ROLE_SPOT_REMOVAL: u32 = 1 << 4;
/// Role bit 5: board wipe / mass sweeper.
pub const ROLE_BOARD_WIPE: u32 = 1 << 5;
/// Role bit 6: stack countermagic.
pub const ROLE_COUNTERMAGIC: u32 = 1 << 6;
/// Role bit 7: graveyard filler (self-mill, surveil, entomb).
pub const ROLE_GY_FILLER: u32 = 1 << 7;
/// Role bit 8: graveyard payoff (reanimation, flashback, escape, disturb).
pub const ROLE_GY_PAYOFF: u32 = 1 << 8;
/// Role bit 9: token generator.
pub const ROLE_TOKENS: u32 = 1 << 9;
/// Role bit 10: free or repeatable sacrifice outlet.
pub const ROLE_SAC_OUTLET: u32 = 1 << 10;
/// Role bit 11: dies / leaves-battlefield trigger payoff.
pub const ROLE_DEATH_PAYOFF: u32 = 1 << 11;
/// Role bit 12: +1/+1 or other counter synergy.
pub const ROLE_COUNTERS: u32 = 1 << 12;
/// Role bit 13: spellslinger / prowess / magecraft payoff.
pub const ROLE_SPELLSLINGER: u32 = 1 << 13;
/// Role bit 14: artifact or artifact-matters card.
pub const ROLE_ARTIFACT: u32 = 1 << 14;
/// Role bit 15: enchantment or enchantment-matters card.
pub const ROLE_ENCHANTMENT: u32 = 1 << 15;
/// Role bit 16: evasive creature threat (flying, menace, trample, unblockable).
pub const ROLE_EVASION: u32 = 1 << 16;
/// Role bit 17: instant-speed combat pump trick.
pub const ROLE_COMBAT_TRICK: u32 = 1 << 17;
/// Role bit 18: library tutor / search effect.
pub const ROLE_TUTOR: u32 = 1 << 18;
/// Role bit 19: lifegain source or lifegain payoff.
pub const ROLE_LIFEGAIN: u32 = 1 << 19;
/// Role bit 20: direct damage / life drain effect.
pub const ROLE_BURN_DRAIN: u32 = 1 << 20;
/// Role bit 21: enters-the-battlefield value creature or blink effect.
pub const ROLE_BLINK_ETB: u32 = 1 << 21;
/// Role bit 22: static anthem / tribal lord pump.
pub const ROLE_LORD_ANTHEM: u32 = 1 << 22;
/// Role bit 23: high-CMC finisher or alternate win condition.
pub const ROLE_FINISHER: u32 = 1 << 23;

/// Mask of roles that provide mana acceleration.
pub const MASK_RAMP: u32 = ROLE_MANA_PRODUCER | ROLE_LAND_RAMP;
/// Mask of roles that answer opponent threats or spells.
pub const MASK_INTERACTION: u32 =
    ROLE_SPOT_REMOVAL | ROLE_BOARD_WIPE | ROLE_COUNTERMAGIC | ROLE_BURN_DRAIN;
/// Mask of roles that generate card advantage, selection, or recursion.
pub const MASK_CARD_ADVANTAGE: u32 = ROLE_CARD_DRAW | ROLE_CANTRIP | ROLE_GY_PAYOFF | ROLE_TUTOR;

/// Rich semantic profile of one card for deck construction and synergy scoring.
#[derive(Clone, Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Independent card type predicates used in hot deckbuilding loops"
)]
pub struct CardProfile {
    /// Registry index of the card.
    pub card: CardIndex,
    /// Mana value (converted mana cost) of the front face.
    pub mana_value: u32,
    /// Colored and colorless mana pips on the front face: `[W, U, B, R, G, C]`.
    pub pips: [u8; 6],
    /// Colors of mana this card can produce (for lands and mana sources).
    pub produced_colors: ColorSet,
    /// Commander color identity (`CR 903.4`).
    pub identity: ColorSet,
    /// Front-face card types.
    pub types: TypeSet,
    /// Front-face supertypes.
    pub supertypes: SupertypeSet,
    /// Front face is a land.
    pub land: bool,
    /// Front face is a basic land.
    pub basic: bool,
    /// Front face is a creature.
    pub creature: bool,
    /// Card may lead a Commander deck.
    pub leader: bool,
    /// Land enters tapped unconditionally or conditionally.
    pub enters_tapped: bool,
    /// 24-bit role mask derived from `CardDsl` (`summary[120..144]`).
    pub roles: u32,
    /// Raw subtype IDs on the front face (for tribal and land-type matching).
    pub subtypes: Vec<u16>,
    /// L2-normalized 48-float semantic vector (`summary[96..144]`: 24 effect
    /// channels + 24 role tags) for fast dot-product similarity.
    pub dsl_vec: [f32; 48],
}

impl CardProfile {
    /// Builds a [`CardProfile`] from a [`CardDef`].
    #[must_use]
    pub fn from_def(def: &CardDef) -> Self {
        let face = &def.faces[0];
        let dsl = encode_card_face(def, 0);
        let land = face.types.contains(TypeSet::LAND);
        let basic = land && face.supertypes.contains(SupertypeSet::BASIC);
        let creature = face.types.contains(TypeSet::CREATURE);
        let leader = leader_of(def.index).is_some_and(|l| l.eligible);

        let mut roles = 0_u32;
        for (bit, &val) in dsl.summary[120..144].iter().enumerate() {
            if val > 0.0 {
                roles |= 1_u32 << bit;
            }
        }

        let mut dsl_vec = [0.0_f32; 48];
        dsl_vec.copy_from_slice(&dsl.summary[96..144]);
        let norm_sq: f32 = dsl_vec.iter().map(|x| x * x).sum();
        if norm_sq > 1e-8 {
            let inv = 1.0 / norm_sq.sqrt();
            for x in &mut dsl_vec {
                *x *= inv;
            }
        }

        let pips = count_mana_pips(&face.mana_cost);
        let produced_colors = if land || (roles & ROLE_MANA_PRODUCER != 0) {
            def.color_identity
        } else {
            ColorSet::EMPTY
        };
        let enters_tapped = face.enter_modifiers.iter().any(|em| {
            matches!(
                em,
                EnterModifier::Tapped
                    | EnterModifier::TappedUnless(_)
                    | EnterModifier::TappedUnlessCount { .. }
                    | EnterModifier::TappedUnlessAtMost { .. }
                    | EnterModifier::TappedUnlessCondition(_)
                    | EnterModifier::TappedUnlessOpponents { .. }
                    | EnterModifier::TappedUnlessSomeoneAtOrBelow { .. }
                    | EnterModifier::TappedUnlessReveal(_)
                    | EnterModifier::TappedUnlessRevealOrControl(_)
            )
        });
        let subtypes = face.subtypes.iter().map(|s| s.get()).collect();

        Self {
            card: def.index,
            mana_value: face.mana_cost.cmc(),
            pips,
            produced_colors,
            identity: def.color_identity,
            types: face.types,
            supertypes: face.supertypes,
            land,
            basic,
            creature,
            leader,
            enters_tapped,
            roles,
            subtypes,
            dsl_vec,
        }
    }

    /// True if this card accelerates mana (dork, rock, or land ramp).
    #[must_use]
    pub const fn is_ramp(&self) -> bool {
        (self.roles & MASK_RAMP) != 0
    }

    /// True if this card interacts with opponent threats or spells.
    #[must_use]
    pub const fn is_interaction(&self) -> bool {
        (self.roles & MASK_INTERACTION) != 0
    }

    /// True if this card provides card draw, selection, recursion, or tutoring.
    #[must_use]
    pub const fn is_card_advantage(&self) -> bool {
        (self.roles & MASK_CARD_ADVANTAGE) != 0
    }

    /// Infers the most natural [`Archetype`] when building a deck around this card.
    #[must_use]
    pub const fn preferred_archetype(&self) -> Archetype {
        if (self.roles & (ROLE_MANA_PRODUCER | ROLE_LAND_RAMP | ROLE_FINISHER)) != 0
            && self.mana_value >= 5
        {
            Archetype::Ramp
        } else if !self.creature
            && (self.roles & (ROLE_COUNTERMAGIC | ROLE_BOARD_WIPE | ROLE_CARD_DRAW)) != 0
        {
            Archetype::Control
        } else if self.creature && self.mana_value <= 2 {
            Archetype::Aggro
        } else {
            Archetype::Midrange
        }
    }
}

/// Counts `[W, U, B, R, G, C]` pip requirements in a [`ManaCost`].
#[must_use]
pub fn count_mana_pips(cost: &ManaCost) -> [u8; 6] {
    let mut pips = [0_u8; 6];
    for sym in cost.symbols() {
        match sym {
            ManaSymbol::White => pips[0] = pips[0].saturating_add(1),
            ManaSymbol::Blue => pips[1] = pips[1].saturating_add(1),
            ManaSymbol::Black => pips[2] = pips[2].saturating_add(1),
            ManaSymbol::Red => pips[3] = pips[3].saturating_add(1),
            ManaSymbol::Green => pips[4] = pips[4].saturating_add(1),
            ManaSymbol::Colorless => pips[5] = pips[5].saturating_add(1),
            ManaSymbol::Hybrid(_)
            | ManaSymbol::TwoOrColor(_)
            | ManaSymbol::Phyrexian(_)
            | ManaSymbol::HybridPhyrexian(_) => {
                let cols = sym.colors();
                for (idx, col) in [
                    (0, Color::White),
                    (1, Color::Blue),
                    (2, Color::Black),
                    (3, Color::Red),
                    (4, Color::Green),
                ] {
                    if cols.contains(col) {
                        pips[idx] = pips[idx].saturating_add(1);
                    }
                }
            }
            ManaSymbol::Generic(_)
            | ManaSymbol::Variable(_)
            | ManaSymbol::Snow
            | ManaSymbol::HalfGeneric
            | ManaSymbol::Infinite => {}
        }
    }
    pips
}
