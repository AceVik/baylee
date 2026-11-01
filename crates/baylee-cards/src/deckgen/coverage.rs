//! Coverage-constrained synergistic deck builder ($k$-Multi-Set Cover with
//! anchor seeding, functional quotas, local 1-swap refinement, and Karsten mana
//! base optimization).

use std::collections::{BTreeMap, BTreeSet};

use crate::formats::{self, Shape};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index::CRYPTIC_SPIRES;
use baylee_core::ids::CardIndex;

use super::manabase::build_manabase;
use super::strategy::{MultisetSimilarityIndex, compute_multiset};
use super::synergy::{DeckThemeAccumulator, pair_synergy};
use super::{
    Archetype, COLORS, DeckGenError, Entry, Generated, Pool, Rng, colour_letters, shape_name,
};

/// Request parameters for building a single synergistic deck from a [`Pool`].
#[derive(Clone, Debug)]
pub struct DeckBuildRequest<'a> {
    /// Target format shape (`Constructed`, `Highlander`, or `Commander`).
    pub shape: Shape,
    /// Target macro archetype (`Aggro`, `Midrange`, `Control`, or `Ramp`).
    pub archetype: Archetype,
    /// Deterministic RNG seed.
    pub seed: u64,
    /// Optional build-around anchor card (commander, spell, or non-basic land).
    pub anchor: Option<CardIndex>,
    /// Optional explicit colour identity override for `Constructed` / `Highlander`.
    pub forced_colors: Option<ColorSet>,
    /// Optional per-card coverage deficit (`target_coverage - current_coverage`)
    /// to prioritize under-covered cards during candidate scoring.
    pub deficits: Option<&'a BTreeMap<CardIndex, usize>>,
    /// Optional per-card corpus usage count to penalize over-represented filler cards.
    pub usage_counts: Option<&'a BTreeMap<CardIndex, usize>>,
}

impl DeckBuildRequest<'_> {
    /// Creates a standard [`DeckBuildRequest`] for `(shape, archetype, seed)`.
    #[must_use]
    pub const fn new(shape: Shape, archetype: Archetype, seed: u64) -> Self {
        Self {
            shape,
            archetype,
            seed,
            anchor: None,
            forced_colors: None,
            deficits: None,
            usage_counts: None,
        }
    }
}

/// Configuration for [`build_coverage_corpus`].
#[derive(Clone, Debug)]
pub struct CoverageConfig {
    /// Minimum number of decks every working card in the pool must appear in (`k >= 3`).
    pub min_coverage: usize,
    /// Maximum allowed multiset Jaccard similarity against any existing or newly
    /// generated deck (`< 0.85`).
    pub max_jaccard: f64,
    /// Base deterministic seed for corpus generation.
    pub base_seed: u64,
    /// Safety cap on the number of generated decks.
    pub max_new_decks: usize,
}

impl Default for CoverageConfig {
    fn default() -> Self {
        Self {
            min_coverage: 3,
            max_jaccard: 0.82,
            base_seed: 2026_1025,
            max_new_decks: 1000,
        }
    }
}

/// Summary report of a [`build_coverage_corpus`] run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageReport {
    /// Total distinct cards in the target pool.
    pub pool_cards: usize,
    /// Target minimum deck count per card (`k`).
    pub target_min_coverage: usize,
    /// Number of pool cards that had `< k` decks before generation.
    pub under_covered_before: usize,
    /// Number of pool cards that had `0` decks before generation.
    pub zero_covered_before: usize,
    /// Number of pool cards that still have `< k` decks after generation.
    pub under_covered_after: usize,
    /// Minimum deck count across all non-basic cards in the pool after generation.
    pub min_coverage_after: usize,
    /// Number of new synergistic decks generated.
    pub decks_generated: usize,
}

/// Builds one synergistic, curve-balanced, pip-accurate deck according to `req`.
///
/// # Errors
/// Returns an error if the pool has too few spells in the chosen colours or if
/// the resulting deck fails [`crate::formats::check`].
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn build_synergistic(
    pool: &Pool,
    req: &DeckBuildRequest<'_>,
) -> Result<Generated, DeckGenError> {
    let mut rng = Rng::new(req.seed ^ 0x5eed_dec5);
    let shape = req.shape;
    let archetype = req.archetype;

    let pool_by_id: BTreeMap<CardIndex, &Entry> =
        pool.entries.iter().map(|e| (e.card, e)).collect();
    let anchor_entry = req.anchor.and_then(|c| pool_by_id.get(&c).copied());
    let anchor_with_partner_ci = anchor_entry.map_or(ColorSet::EMPTY, |ae| {
        ae.profile
            .named_partners
            .iter()
            .filter_map(|p| pool_by_id.get(p).map(|pe| pe.identity))
            .fold(ae.identity, ColorSet::union)
    });

    // 1. Determine leaders (for Commander) and deck colour identity
    let mut leaders = Vec::new();
    let colors = if shape == Shape::Commander {
        if let Some(ae) = anchor_entry
            && ae.leader
            && !ae.identity.is_empty()
        {
            leaders.push(ae.card);
            ae.identity
        } else {
            let req_ci = anchor_with_partner_ci.union(req.forced_colors.unwrap_or(ColorSet::EMPTY));
            let candidates: Vec<&Entry> = pool
                .entries
                .iter()
                .filter(|e| {
                    e.leader
                        && !e.identity.is_empty()
                        && req_ci.difference(e.identity).is_empty()
                        && (anchor_entry.map(|a| a.card) != Some(CRYPTIC_SPIRES)
                            || e.identity.len() >= 2)
                })
                .collect();
            if candidates.is_empty() {
                return Err(DeckGenError(
                    "no working commander covers required identity".to_string(),
                ));
            }
            let leader = choose_commander(&candidates, anchor_entry, req.deficits, &mut rng);
            leaders.push(leader.card);
            leader.identity
        }
    } else if let Some(fc) = req.forced_colors {
        expand_colors(pool, fc.union(anchor_with_partner_ci), &mut rng)
    } else if anchor_entry.is_some() {
        expand_colors(pool, anchor_with_partner_ci, &mut rng)
    } else {
        choose_random_colors(pool, &mut rng)
    };

    let size = match shape {
        Shape::Constructed => {
            if req.deficits.is_none() && rng.unit() < 0.25 {
                61 + rng.below(15)
            } else {
                60
            }
        }
        Shape::Highlander | Shape::Commander => formats::SINGLETON_SIZE,
    };
    let library = size - leaders.len();
    let lands = ((size as f64) * archetype.land_share()).round() as usize;
    let spells = library - lands;
    let singleton = shape != Shape::Constructed;

    let fits = |e: &&Entry| e.identity.difference(colors).is_empty() && !leaders.contains(&e.card);
    let spell_pool: Vec<&Entry> = pool
        .entries
        .iter()
        .filter(|e| !e.land)
        .filter(fits)
        .collect();
    if spell_pool.len() < 12 {
        return Err(DeckGenError(format!(
            "only {} working spells in {}",
            spell_pool.len(),
            colour_letters(colors)
        )));
    }

    // 2. Initialize DeckThemeAccumulator with leaders and/or anchor spell
    let mut theme = DeckThemeAccumulator::default();
    let mut wanted_partners: BTreeSet<CardIndex> = BTreeSet::new();
    for &l in &leaders {
        if let Some(le) = pool_by_id.get(&l) {
            theme.add(&le.profile, 1);
            wanted_partners.extend(le.profile.named_partners.iter().copied());
        }
    }

    let mut main: Vec<CardIndex> = Vec::with_capacity(library);
    let mut taken: BTreeSet<CardIndex> = BTreeSet::new();
    let mut protected: BTreeSet<CardIndex> = BTreeSet::new();

    if let Some(ae) = anchor_entry {
        protected.insert(ae.card);
        wanted_partners.extend(ae.profile.named_partners.iter().copied());
        if !ae.land && !leaders.contains(&ae.card) && fits(&ae) {
            taken.insert(ae.card);
            let copies = if singleton {
                1
            } else if req.deficits.is_some() {
                2
            } else {
                3 + rng.below(2)
            };
            let add_n = copies.min(spells);
            for _ in 0..add_n {
                main.push(ae.card);
            }
            theme.add(&ae.profile, add_n as u16);
        }
    } else if leaders.is_empty() {
        // Seed a thematic ON-COLOUR build-around spell so standalone Constructed/Highlander
        // decks form a coherent mechanical cluster in their own colours.
        let thematic_colored: Vec<&Entry> = spell_pool
            .iter()
            .copied()
            .filter(|e| {
                !e.identity.is_empty()
                    && e.profile.roles != 0
                    && e.profile.pips[5] == 0
                    && !e.profile.has_external_named_dependency
            })
            .collect();
        let seed_pool = if thematic_colored.is_empty() {
            &spell_pool
        } else {
            &thematic_colored
        };
        let seed_card = seed_pool[rng.below(seed_pool.len())];
        taken.insert(seed_card.card);
        protected.insert(seed_card.card);
        wanted_partners.extend(seed_card.profile.named_partners.iter().copied());
        let copies = if singleton { 1 } else { 3 + rng.below(2) };
        let add_n = copies.min(spells);
        for _ in 0..add_n {
            main.push(seed_card.card);
        }
        theme.add(&seed_card.profile, add_n as u16);
    }

    let anchor_is_artifact_or_colorless = anchor_entry.is_some_and(|ae| {
        ae.identity.is_empty() || (ae.profile.roles & super::profile::ROLE_ARTIFACT) != 0
    }) || leaders.iter().any(|l| {
        pool_by_id.get(l).is_some_and(|le| {
            le.identity.is_empty() || (le.profile.roles & super::profile::ROLE_ARTIFACT) != 0
        })
    });
    let anchor_uses_c_mana = anchor_entry.is_some_and(|ae| ae.profile.pips[5] > 0);
    let max_colorless_spells = if anchor_is_artifact_or_colorless || colors.is_empty() {
        spells
    } else {
        (spells / 7).max(3)
    };
    let mut colorless_spells_added = main
        .iter()
        .filter(|c| pool_by_id.get(c).is_some_and(|e| e.identity.is_empty()))
        .count();

    // 3. Pre-partition spell_pool by mana-value bucket (0..=7+) and deficit status
    let mut by_bucket: [Vec<&Entry>; 8] = std::array::from_fn(|_| Vec::new());
    let mut deficit_spells: Vec<&Entry> = Vec::new();
    for &e in &spell_pool {
        by_bucket[(e.mana_value as usize).min(7)].push(e);
        if req
            .deficits
            .and_then(|m| m.get(&e.card).copied())
            .unwrap_or(0)
            > 0
        {
            deficit_spells.push(e);
        }
    }

    // Constructive synergistic spell selection
    let curve = archetype.curve();
    let mut guard = 0;
    while main.len() < spells {
        guard += 1;
        if guard > 10 * library {
            return Err(DeckGenError(format!(
                "the pool in {} cannot fill {spells} spells",
                colour_letters(colors)
            )));
        }
        // Choose target CMC bucket weighted by remaining curve demand
        let bucket_weights: [f64; 8] = std::array::from_fn(|b| {
            if by_bucket[b].is_empty() {
                0.0
            } else {
                let target_b = (spells as f64) * curve[b];
                let have_b = f64::from(theme.curve_counts[b]);
                let rem = (target_b - have_b).max(0.08);
                rem * (curve[b] + 0.02)
            }
        });
        let target_b = if bucket_weights.iter().sum::<f64>() > 0.0 {
            rng.weighted(&bucket_weights)
        } else {
            rng.below(8)
        };

        let has_untaken_partner = wanted_partners.iter().any(|p| {
            !taken.contains(p) && pool_by_id.get(p).is_some_and(|pe| !pe.land && fits(pe))
        });
        let candidates: &[&Entry] = if has_untaken_partner {
            &spell_pool
        } else if !deficit_spells.iter().all(|e| taken.contains(&e.card)) {
            &deficit_spells
        } else if by_bucket[target_b].iter().any(|e| !taken.contains(&e.card)) {
            &by_bucket[target_b]
        } else {
            &spell_pool
        };

        let mut top_cands: Vec<(f64, &Entry)> = Vec::with_capacity(12);
        for &e in candidates {
            if taken.contains(&e.card) {
                continue;
            }
            let def_val = req
                .deficits
                .and_then(|m| m.get(&e.card).copied())
                .unwrap_or(0);
            let is_wanted_partner = wanted_partners.contains(&e.card);
            // Cards with an external `Filter::Named` dependency:
            // - If any named partner exists in `pool_by_id`, require that all available
            //   partners fit the deck's colours so they can be co-selected together.
            // - If no named partner exists in `pool_by_id`, only allow when `def_val > 0`.
            if e.profile.has_external_named_dependency && !is_wanted_partner {
                let avail_partners: Vec<&Entry> = e
                    .profile
                    .named_partners
                    .iter()
                    .filter_map(|p| pool_by_id.get(p).copied())
                    .filter(|pe| !pe.land)
                    .collect();
                if avail_partners.is_empty() {
                    if def_val == 0 {
                        continue;
                    }
                } else if !avail_partners.iter().all(&fits) {
                    continue;
                }
            }
            // Cards requiring explicit {C} mana are only allowed when the anchor uses {C}
            // or when the card itself has a coverage deficit.
            if e.profile.pips[5] > 0 && !anchor_uses_c_mana && def_val == 0 && !is_wanted_partner {
                continue;
            }
            // Enforce colourless spell cap unless the card has a coverage deficit
            if e.identity.is_empty()
                && colorless_spells_added >= max_colorless_spells
                && def_val == 0
                && !is_wanted_partner
            {
                continue;
            }
            let syn = theme.synergy_score(&e.profile);
            let quota = theme.quota_bonus(&e.profile, archetype, spells);
            let curve_b = theme.curve_bonus(&e.profile, archetype, spells);
            let color_bonus = if !e.identity.is_empty() {
                1.8
            } else if anchor_is_artifact_or_colorless {
                0.4
            } else {
                -1.2
            };
            let cov_bonus = if def_val > 0 {
                8.5 * (def_val as f64)
            } else {
                0.0
            };
            let partner_bonus = if is_wanted_partner { 25.0 } else { 0.0 };
            let sat_penalty = if def_val == 0 && !is_wanted_partner {
                req.usage_counts
                    .and_then(|m| m.get(&e.card).copied())
                    .map_or(0.0, |u| -0.95 * (u.saturating_sub(2) as f64))
            } else {
                0.0
            };
            let jitter = 0.85 * rng.unit();
            let total = 2.0 * syn
                + 1.5 * quota
                + 1.4 * curve_b
                + color_bonus
                + cov_bonus
                + partner_bonus
                + sat_penalty
                + jitter;

            if top_cands.len() < 8 {
                top_cands.push((total, e));
                top_cands.sort_by(|a, b| b.0.total_cmp(&a.0));
            } else if total > top_cands[7].0 {
                top_cands[7] = (total, e);
                top_cands.sort_by(|a, b| b.0.total_cmp(&a.0));
            }
        }
        // Fallback if strict colourless/{C} filter left `top_cands` empty in `target_b`
        if top_cands.is_empty() {
            if let Some(&fallback) = spell_pool
                .iter()
                .find(|e| !taken.contains(&e.card) && !e.profile.has_external_named_dependency)
            {
                top_cands.push((0.0, fallback));
            } else if let Some(&fallback) = spell_pool.iter().find(|e| !taken.contains(&e.card)) {
                top_cands.push((0.0, fallback));
            } else {
                break;
            }
        }
        let max_s = top_cands[0].0;
        let softmax_weights: Vec<f64> = top_cands
            .iter()
            .map(|(s, _)| ((s - max_s) / 0.90).exp())
            .collect();
        let chosen = top_cands[rng.weighted(&softmax_weights)].1;

        taken.insert(chosen.card);
        wanted_partners.extend(chosen.profile.named_partners.iter().copied());
        let has_deficit = req
            .deficits
            .and_then(|m| m.get(&chosen.card).copied())
            .unwrap_or(0)
            > 0;
        if has_deficit || wanted_partners.contains(&chosen.card) {
            protected.insert(chosen.card);
        }
        let copies = if singleton {
            1
        } else if has_deficit {
            // In coverage mode, use 2 copies so each Constructed deck packs ~18 distinct spells
            2
        } else {
            [4, 4, 3, 2][rng.below(4)]
        };
        let add_n = copies.min(spells - main.len());
        for _ in 0..add_n {
            main.push(chosen.card);
        }
        if chosen.identity.is_empty() {
            colorless_spells_added += add_n;
        }
        theme.add(&chosen.profile, add_n as u16);
    }

    if main.len() < spells {
        return Err(DeckGenError(format!(
            "the pool in {} filled only {}/{spells} spells",
            colour_letters(colors),
            main.len()
        )));
    }

    // 4. Local 1-swap refinement pass: swap weak non-protected spells for stronger
    //    same-CMC synergy partners in the pool.
    refine_spells_1swap(
        &mut main,
        &mut taken,
        &protected,
        &mut theme,
        &pool_by_id,
        &by_bucket,
        archetype,
        spells,
    );

    // 5. Build pip-accurate Frank Karsten mana base (including anchor land if any)
    let nonbasics: Vec<&Entry> = pool
        .entries
        .iter()
        .filter(|e| e.land && !e.basic)
        .filter(|e| shape != Shape::Commander || colors.len() >= 2 || e.card != CRYPTIC_SPIRES)
        .filter(fits)
        .collect();

    let mut land_cards: Vec<CardIndex> = Vec::with_capacity(lands);
    if let Some(ae) = anchor_entry
        && ae.land
        && !ae.basic
        && fits(&ae)
        && (shape != Shape::Commander || colors.len() >= 2 || ae.card != CRYPTIC_SPIRES)
    {
        taken.insert(ae.card);
        let copies = if singleton { 1 } else { 2 };
        for _ in 0..copies.min(lands) {
            land_cards.push(ae.card);
        }
    }

    let rem_lands = lands.saturating_sub(land_cards.len());
    if rem_lands > 0 {
        let mut extra_lands = build_manabase(
            &pool_by_id,
            &nonbasics,
            &main,
            &leaders,
            rem_lands,
            size,
            colors,
            singleton,
            &mut taken,
            req.deficits,
            &mut rng,
        )?;
        land_cards.append(&mut extra_lands);
    }

    main.extend(land_cards);
    main.sort();

    let generated = Generated {
        name: format!(
            "{} {} {} #{}",
            shape_name(shape),
            archetype.name(),
            colour_letters(colors),
            req.seed
        ),
        shape,
        archetype,
        colors,
        main,
        leaders,
    };
    let problems = formats::check(shape, &generated.main, &generated.leaders);
    if !problems.is_empty() {
        return Err(DeckGenError(format!(
            "a generated {} deck is not one: {problems:?}",
            shape_name(shape)
        )));
    }
    Ok(generated)
}

#[allow(clippy::cast_precision_loss)]
fn choose_commander<'a>(
    candidates: &[&'a Entry],
    anchor: Option<&Entry>,
    deficits: Option<&BTreeMap<CardIndex, usize>>,
    rng: &mut Rng,
) -> &'a Entry {
    if deficits.is_none() && anchor.is_none() {
        return candidates[rng.below(candidates.len())];
    }
    let mut best: Option<(f64, &'a Entry)> = None;
    for &cand in candidates {
        let def_val = deficits
            .and_then(|m| m.get(&cand.card).copied())
            .unwrap_or(0);
        let mut score = (def_val as f64) * 10.0;
        if let Some(ae) = anchor {
            score += 2.5 * pair_synergy(&cand.profile, &ae.profile);
            // Prefer 2–3 colour commanders for 1–3 colour anchors so we build
            // cohesive guild/shard/wedge decks instead of 5-colour soup.
            let target_cols = ae.identity.len().clamp(2, 3);
            if def_val == 0 {
                let extra_cols = cand.identity.len().saturating_sub(target_cols);
                score -= 2.5 * f64::from(extra_cols);
                if cand.identity.len() == target_cols {
                    score += 1.5;
                }
            }
        }
        score += 0.65 * rng.unit();
        if best.is_none_or(|(b_score, _)| score > b_score) {
            best = Some((score, cand));
        }
    }
    best.map_or(candidates[0], |(_, e)| e)
}

#[allow(clippy::cast_precision_loss)]
fn choose_random_colors(pool: &Pool, rng: &mut Rng) -> ColorSet {
    let count = [1, 2, 2, 2, 3][rng.below(5)];
    let mut chosen = ColorSet::EMPTY;
    let all = [0_u8, 1, 2, 3, 4];
    while chosen.len() < count {
        let weights: Vec<f64> = all
            .iter()
            .map(|i| {
                if chosen.bits() >> i & 1 == 1 {
                    0.0
                } else {
                    pool.entries
                        .iter()
                        .filter(|e| !e.land && e.identity.bits() >> i & 1 == 1)
                        .count() as f64
                }
            })
            .collect();
        let pick = all[rng.weighted(&weights)];
        chosen = chosen.union(ColorSet::of(COLORS[usize::from(pick)]));
    }
    chosen
}

fn expand_colors(pool: &Pool, base: ColorSet, rng: &mut Rng) -> ColorSet {
    let mut chosen = base;
    let target_len = if chosen.is_empty() || (chosen.len() == 1 && rng.unit() < 0.65) {
        2
    } else {
        usize::from(chosen.len())
    };
    while usize::from(chosen.len()) < target_len {
        let c = COLORS[rng.below(5)];
        chosen = chosen.union(ColorSet::of(c));
    }
    // Ensure there are enough spells in `chosen`; if not, add another colour
    while pool
        .entries
        .iter()
        .filter(|e| !e.land && e.identity.difference(chosen).is_empty())
        .count()
        < 60
        && chosen.len() < 5
    {
        for c in COLORS {
            if !chosen.contains(c) {
                chosen = chosen.union(ColorSet::of(c));
                break;
            }
        }
    }
    chosen
}

#[allow(clippy::too_many_arguments, clippy::cast_possible_truncation)]
fn refine_spells_1swap(
    main: &mut [CardIndex],
    taken: &mut BTreeSet<CardIndex>,
    protected: &BTreeSet<CardIndex>,
    theme: &mut DeckThemeAccumulator,
    pool_by_id: &BTreeMap<CardIndex, &Entry>,
    by_bucket: &[Vec<&Entry>; 8],
    archetype: Archetype,
    target_spells: usize,
) {
    // Count distinct non-protected spells currently in `main`
    let mut counts: BTreeMap<CardIndex, usize> = BTreeMap::new();
    for &c in main.iter() {
        *counts.entry(c).or_insert(0) += 1;
    }
    let swappable: Vec<(CardIndex, usize)> = counts
        .into_iter()
        .filter(|(c, _)| !protected.contains(c))
        .collect();
    if swappable.is_empty() {
        return;
    }

    // Score each swappable card and try replacing the weakest 6 cards
    let mut weakest: Vec<(f64, CardIndex, usize)> = swappable
        .into_iter()
        .filter_map(|(cid, cnt)| {
            let e = pool_by_id.get(&cid)?;
            let s = 2.2 * theme.synergy_score(&e.profile)
                + 1.5 * theme.quota_bonus(&e.profile, archetype, target_spells);
            Some((s, cid, cnt))
        })
        .collect();
    weakest.sort_by(|a, b| a.0.total_cmp(&b.0));

    for (_, old_cid, cnt) in weakest.into_iter().take(6) {
        let Some(&old_entry) = pool_by_id.get(&old_cid) else {
            continue;
        };
        theme.remove(&old_entry.profile, cnt as u16);
        let base_score = 2.2 * theme.synergy_score(&old_entry.profile)
            + 1.5 * theme.quota_bonus(&old_entry.profile, archetype, target_spells)
            + 1.4 * theme.curve_bonus(&old_entry.profile, archetype, target_spells);

        let old_bucket = (old_entry.mana_value as usize).min(7);
        let mut best_rep: Option<(f64, &Entry)> = None;
        for &cand in &by_bucket[old_bucket] {
            if taken.contains(&cand.card)
                || cand.profile.has_external_named_dependency
                || (cand.identity.is_empty() && !old_entry.identity.is_empty())
                || (cand.profile.pips[5] > 0 && old_entry.profile.pips[5] == 0)
            {
                continue;
            }
            let cand_score = 2.2 * theme.synergy_score(&cand.profile)
                + 1.5 * theme.quota_bonus(&cand.profile, archetype, target_spells)
                + 1.4 * theme.curve_bonus(&cand.profile, archetype, target_spells);
            if cand_score > base_score + 0.4 && best_rep.is_none_or(|(b_s, _)| cand_score > b_s) {
                best_rep = Some((cand_score, cand));
            }
        }

        if let Some((_, new_entry)) = best_rep {
            taken.remove(&old_cid);
            taken.insert(new_entry.card);
            for slot in main.iter_mut() {
                if *slot == old_cid {
                    *slot = new_entry.card;
                }
            }
            theme.add(&new_entry.profile, cnt as u16);
        } else {
            theme.add(&old_entry.profile, cnt as u16);
        }
    }
}

/// Generates a suite of synergistic, rule-valid decks from `pool` until every
/// card in `pool` appears in at least `config.min_coverage` decks (`k >= 3`),
/// taking into account `existing_multisets` and enforcing `J < config.max_jaccard`.
///
/// # Errors
/// Returns an error if a deck cannot be constructed from `pool`.
pub fn build_coverage_corpus(
    pool: &Pool,
    existing_multisets: &[BTreeMap<CardIndex, u32>],
    config: &CoverageConfig,
) -> Result<(Vec<Generated>, CoverageReport), DeckGenError> {
    build_coverage_corpus_scoped(pool, None, existing_multisets, &[], config)
}

/// Like [`build_coverage_corpus`], but optionally restricts the required
/// `min_coverage` target to `target_cards` (while drawing partner cards from all
/// of `pool`) and additionally avoids near-duplicates against `avoid_multisets`
/// (e.g. decks in other splits).
///
/// # Errors
/// Returns an error if a deck cannot be constructed from `pool`.
#[allow(clippy::too_many_lines)]
pub fn build_coverage_corpus_scoped(
    pool: &Pool,
    target_cards: Option<&BTreeSet<CardIndex>>,
    existing_multisets: &[BTreeMap<CardIndex, u32>],
    avoid_multisets: &[BTreeMap<CardIndex, u32>],
    config: &CoverageConfig,
) -> Result<(Vec<Generated>, CoverageReport), DeckGenError> {
    let mut coverage: BTreeMap<CardIndex, usize> = pool
        .entries
        .iter()
        .filter(|e| target_cards.is_none_or(|t| t.contains(&e.card)))
        .map(|e| (e.card, 0))
        .collect();
    for ms in existing_multisets {
        for &cid in ms.keys() {
            if let Some(cnt) = coverage.get_mut(&cid) {
                *cnt += 1;
            }
        }
    }

    let under_covered_before = coverage
        .iter()
        .filter(|(c, n)| !formats::is_basic_land(**c) && **n < config.min_coverage)
        .count();
    let zero_covered_before = coverage
        .iter()
        .filter(|(c, n)| !formats::is_basic_land(**c) && **n == 0)
        .count();

    let pool_by_id: BTreeMap<CardIndex, &Entry> =
        pool.entries.iter().map(|e| (e.card, e)).collect();

    // Check which colour identities have at least one eligible Commander in `pool`
    let commander_identities: Vec<ColorSet> = pool
        .entries
        .iter()
        .filter(|e| e.leader && !e.identity.is_empty())
        .map(|e| e.identity)
        .collect();

    let mut sim_index = MultisetSimilarityIndex::from_slice(existing_multisets);
    for ms in avoid_multisets {
        sim_index.add(ms);
    }
    let mut generated_decks: Vec<Generated> = Vec::new();
    let mut seed_cursor = config.base_seed;

    while generated_decks.len() < config.max_new_decks {
        // Compute current deficits (`min_coverage - count`)
        let mut deficits: BTreeMap<CardIndex, usize> = BTreeMap::new();
        for (&cid, &cnt) in &coverage {
            if !formats::is_basic_land(cid) && cnt < config.min_coverage {
                deficits.insert(cid, config.min_coverage - cnt);
            }
        }
        if deficits.is_empty() {
            break;
        }

        // Select the highest-priority under-covered anchor card `u*`:
        // 1. Largest deficit (`0` coverage first, then `1`, then `2`)
        // 2. Eligible Commanders first (each Commander deck covers 1 leader + ~60 cards)
        // 3. Multi-colour cards before mono-colour / colourless cards
        // 4. Cards with active mechanical roles before vanilla cards
        let Some((&anchor_cid, _)) = deficits.iter().max_by_key(|&(&cid, &def_val)| {
            let e = pool_by_id[&cid];
            let is_cmd = u8::from(e.leader && !e.identity.is_empty());
            let has_roles = u8::from(e.profile.roles != 0);
            (
                def_val,
                is_cmd,
                e.identity.len(),
                has_roles,
                !e.land,
                e.mana_value,
                u32::MAX - cid.get(),
            )
        }) else {
            break;
        };

        let anchor_entry = pool_by_id[&anchor_cid];
        let anchor_ci = anchor_entry
            .profile
            .named_partners
            .iter()
            .filter_map(|p| pool_by_id.get(p).map(|pe| pe.identity))
            .fold(anchor_entry.identity, ColorSet::union);
        let pref_arch = anchor_entry.profile.preferred_archetype();
        let archetype = if pref_arch == Archetype::Midrange && anchor_entry.profile.roles == 0 {
            Archetype::ALL[generated_decks.len() % Archetype::ALL.len()]
        } else {
            pref_arch
        };
        let can_use_commander = commander_identities.iter().any(|&ci| {
            anchor_ci.difference(ci).is_empty() && (anchor_cid != CRYPTIC_SPIRES || ci.len() >= 2)
        });

        let preferred_shapes: [Shape; 3] =
            if anchor_entry.leader && !anchor_entry.identity.is_empty() {
                [Shape::Commander, Shape::Highlander, Shape::Constructed]
            } else if !can_use_commander {
                [Shape::Highlander, Shape::Constructed, Shape::Highlander]
            } else {
                match generated_decks.len() % 3 {
                    0 => [Shape::Highlander, Shape::Commander, Shape::Constructed],
                    1 => [Shape::Commander, Shape::Highlander, Shape::Constructed],
                    _ => [Shape::Constructed, Shape::Highlander, Shape::Commander],
                }
            };

        let mut accepted: Option<(Generated, BTreeMap<CardIndex, u32>)> = None;
        for attempt in 0..12_u64 {
            seed_cursor = seed_cursor.wrapping_add(1);
            let shape = preferred_shapes[(attempt as usize) % preferred_shapes.len()];
            let forced_colors = if shape == Shape::Commander {
                None
            } else {
                Some(choose_broad_deficit_colors(
                    pool, anchor_ci, &deficits, attempt,
                ))
            };
            let req = DeckBuildRequest {
                shape,
                archetype,
                seed: seed_cursor,
                anchor: Some(anchor_cid),
                forced_colors,
                deficits: Some(&deficits),
                usage_counts: Some(&coverage),
            };
            let Ok(deck) = build_synergistic(pool, &req) else {
                continue;
            };
            if !deck.distinct().contains(&anchor_cid) {
                continue;
            }
            let ms = compute_multiset(&deck.main, &deck.leaders);
            let too_similar = sim_index.max_jaccard(&ms) >= config.max_jaccard;
            if too_similar && attempt < 11 {
                continue;
            }
            accepted = Some((deck, ms));
            break;
        }

        if let Some((deck, ms)) = accepted {
            for &cid in ms.keys() {
                if let Some(cnt) = coverage.get_mut(&cid) {
                    *cnt += 1;
                }
            }
            sim_index.add(&ms);
            generated_decks.push(deck);
        } else {
            seed_cursor = seed_cursor.wrapping_add(1);
            let req = DeckBuildRequest {
                shape: Shape::Constructed,
                archetype,
                seed: seed_cursor,
                anchor: Some(anchor_cid),
                forced_colors: Some(ColorSet::ALL),
                deficits: Some(&deficits),
                usage_counts: Some(&coverage),
            };
            let deck = build_synergistic(pool, &req)?;
            let ms = compute_multiset(&deck.main, &deck.leaders);
            for &cid in ms.keys() {
                if let Some(cnt) = coverage.get_mut(&cid) {
                    *cnt += 1;
                }
            }
            sim_index.add(&ms);
            generated_decks.push(deck);
        }
    }

    let under_covered_after = coverage
        .iter()
        .filter(|(c, n)| !formats::is_basic_land(**c) && **n < config.min_coverage)
        .count();
    let min_coverage_after = coverage
        .iter()
        .filter(|(c, _)| !formats::is_basic_land(**c))
        .map(|(_, &n)| n)
        .min()
        .unwrap_or(0);

    let report = CoverageReport {
        pool_cards: coverage.len(),
        target_min_coverage: config.min_coverage,
        under_covered_before,
        zero_covered_before,
        under_covered_after,
        min_coverage_after,
        decks_generated: generated_decks.len(),
    };

    Ok((generated_decks, report))
}

fn choose_broad_deficit_colors(
    pool: &Pool,
    base: ColorSet,
    deficits: &BTreeMap<CardIndex, usize>,
    attempt: u64,
) -> ColorSet {
    let mut chosen = base;
    let target_len = if base.len() >= 3 {
        usize::from(base.len())
    } else if attempt.is_multiple_of(3) {
        3
    } else {
        2
    };
    // Rank colours by how many under-covered cards have that colour
    let mut color_deficits: Vec<(usize, Color)> = COLORS
        .into_iter()
        .map(|c| {
            let sum: usize = pool
                .entries
                .iter()
                .filter(|e| e.identity.contains(c))
                .filter_map(|e| deficits.get(&e.card).copied())
                .sum();
            (sum, c)
        })
        .collect();
    color_deficits.sort_by_key(|a| std::cmp::Reverse(a.0));

    for (_, c) in color_deficits {
        if usize::from(chosen.len()) >= target_len {
            break;
        }
        chosen = chosen.union(ColorSet::of(c));
    }
    chosen
}
