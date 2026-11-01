//! Frank Karsten hypergeometric mana-base calculator and dual/basic land
//! allocator.

use std::collections::{BTreeMap, BTreeSet};

use crate::decks::basic_lands;
use baylee_core::color::ColorSet;
use baylee_core::ids::CardIndex;

use super::{COLORS, DeckGenError, Entry, Rng};

/// Hypergeometric probability $\mathbb{P}(X \ge k)$ when drawing `n_drawn` cards
/// without replacement from a deck of `deck_size` containing `sources` mana sources.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "Exact floating-point hypergeometric PMF over small integer deck sizes (<= 100)"
)]
pub fn hypergeom_at_least(deck_size: usize, sources: usize, n_drawn: usize, k: usize) -> f64 {
    if k == 0 {
        return 1.0;
    }
    if sources < k || n_drawn < k || deck_size == 0 {
        return 0.0;
    }
    let n_drawn = n_drawn.min(deck_size);
    let max_x = sources.min(n_drawn);
    let mut prob_sum = 0.0_f64;
    for x in k..=max_x {
        let non_sources = deck_size.saturating_sub(sources);
        let needed_non = n_drawn.saturating_sub(x);
        if needed_non > non_sources {
            continue;
        }
        let log_p = ln_choose(sources, x) + ln_choose(non_sources, needed_non)
            - ln_choose(deck_size, n_drawn);
        prob_sum += log_p.exp();
    }
    prob_sum.clamp(0.0, 1.0)
}

#[allow(clippy::cast_precision_loss)]
fn ln_choose(n: usize, k: usize) -> f64 {
    if k > n {
        return f64::NEG_INFINITY;
    }
    let k = k.min(n - k);
    let mut acc = 0.0_f64;
    for i in 1..=k {
        acc += ((n - k + i) as f64).ln() - (i as f64).ln();
    }
    acc
}

/// Computes the minimum number of sources of a colour in a deck of `deck_size`
/// required by Frank Karsten's hypergeometric criterion ($\ge 88\%$ consistency)
/// to cast a spell of `mana_value` with `pips` coloured symbols on curve.
#[must_use]
pub fn karsten_sources_for_spell(deck_size: usize, mana_value: u32, pips: u8) -> usize {
    if pips == 0 {
        return 0;
    }
    let turn = (mana_value as usize).clamp(1, 7);
    let cards_seen = 6 + turn;
    let k = usize::from(pips).min(cards_seen);
    for sources in k..=deck_size {
        if hypergeom_at_least(deck_size, sources, cards_seen, k) >= 0.88 {
            return sources;
        }
    }
    deck_size / 2
}

/// Computes target mana-source counts `[W, U, B, R, G]` across all spells and
/// leaders in the deck.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_color_source_targets(
    pool_by_id: &BTreeMap<CardIndex, &Entry>,
    spells: &[CardIndex],
    leaders: &[CardIndex],
    colors: ColorSet,
    deck_size: usize,
) -> [f64; 5] {
    let mut max_karsten = [0_usize; 5];
    let mut pip_weighted_sum = [0.0_f64; 5];

    for &cid in spells.iter().chain(leaders) {
        let Some(entry) = pool_by_id.get(&cid) else {
            continue;
        };
        let mv = entry.mana_value.max(1);
        for c in 0..5 {
            if (colors.bits() >> c) & 1 == 0 {
                continue;
            }
            let p = entry.profile.pips[c];
            if p > 0 {
                let req = karsten_sources_for_spell(deck_size, mv, p);
                if req > max_karsten[c] {
                    max_karsten[c] = req;
                }
                // Early double/triple-pip spells weigh more heavily than late single-pip spells
                let turn_urgency = 1.0 / (f64::from(mv) + 1.0);
                pip_weighted_sum[c] += f64::from(p) * (1.0 + 2.0 * turn_urgency);
            } else if (entry.identity.bits() >> c) & 1 == 1 {
                // Off-face / activated ability colour identity still needs mana sources
                pip_weighted_sum[c] += 0.75;
                max_karsten[c] = max_karsten[c].max(6);
            }
        }
    }

    let total_pips = pip_weighted_sum.iter().sum::<f64>().max(1.0);
    let approx_lands = (deck_size as f64) * 0.40;
    let mut targets = [0.0_f64; 5];
    for c in 0..5 {
        if (colors.bits() >> c) & 1 == 1 {
            let pip_share = approx_lands * (pip_weighted_sum[c] / total_pips);
            targets[c] = (0.55 * (max_karsten[c] as f64) + 0.45 * pip_share).max(4.0);
        }
    }
    targets
}

/// Selects non-basic fixing/utility lands and basic lands to satisfy the deck's
/// Karsten colour-source targets (`compute_color_source_targets`).
///
/// # Errors
/// Returns an error if the five basic lands are missing from the card registry.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn build_manabase(
    pool_by_id: &BTreeMap<CardIndex, &Entry>,
    nonbasics: &[&Entry],
    spells: &[CardIndex],
    leaders: &[CardIndex],
    lands_needed: usize,
    deck_size: usize,
    colors: ColorSet,
    singleton: bool,
    taken: &mut BTreeSet<CardIndex>,
    deficits: Option<&BTreeMap<CardIndex, usize>>,
    rng: &mut Rng,
) -> Result<Vec<CardIndex>, DeckGenError> {
    let basics = basic_lands();
    let targets = compute_color_source_targets(pool_by_id, spells, leaders, colors, deck_size);
    let mut have_sources = [0.0_f64; 5];
    let mut c_pips_needed = 0_usize;
    let mut have_c_sources = 0_usize;
    let mut wants_snow = false;

    // Count non-land mana sources (dorks/rocks), {C} pip requirements, and snow synergy
    for &cid in spells.iter().chain(leaders) {
        if let Some(e) = pool_by_id.get(&cid) {
            if e.profile.cares_about_snow {
                wants_snow = true;
            }
            if e.profile.pips[5] > 0 {
                let req_c =
                    karsten_sources_for_spell(deck_size, e.mana_value.max(1), e.profile.pips[5]);
                c_pips_needed = c_pips_needed.max(req_c);
            }
            if e.profile.is_ramp() {
                for (c, have) in have_sources.iter_mut().enumerate() {
                    if (e.profile.produced_colors.bits() >> c) & 1 == 1 {
                        *have += 0.4;
                    }
                }
                if e.profile.produces_colorless {
                    have_c_sources += 1;
                }
            }
        }
    }

    let mut land_cards: Vec<CardIndex> = Vec::with_capacity(lands_needed);
    let nonbasic_share = if c_pips_needed > 0 {
        0.50
    } else if colors.len() > 1 {
        0.38
    } else {
        0.16
    };
    let nonbasic_target = ((lands_needed as f64) * nonbasic_share).round() as usize;
    let max_tapped_lands = (lands_needed / 4).max(2);
    let mut tapped_count = 0_usize;

    // Score and rank available non-basic lands
    let mut scored_nonbasics: Vec<(f64, &Entry)> = nonbasics
        .iter()
        .filter(|e| !taken.contains(&e.card))
        .map(|&e| {
            let mut score = 0.0_f64;
            let mut colors_fixed = 0_usize;
            for (c, &target_c) in targets.iter().enumerate() {
                if (colors.bits() >> c) & 1 == 1 && (e.profile.produced_colors.bits() >> c) & 1 == 1
                {
                    colors_fixed += 1;
                    score += (target_c * 0.12).min(2.0);
                }
            }
            if colors_fixed >= 2 {
                score += 2.8 * ((colors_fixed - 1) as f64);
            } else if colors_fixed == 1 {
                score += 0.6;
            } else if c_pips_needed > 0 && e.profile.produces_colorless {
                score += 4.5;
            } else {
                // Pure utility colourless land when no {C} pips are needed: keep scarce
                score += if colors.len() <= 1 { 0.3 } else { -1.2 };
            }
            if e.profile.enters_tapped {
                score -= if colors_fixed >= 2 { 0.35 } else { 1.1 };
            }
            if let Some(def_map) = deficits
                && let Some(&def_val) = def_map.get(&e.card)
                && def_val > 0
            {
                score += 6.5 * (def_val as f64);
            }
            score += 0.45 * rng.unit();
            (score, e)
        })
        .collect();

    scored_nonbasics.sort_by(|a, b| b.0.total_cmp(&a.0));

    for (score, e) in scored_nonbasics {
        if land_cards.len() >= nonbasic_target {
            break;
        }
        let has_deficit = deficits
            .and_then(|m| m.get(&e.card))
            .is_some_and(|&d| d > 0);
        if score < 0.0
            && !has_deficit
            && !(c_pips_needed > have_c_sources && e.profile.produces_colorless)
        {
            continue;
        }
        if e.profile.enters_tapped && tapped_count >= max_tapped_lands && !has_deficit {
            continue;
        }
        if taken.contains(&e.card) {
            continue;
        }
        taken.insert(e.card);
        let copies = if singleton {
            1
        } else if has_deficit || (c_pips_needed > have_c_sources && e.profile.produces_colorless) {
            2 + rng.below(3)
        } else {
            1 + rng.below(3)
        };
        let add_n = copies.min(nonbasic_target - land_cards.len());
        for _ in 0..add_n {
            land_cards.push(e.card);
            if e.profile.enters_tapped {
                tapped_count += 1;
            }
            if e.profile.produces_colorless {
                have_c_sources += 1;
            }
            for (c, have) in have_sources.iter_mut().enumerate() {
                if (e.profile.produced_colors.bits() >> c) & 1 == 1 {
                    *have += 1.0;
                }
            }
        }
    }

    let snow_basics = [
        baylee_core::generated::index::SNOW_COVERED_PLAINS,
        baylee_core::generated::index::SNOW_COVERED_ISLAND,
        baylee_core::generated::index::SNOW_COVERED_SWAMP,
        baylee_core::generated::index::SNOW_COVERED_MOUNTAIN,
        baylee_core::generated::index::SNOW_COVERED_FOREST,
    ];

    // Allocate basic lands to satisfy remaining Karsten colour-source deficits
    while land_cards.len() < lands_needed {
        let best_deficit = (0..5_usize)
            .filter(|&c| (colors.bits() >> c) & 1 == 1)
            .max_by(|&a, &b| {
                (targets[a] - have_sources[a]).total_cmp(&(targets[b] - have_sources[b]))
            });
        let colour = if let Some(c) = best_deficit
            && targets[c] - have_sources[c] > 0.25
        {
            c
        } else {
            let weights: Vec<f64> = (0..5_usize)
                .map(|c| {
                    if (colors.bits() >> c) & 1 == 1 {
                        targets[c].max(1.0)
                    } else {
                        0.0
                    }
                })
                .collect();
            if weights.iter().sum::<f64>() > 0.0 {
                rng.weighted(&weights)
            } else {
                let _ = COLORS;
                rng.below(5)
            }
        };
        let snow_cid = snow_basics[colour];
        let basic = if wants_snow && pool_by_id.contains_key(&snow_cid) {
            snow_cid
        } else {
            basics[colour]
                .ok_or_else(|| DeckGenError("the five basic lands are in the pool".to_string()))?
        };
        land_cards.push(basic);
        have_sources[colour] += 1.0;
    }

    Ok(land_cards)
}
