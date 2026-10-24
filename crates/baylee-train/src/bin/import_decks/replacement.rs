use super::{BTreeMap, CardIndex, ColorSet, SupertypeSet, TypeSet};

/// Checks if a card is a functional replacement candidate for a non-working card in an adapted precon.
pub(super) fn find_functional_replacement(
    orig_name: &str,
    deck_colors: ColorSet,
    copy_limit: u32,
    current_counts: &BTreeMap<CardIndex, u32>,
    working_pool: &[CardIndex],
) -> Option<(CardIndex, &'static str)> {
    // Look up original card in baylee_cards or baylee_cards_index to match its characteristics
    let orig_def = baylee_cards::decks::by_name(orig_name).and_then(baylee_cards::by_index);
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

    // Deterministic tie-breaking hash from original card name
    let mut name_hash = 0xcbf2_9ce4_8422_2325_u64;
    for b in orig_name.bytes() {
        name_hash ^= u64::from(b);
        name_hash = name_hash.wrapping_mul(0x0100_0000_01b3);
    }

    let mut best: Option<((u32, u32, u64), CardIndex, &'static str)> = None;
    for &cand in working_pool {
        let Some(def) = baylee_cards::by_index(cand) else {
            continue;
        };
        let face = &def.faces[0];
        let is_basic =
            face.types.contains(TypeSet::LAND) && face.supertypes.contains(SupertypeSet::BASIC);
        // Never replace a non-land card with a basic land (§14)
        if !target_is_land && face.types.contains(TypeSet::LAND) {
            continue;
        }
        if target_is_land && !face.types.contains(TypeSet::LAND) {
            continue;
        }
        // Must fit within the deck's color identity
        if def.color_identity.union(deck_colors) != deck_colors {
            continue;
        }
        let cur = current_counts.get(&cand).copied().unwrap_or(0);
        if !is_basic && cur >= copy_limit {
            continue;
        }
        // Prefer non-basic lands when replacing a non-basic land if available
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
        let cmc_diff = face.mana_cost.cmc().abs_diff(target_cmc);
        let tie = name_hash.wrapping_add(u64::from(cand.get()).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let score = (type_penalty + ci_penalty, cmc_diff, tie);
        if best.is_none_or(|(b_score, _, _)| score < b_score) {
            best = Some((score, cand, def.name()));
        }
    }
    best.map(|(_, cid, name)| (cid, name))
}
