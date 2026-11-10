//! Mechanical synergy graph, bipartite enabler/payoff matching, functional role
//! quotas, and $O(1)$ incremental deck theme accumulation.

use std::collections::BTreeMap;

use baylee_core::types::TypeSet;

use super::Archetype;
use super::profile::{
    CardProfile, ROLE_CARD_DRAW, ROLE_COUNTERS, ROLE_LAND_RAMP, ROLE_LORD_ANTHEM,
    ROLE_MANA_PRODUCER, ROLE_SPELLSLINGER, ROLE_SPOT_REMOVAL, ROLE_TOKENS,
};

/// Directed/undirected enabler-to-payoff role couplings `(role_bit_a, role_bit_b, weight)`.
const ENABLER_PAYOFF_COUPLINGS: &[(usize, usize, f64)] = &[
    // Tokens <-> Sacrifice / Death Triggers / Anthems / Counters
    (9, 10, 2.2),
    (9, 11, 2.0),
    (9, 22, 2.4),
    (9, 9, 1.2),
    // Sacrifice Outlets <-> Death Payoffs / Graveyard Recursion
    (10, 11, 2.6),
    (10, 8, 1.8),
    (11, 8, 1.6),
    // Graveyard Fillers (Self-Mill/Surveil) <-> Graveyard Payoffs / Reanimator / Finishers
    (7, 8, 2.8),
    (7, 23, 1.2),
    (8, 23, 1.5),
    // +1/+1 Counters <-> Counters / Evasion / Tokens
    (12, 12, 2.0),
    (12, 16, 1.4),
    (12, 9, 1.1),
    // Spellslinger / Prowess <-> Cantrips / Burn / Countermagic / Combat Tricks
    (13, 3, 2.5),
    (13, 20, 2.0),
    (13, 6, 1.6),
    (13, 17, 1.8),
    (13, 13, 1.8),
    // Artifact & Enchantment density clusters
    (14, 14, 1.8),
    (15, 15, 1.9),
    // Lifegain & Burn/Drain clusters
    (19, 19, 2.1),
    (20, 20, 1.4),
    // Blink & ETB value clusters
    (21, 21, 1.9),
    (21, 2, 1.2),
    // Ramp <-> Finishers & Card Draw
    (0, 23, 1.7),
    (1, 23, 1.9),
    (0, 2, 1.2),
    (1, 2, 1.3),
];

/// Computes the pairwise mechanical synergy score between two cards `a` and `b`.
#[must_use]
pub fn pair_synergy(a: &CardProfile, b: &CardProfile) -> f64 {
    let mut score = 0.0_f64;

    // 1. Bipartite Enabler <-> Payoff role couplings
    for &(ra, rb, w) in ENABLER_PAYOFF_COUPLINGS {
        let a_has_ra = (a.roles >> ra) & 1 == 1;
        let b_has_rb = (b.roles >> rb) & 1 == 1;
        let a_has_rb = (a.roles >> rb) & 1 == 1;
        let b_has_ra = (b.roles >> ra) & 1 == 1;
        if (a_has_ra && b_has_rb) || (ra != rb && a_has_rb && b_has_ra) {
            score += w;
        }
    }

    // 2. Spellslinger payoff paired with any cheap instant/sorcery
    let a_inst_sorc = a.types.contains(TypeSet::INSTANT) || a.types.contains(TypeSet::SORCERY);
    let b_inst_sorc = b.types.contains(TypeSet::INSTANT) || b.types.contains(TypeSet::SORCERY);
    if ((a.roles & ROLE_SPELLSLINGER != 0) && b_inst_sorc && b.mana_value <= 3)
        || ((b.roles & ROLE_SPELLSLINGER != 0) && a_inst_sorc && a.mana_value <= 3)
    {
        score += 1.5;
    }

    // 3. Tribal subtype alignment (shared creature subtype, amplified by lords/anthems)
    if !a.subtypes.is_empty() && !b.subtypes.is_empty() && (a.creature || b.creature) {
        let shared = a.subtypes.iter().any(|st| b.subtypes.contains(st));
        if shared {
            let lord_bonus = if (a.roles | b.roles) & ROLE_LORD_ANTHEM != 0 {
                2.5
            } else {
                1.1
            };
            score += lord_bonus;
        }
    }

    // 3b. Snow permanents & snow-matter payoffs
    if a.cares_about_snow && b.cares_about_snow {
        score += 1.8;
    }

    // 4. Continuous 48-dim CardDsl cosine similarity
    let mut dot = 0.0_f32;
    for i in 0..48 {
        dot += a.dsl_vec[i] * b.dsl_vec[i];
    }
    if dot > 0.0 {
        score += f64::from(dot) * 1.5;
    }

    score
}

/// Incremental $O(1)$ accumulator of a deck's mechanical theme, functional quotas,
/// and mana curve.
#[derive(Clone, Debug)]
pub struct DeckThemeAccumulator {
    /// Count of cards carrying each of the 24 `CardDsl` role bits.
    pub role_counts: [u16; 24],
    /// Count of cards per creature subtype ID.
    pub subtype_counts: BTreeMap<u16, u16>,
    /// Count of spells per mana-value bucket `0..=7+`.
    pub curve_counts: [u16; 8],
    /// Number of non-land cards currently in the deck.
    pub spell_count: u16,
    /// Number of creatures currently in the deck.
    pub creature_count: u16,
    /// Number of artifacts currently in the deck.
    pub artifact_count: u16,
    /// Number of enchantments currently in the deck.
    pub enchantment_count: u16,
    /// Number of instants and sorceries currently in the deck.
    pub instant_sorcery_count: u16,
    /// Number of mana ramp cards (`MASK_RAMP`).
    pub ramp_count: u16,
    /// Number of interactive removal/counter/wipe cards (`MASK_INTERACTION`).
    pub interaction_count: u16,
    /// Number of card-advantage/draw/tutor/recursion cards (`MASK_CARD_ADVANTAGE`).
    pub card_advantage_count: u16,
    /// Number of snow-synergy cards currently in the deck.
    pub snow_count: u16,
    /// Number of Equipment and Aura cards currently in the deck.
    pub equipment_aura_count: u16,
    /// Running sum of `dsl_vec` across all added cards.
    pub dsl_sum: [f32; 48],
}

impl Default for DeckThemeAccumulator {
    fn default() -> Self {
        Self {
            role_counts: [0; 24],
            subtype_counts: BTreeMap::new(),
            curve_counts: [0; 8],
            spell_count: 0,
            creature_count: 0,
            artifact_count: 0,
            enchantment_count: 0,
            instant_sorcery_count: 0,
            ramp_count: 0,
            interaction_count: 0,
            card_advantage_count: 0,
            snow_count: 0,
            equipment_aura_count: 0,
            dsl_sum: [0.0; 48],
        }
    }
}

impl DeckThemeAccumulator {
    /// Adds `copies` of `profile` to the running deck summary.
    pub fn add(&mut self, profile: &CardProfile, copies: u16) {
        if copies == 0 {
            return;
        }
        for bit in 0..24 {
            if (profile.roles >> bit) & 1 == 1 {
                self.role_counts[bit] = self.role_counts[bit].saturating_add(copies);
            }
        }
        if profile.creature {
            for &st in &profile.subtypes {
                let entry = self.subtype_counts.entry(st).or_insert(0);
                *entry = entry.saturating_add(copies);
            }
        }
        if profile.cares_about_snow {
            self.snow_count = self.snow_count.saturating_add(copies);
        }
        if profile.is_equipment_or_aura {
            self.equipment_aura_count = self.equipment_aura_count.saturating_add(copies);
        }
        if !profile.land {
            self.spell_count = self.spell_count.saturating_add(copies);
            let bucket = (profile.mana_value as usize).min(7);
            self.curve_counts[bucket] = self.curve_counts[bucket].saturating_add(copies);
            if profile.creature {
                self.creature_count = self.creature_count.saturating_add(copies);
            }
            if profile.types.contains(TypeSet::ARTIFACT) {
                self.artifact_count = self.artifact_count.saturating_add(copies);
            }
            if profile.types.contains(TypeSet::ENCHANTMENT) {
                self.enchantment_count = self.enchantment_count.saturating_add(copies);
            }
            if profile.types.contains(TypeSet::INSTANT) || profile.types.contains(TypeSet::SORCERY)
            {
                self.instant_sorcery_count = self.instant_sorcery_count.saturating_add(copies);
            }
            if profile.is_ramp() {
                self.ramp_count = self.ramp_count.saturating_add(copies);
            }
            if profile.is_interaction() {
                self.interaction_count = self.interaction_count.saturating_add(copies);
            }
            if profile.is_card_advantage() {
                self.card_advantage_count = self.card_advantage_count.saturating_add(copies);
            }
        }
        let mult = f32::from(copies);
        for (dst, &src) in self.dsl_sum.iter_mut().zip(&profile.dsl_vec) {
            *dst += src * mult;
        }
    }

    /// Removes `copies` of `profile` from the running deck summary.
    pub fn remove(&mut self, profile: &CardProfile, copies: u16) {
        if copies == 0 {
            return;
        }
        for bit in 0..24 {
            if (profile.roles >> bit) & 1 == 1 {
                self.role_counts[bit] = self.role_counts[bit].saturating_sub(copies);
            }
        }
        if profile.creature {
            for &st in &profile.subtypes {
                if let Some(entry) = self.subtype_counts.get_mut(&st) {
                    *entry = entry.saturating_sub(copies);
                }
            }
        }
        if profile.cares_about_snow {
            self.snow_count = self.snow_count.saturating_sub(copies);
        }
        if profile.is_equipment_or_aura {
            self.equipment_aura_count = self.equipment_aura_count.saturating_sub(copies);
        }
        if !profile.land {
            self.spell_count = self.spell_count.saturating_sub(copies);
            let bucket = (profile.mana_value as usize).min(7);
            self.curve_counts[bucket] = self.curve_counts[bucket].saturating_sub(copies);
            if profile.creature {
                self.creature_count = self.creature_count.saturating_sub(copies);
            }
            if profile.types.contains(TypeSet::ARTIFACT) {
                self.artifact_count = self.artifact_count.saturating_sub(copies);
            }
            if profile.types.contains(TypeSet::ENCHANTMENT) {
                self.enchantment_count = self.enchantment_count.saturating_sub(copies);
            }
            if profile.types.contains(TypeSet::INSTANT) || profile.types.contains(TypeSet::SORCERY)
            {
                self.instant_sorcery_count = self.instant_sorcery_count.saturating_sub(copies);
            }
            if profile.is_ramp() {
                self.ramp_count = self.ramp_count.saturating_sub(copies);
            }
            if profile.is_interaction() {
                self.interaction_count = self.interaction_count.saturating_sub(copies);
            }
            if profile.is_card_advantage() {
                self.card_advantage_count = self.card_advantage_count.saturating_sub(copies);
            }
        }
        let mult = f32::from(copies);
        for (dst, &src) in self.dsl_sum.iter_mut().zip(&profile.dsl_vec) {
            *dst -= src * mult;
        }
    }

    /// Evaluates the $O(1)$ mechanical synergy between `cand` and the current deck.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "Small bounded card counts in a deck"
    )]
    pub fn synergy_score(&self, cand: &CardProfile) -> f64 {
        let denom = f64::from(self.spell_count.max(1));
        let mut score = 0.0_f64;

        // 1. Bipartite Enabler <-> Payoff couplings against accumulated deck roles
        //    (concave-compressed so multi-role Swiss-army cards don't dominate)
        let mut role_score = 0.0_f64;
        for &(ra, rb, w) in ENABLER_PAYOFF_COUPLINGS {
            if (cand.roles >> ra) & 1 == 1 {
                let partner_cnt = f64::from(self.role_counts[rb]);
                if partner_cnt > 0.0 {
                    role_score += w * (partner_cnt / denom).sqrt();
                }
            }
            if ra != rb && (cand.roles >> rb) & 1 == 1 {
                let partner_cnt = f64::from(self.role_counts[ra]);
                if partner_cnt > 0.0 {
                    role_score += w * (partner_cnt / denom).sqrt();
                }
            }
        }
        score += 3.2 * (1.0 - (-role_score / 2.8).exp());

        // 2. Spellslinger density coupling
        let cand_inst_sorc =
            cand.types.contains(TypeSet::INSTANT) || cand.types.contains(TypeSet::SORCERY);
        if (cand.roles & ROLE_SPELLSLINGER != 0) && self.instant_sorcery_count > 0 {
            score += 1.6 * (f64::from(self.instant_sorcery_count) / denom);
        } else if cand_inst_sorc && cand.mana_value <= 3 && self.role_counts[13] > 0 {
            score += 1.6 * (f64::from(self.role_counts[13]) / denom).sqrt();
        }

        // 3. Tribal subtype coupling
        if cand.creature && !cand.subtypes.is_empty() {
            let max_shared = cand
                .subtypes
                .iter()
                .filter_map(|st| self.subtype_counts.get(st).copied())
                .max()
                .unwrap_or(0);
            if max_shared > 0 {
                let mult = if (cand.roles & ROLE_LORD_ANTHEM != 0) || self.role_counts[22] > 0 {
                    2.2
                } else {
                    1.2
                };
                score += mult * (f64::from(max_shared) / denom).sqrt();
            }
        }

        // 3b. Snow permanents & snow-matter payoffs
        if cand.cares_about_snow && self.snow_count > 0 {
            score += 1.8 * (f64::from(self.snow_count) / denom).sqrt();
        }

        // 4. Centroid cosine similarity in 48-dim CardDsl space
        let mut dot = 0.0_f32;
        for i in 0..48 {
            dot += self.dsl_sum[i] * cand.dsl_vec[i];
        }
        if dot > 0.0 {
            score += (f64::from(dot) / denom) * 1.8;
        }

        score
    }

    /// Rewards filling missing functional quotas (Ramp, Interaction/Removal,
    /// Card Advantage/Draw, and Creature/Non-Creature balance) for `archetype`.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Small bounded card quota calculations"
    )]
    pub fn quota_bonus(
        &self,
        cand: &CardProfile,
        archetype: Archetype,
        target_spells: usize,
    ) -> f64 {
        let spells_f = target_spells.max(1) as f64;
        let (ramp_share, inter_share, draw_share) = match archetype {
            Archetype::Aggro => (0.04, 0.16, 0.10),
            Archetype::Midrange => (0.12, 0.18, 0.14),
            Archetype::Control => (0.12, 0.28, 0.20),
            Archetype::Ramp => (0.22, 0.14, 0.16),
        };
        let target_ramp = (spells_f * ramp_share).round() as u16;
        let target_inter = (spells_f * inter_share).round() as u16;
        let target_draw = (spells_f * draw_share).round() as u16;
        let target_creatures = (spells_f * archetype.creature_share()).round() as u16;

        let mut bonus = 0.0_f64;
        if cand.is_ramp() && self.ramp_count < target_ramp {
            let deficit = f64::from(target_ramp - self.ramp_count) / f64::from(target_ramp.max(1));
            bonus += 1.8 * deficit;
        }
        if cand.is_interaction() && self.interaction_count < target_inter {
            let deficit =
                f64::from(target_inter - self.interaction_count) / f64::from(target_inter.max(1));
            bonus += 2.0 * deficit;
        }
        if cand.is_card_advantage() && self.card_advantage_count < target_draw {
            let deficit =
                f64::from(target_draw - self.card_advantage_count) / f64::from(target_draw.max(1));
            bonus += 1.7 * deficit;
        }
        // Equipment/Aura guard: ensure enough creatures to wear them
        if cand.is_equipment_or_aura {
            let max_eq = (f64::from(self.creature_count) * 0.25).ceil() as u16 + 2;
            if self.equipment_aura_count >= max_eq {
                bonus -= 2.5;
            }
        } else if cand.creature && self.equipment_aura_count > 2 {
            bonus += 0.6;
        }
        // Creature vs non-creature balance
        if cand.creature {
            if self.creature_count < target_creatures {
                bonus += 0.9;
            } else {
                bonus -= 0.4;
            }
        } else {
            let non_creatures = self.spell_count.saturating_sub(self.creature_count);
            let target_non_creatures = (target_spells as u16).saturating_sub(target_creatures);
            if non_creatures < target_non_creatures {
                bonus += 0.9;
            } else {
                bonus -= 0.4;
            }
        }
        // Slight Intrinsic quality bonus for cards that have any active mechanical role
        if cand.roles != 0
            || (cand.roles
                & (ROLE_SPOT_REMOVAL
                    | ROLE_CARD_DRAW
                    | ROLE_MANA_PRODUCER
                    | ROLE_LAND_RAMP
                    | ROLE_TOKENS
                    | ROLE_COUNTERS))
                != 0
        {
            bonus += 0.35;
        }
        bonus
    }

    /// Rewards filling under-populated mana-value buckets for `archetype` and
    /// penalizes buckets that already exceed their curve target.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "Curve proportions over bounded deck spell counts"
    )]
    pub fn curve_bonus(
        &self,
        cand: &CardProfile,
        archetype: Archetype,
        target_spells: usize,
    ) -> f64 {
        let bucket = (cand.mana_value as usize).min(7);
        let curve = archetype.curve();
        let target_in_bucket = (target_spells as f64) * curve[bucket];
        let have = f64::from(self.curve_counts[bucket]);
        if target_in_bucket <= 0.25 {
            return -2.5;
        }
        let diff = target_in_bucket - have;
        if diff > 0.0 {
            1.6 * (diff / target_in_bucket.max(1.0)).min(1.5)
        } else {
            -1.4 * ((-diff + 1.0) / target_in_bucket.max(1.0)).min(2.5)
        }
    }
}
