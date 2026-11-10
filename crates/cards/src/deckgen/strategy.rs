//! Reusable deck strategy classification, functional card replacement, and
//! canonical multiset Jaccard similarity.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::dsl::CardDef;
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::types::{SupertypeSet, TypeSet};

use super::profile::CardProfile;
use crate::carddsl::encode_card_face;

/// Formats a [`ColorSet`] into canonical `WUBRG` order (or `"C"` when empty).
#[must_use]
pub fn color_string(cs: ColorSet) -> String {
    let mut s = String::new();
    for (c, ch) in [
        (Color::White, 'W'),
        (Color::Blue, 'U'),
        (Color::Black, 'B'),
        (Color::Red, 'R'),
        (Color::Green, 'G'),
    ] {
        if cs.contains(c) {
            s.push(ch);
        }
    }
    if s.is_empty() { "C".to_string() } else { s }
}

/// Classifies a deck's primary archetype and multi-label strategy tags from its
/// actual card definitions (`crate::by_index`).
#[must_use]
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    reason = "Single pass over card definitions followed by independent strategy tags"
)]
pub fn classify_deck_strategy(
    main: &[CardIndex],
    commanders: &[CardIndex],
) -> (String, Vec<String>, ColorSet) {
    let mut colors = ColorSet::EMPTY;
    let mut nonland = 0_usize;
    let mut creatures = 0_usize;
    let mut artifacts = 0_usize;
    let mut enchantments = 0_usize;
    let mut instants_sorceries = 0_usize;
    let mut lands = 0_usize;
    let mut cmc_sum = 0_u32;
    let mut low_cmc_creatures = 0_usize;
    let mut big_drops = 0_usize;

    let mut token_makers = 0_usize;
    let mut counter_synergy = 0_usize;
    let mut graveyard_reanim = 0_usize;
    let mut sacrifice_cards = 0_usize;
    let mut blink_etb = 0_usize;
    let mut ramp_mana = 0_usize;
    let mut lifegain_cards = 0_usize;
    let mut burn_drain_cards = 0_usize;
    let mut subtype_counts: BTreeMap<u16, usize> = BTreeMap::new();

    for &cid in main.iter().chain(commanders) {
        let Some(def) = crate::by_index(cid) else {
            continue;
        };
        colors = colors.union(def.color_identity);
        let face = &def.faces[0];
        if face.types.contains(TypeSet::LAND) {
            lands += 1;
            continue;
        }
        nonland += 1;
        let cmc = face.mana_cost.cmc();
        cmc_sum += cmc;
        if cmc >= 6 {
            big_drops += 1;
        }
        if face.types.contains(TypeSet::CREATURE) {
            creatures += 1;
            if cmc <= 2 {
                low_cmc_creatures += 1;
            }
            for st in face.subtypes {
                *subtype_counts.entry(st.get()).or_insert(0) += 1;
            }
        }
        if face.types.contains(TypeSet::ARTIFACT) {
            artifacts += 1;
        }
        if face.types.contains(TypeSet::ENCHANTMENT) {
            enchantments += 1;
        }
        if face.types.contains(TypeSet::INSTANT) || face.types.contains(TypeSet::SORCERY) {
            instants_sorceries += 1;
        }
        inspect_def_mechanics(
            def,
            &mut token_makers,
            &mut counter_synergy,
            &mut graveyard_reanim,
            &mut sacrifice_cards,
            &mut blink_etb,
            &mut ramp_mana,
            &mut lifegain_cards,
            &mut burn_drain_cards,
        );
    }

    let avg_cmc = if nonland > 0 {
        f64::from(cmc_sum) / nonland as f64
    } else {
        3.0
    };
    let creature_share = if nonland > 0 {
        creatures as f64 / nonland as f64
    } else {
        0.5
    };

    let primary = if avg_cmc <= 2.45 && creature_share >= 0.55 {
        "aggro"
    } else if creature_share <= 0.32 && instants_sorceries >= nonland / 3 {
        "control"
    } else if ramp_mana >= 6 && (big_drops >= 5 || avg_cmc >= 3.4) {
        "ramp"
    } else {
        "midrange"
    };

    let mut tags = vec![primary.to_string()];
    if low_cmc_creatures >= 10 && instants_sorceries >= 10 {
        tags.push("tempo".to_string());
    }
    if instants_sorceries >= 16 {
        tags.push("spellslinger".to_string());
    }
    if artifacts >= 10 {
        tags.push("artifacts".to_string());
    }
    if enchantments >= 10 {
        tags.push("enchantments".to_string());
    }
    if token_makers >= 4 {
        tags.push("tokens".to_string());
    }
    if counter_synergy >= 4 {
        tags.push("counters".to_string());
    }
    if graveyard_reanim >= 4 {
        tags.push("graveyard".to_string());
        if big_drops >= 3 {
            tags.push("reanimator".to_string());
        }
    }
    if sacrifice_cards >= 4 {
        tags.push("sacrifice".to_string());
    }
    if blink_etb >= 5 {
        tags.push("blink_etb".to_string());
    }
    if lifegain_cards >= 5 {
        tags.push("lifegain".to_string());
    }
    if burn_drain_cards >= 6 {
        tags.push("burn".to_string());
    }
    if ramp_mana >= 6 && !tags.contains(&"ramp".to_string()) {
        tags.push("ramp".to_string());
    }
    if lands >= (main.len() * 44) / 100 {
        tags.push("lands".to_string());
    }
    if subtype_counts.values().copied().max().unwrap_or(0) >= 8 {
        tags.push("tribal".to_string());
    }
    if !commanders.is_empty() && (artifacts + enchantments) >= 14 && creatures <= 22 {
        tags.push("voltron".to_string());
    }
    tags.sort();
    tags.dedup();
    (primary.to_string(), tags, colors)
}

#[allow(clippy::too_many_arguments)]
fn inspect_def_mechanics(
    def: &CardDef,
    token_makers: &mut usize,
    counter_synergy: &mut usize,
    graveyard_reanim: &mut usize,
    sacrifice_cards: &mut usize,
    blink_etb: &mut usize,
    ramp_mana: &mut usize,
    lifegain_cards: &mut usize,
    burn_drain_cards: &mut usize,
) {
    let dsl = encode_card_face(def, 0);
    let roles = &dsl.summary[120..144];
    if roles[0] > 0.0 || roles[1] > 0.0 {
        *ramp_mana += 1;
    }
    if roles[7] > 0.0 || roles[8] > 0.0 {
        *graveyard_reanim += 1;
    }
    if roles[9] > 0.0 {
        *token_makers += 1;
    }
    if roles[10] > 0.0 || roles[11] > 0.0 {
        *sacrifice_cards += 1;
    }
    if roles[12] > 0.0 {
        *counter_synergy += 1;
    }
    if roles[19] > 0.0 {
        *lifegain_cards += 1;
    }
    if roles[20] > 0.0 {
        *burn_drain_cards += 1;
    }
    if roles[21] > 0.0 {
        *blink_etb += 1;
    }
}

/// Finds a functionally matched working replacement for a card in an adapted
/// deck, matching card type, colour identity, mana value, and `CardDsl` roles
/// while never substituting a basic land for a non-land card.
#[must_use]
pub fn find_functional_replacement(
    orig_name: &str,
    deck_colors: ColorSet,
    copy_limit: u32,
    current_counts: &BTreeMap<CardIndex, u32>,
    working_pool: &[CardIndex],
) -> Option<(CardIndex, &'static str)> {
    let orig_def = crate::decks::by_name(orig_name).and_then(crate::by_index);
    let orig_profile = orig_def.map(CardProfile::from_def);
    let target_is_land = orig_def.map_or_else(
        || {
            let lower = orig_name.to_ascii_lowercase();
            lower.contains("land")
                || lower.contains("isle")
                || lower.contains("haven")
                || lower.contains("tower")
                || lower.contains("ruin")
                || lower.contains("temple")
        },
        |d| d.faces[0].types.contains(TypeSet::LAND),
    );
    let target_is_creature = orig_def.is_some_and(|d| d.faces[0].types.contains(TypeSet::CREATURE));
    let target_is_artifact = orig_def.is_some_and(|d| d.faces[0].types.contains(TypeSet::ARTIFACT));
    let target_is_enchantment =
        orig_def.is_some_and(|d| d.faces[0].types.contains(TypeSet::ENCHANTMENT));
    let target_cmc = orig_def.map_or(3, |d| d.faces[0].mana_cost.cmc());
    let target_ci = orig_def.map_or(deck_colors, |d| d.color_identity.intersection(deck_colors));
    let target_roles = orig_profile.as_ref().map_or(0, |p| p.roles);

    let mut name_hash = 0xcbf2_9ce4_8422_2325_u64;
    for b in orig_name.bytes() {
        name_hash ^= u64::from(b);
        name_hash = name_hash.wrapping_mul(0x0100_0000_01b3);
    }

    let mut best: Option<((u32, u32, u64), CardIndex, &'static str)> = None;
    for &cand in working_pool {
        let Some(def) = crate::by_index(cand) else {
            continue;
        };
        let face = &def.faces[0];
        let is_basic =
            face.types.contains(TypeSet::LAND) && face.supertypes.contains(SupertypeSet::BASIC);
        if !target_is_land && face.types.contains(TypeSet::LAND) {
            continue;
        }
        if target_is_land && !face.types.contains(TypeSet::LAND) {
            continue;
        }
        if def.color_identity.union(deck_colors) != deck_colors {
            continue;
        }
        let cur = current_counts.get(&cand).copied().unwrap_or(0);
        if !is_basic && cur >= copy_limit {
            continue;
        }
        let type_penalty: u32 = if target_is_land {
            u32::from(is_basic) * 2
        } else if target_is_creature {
            u32::from(!face.types.contains(TypeSet::CREATURE)) * 4
        } else if target_is_artifact {
            u32::from(!face.types.contains(TypeSet::ARTIFACT)) * 4
        } else if target_is_enchantment {
            u32::from(!face.types.contains(TypeSet::ENCHANTMENT)) * 4
        } else {
            u32::from(
                !(face.types.contains(TypeSet::INSTANT) || face.types.contains(TypeSet::SORCERY)),
            ) * 3
        };
        let ci_penalty: u32 = if def.color_identity == target_ci {
            0
        } else if !def.color_identity.intersection(target_ci).is_empty() {
            1
        } else {
            2
        };
        let role_penalty: u32 = if target_roles != 0 {
            let cand_prof = CardProfile::from_def(def);
            u32::from((cand_prof.roles & target_roles) == 0)
        } else {
            0
        };
        let cmc_diff = face.mana_cost.cmc().abs_diff(target_cmc);
        let tie = name_hash.wrapping_add(u64::from(cand.get()).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let score = (type_penalty + ci_penalty + role_penalty, cmc_diff, tie);
        if best.is_none_or(|(b_score, _, _)| score < b_score) {
            best = Some((score, cand, def.name()));
        }
    }
    best.map(|(_, cid, name)| (cid, name))
}

/// Builds a canonical `CardIndex -> count` multiset over `main` and `commanders`.
#[must_use]
pub fn compute_multiset(main: &[CardIndex], commanders: &[CardIndex]) -> BTreeMap<CardIndex, u32> {
    let mut m = BTreeMap::new();
    for &c in main.iter().chain(commanders) {
        *m.entry(c).or_insert(0) += 1;
    }
    m
}

/// Computes a deterministic 32-hex-char (128-bit) fingerprint of a deck's card
/// multiset and commanders.
#[must_use]
pub fn compute_deck_hash(multiset: &BTreeMap<CardIndex, u32>, commanders: &[CardIndex]) -> String {
    let mut h1 = 0xcbf2_9ce4_8422_2325_u64;
    let mut h2 = 0x8422_2325_cbf2_9ce4_u64;
    let feed = |h1: &mut u64, h2: &mut u64, bytes: [u8; 4]| {
        for b in bytes {
            *h1 ^= u64::from(b);
            *h1 = h1.wrapping_mul(0x0100_0000_01b3);
            *h2 = h2
                .wrapping_add(u64::from(b).wrapping_mul(0x9e37_79b9_7f4a_7c15))
                .rotate_left(13)
                .wrapping_mul(0xbf58_476d_1ce4_e5b9);
        }
    };
    for (&cid, &cnt) in multiset {
        feed(&mut h1, &mut h2, cid.get().to_le_bytes());
        feed(&mut h1, &mut h2, cnt.to_le_bytes());
    }
    feed(&mut h1, &mut h2, 0xFF_u32.to_le_bytes());
    let mut cmds: Vec<u32> = commanders.iter().map(|c| c.get()).collect();
    cmds.sort_unstable();
    for c in cmds {
        feed(&mut h1, &mut h2, c.to_le_bytes());
    }
    let mut hex = String::with_capacity(32);
    let _ = write!(hex, "{h1:016x}{h2:016x}");
    hex
}

/// Computes exact weighted Jaccard similarity `sum(min(a_i, b_i)) / sum(max(a_i, b_i))`
/// between two sorted card multisets in $O(|A| + |B|)$ time.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "Approximate similarity ratios of bounded deck card counts"
)]
pub fn multiset_jaccard(a: &BTreeMap<CardIndex, u32>, b: &BTreeMap<CardIndex, u32>) -> f64 {
    let mut inter = 0_u64;
    let mut union = 0_u64;
    let mut it_a = a.iter().peekable();
    let mut it_b = b.iter().peekable();
    while it_a.peek().is_some() || it_b.peek().is_some() {
        match (it_a.peek(), it_b.peek()) {
            (Some(&(&ka, &va)), Some(&(&kb, &vb))) => match ka.cmp(&kb) {
                Ordering::Equal => {
                    inter += u64::from(va.min(vb));
                    union += u64::from(va.max(vb));
                    it_a.next();
                    it_b.next();
                }
                Ordering::Less => {
                    union += u64::from(va);
                    it_a.next();
                }
                Ordering::Greater => {
                    union += u64::from(vb);
                    it_b.next();
                }
            },
            (Some(&(_, &va)), None) => {
                union += u64::from(va);
                it_a.next();
            }
            (None, Some(&(_, &vb))) => {
                union += u64::from(vb);
                it_b.next();
            }
            (None, None) => break,
        }
    }
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// Inverted-index accelerator for exact multiset Jaccard similarity queries
/// (`max_{D in corpus} J(query, D)`).
#[derive(Clone, Debug, Default)]
pub struct MultisetSimilarityIndex {
    postings: BTreeMap<CardIndex, Vec<(usize, u32)>>,
    sizes: Vec<u64>,
}

impl MultisetSimilarityIndex {
    /// Creates an empty similarity index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an index pre-populated with `multisets`.
    #[must_use]
    pub fn from_slice(multisets: &[BTreeMap<CardIndex, u32>]) -> Self {
        let mut idx = Self::new();
        for ms in multisets {
            idx.add(ms);
        }
        idx
    }

    /// Adds one deck multiset to the index.
    pub fn add(&mut self, multiset: &BTreeMap<CardIndex, u32>) {
        let deck_id = self.sizes.len();
        let total: u64 = multiset.values().map(|&v| u64::from(v)).sum();
        self.sizes.push(total);
        for (&cid, &cnt) in multiset {
            self.postings.entry(cid).or_default().push((deck_id, cnt));
        }
    }

    /// Returns the maximum exact multiset Jaccard similarity between `query`
    /// and any deck currently in the index.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn max_jaccard(&self, query: &BTreeMap<CardIndex, u32>) -> f64 {
        if self.sizes.is_empty() || query.is_empty() {
            return 0.0;
        }
        let query_size: u64 = query.values().map(|&v| u64::from(v)).sum();
        if query_size == 0 {
            return 0.0;
        }
        let mut intersections = vec![0_u64; self.sizes.len()];
        for (&cid, &q_cnt) in query {
            if let Some(list) = self.postings.get(&cid) {
                for &(deck_id, d_cnt) in list {
                    intersections[deck_id] += u64::from(q_cnt.min(d_cnt));
                }
            }
        }
        let mut max_j = 0.0_f64;
        for (deck_id, &inter) in intersections.iter().enumerate() {
            if inter == 0 {
                continue;
            }
            let union = query_size + self.sizes[deck_id] - inter;
            if union > 0 {
                let j = (inter as f64) / (union as f64);
                if j > max_j {
                    max_j = j;
                }
            }
        }
        max_j
    }
}
