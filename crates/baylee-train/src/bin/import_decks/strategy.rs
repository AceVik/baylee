use super::{BTreeMap, CardDef, CardIndex, Color, ColorSet, TypeSet, encode_card_face};

pub(super) fn color_string(cs: ColorSet) -> String {
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

/// Classifies a deck's primary archetype and multi-label strategy tags (§12)
/// from its actual card definitions (`baylee_cards::by_index`).
#[allow(
    clippy::too_many_lines,
    reason = "Single pass over card definitions followed by independent strategy tags"
)]
#[allow(
    clippy::cast_precision_loss,
    reason = "Approximate archetype heuristics on bounded deck counts"
)]
pub(super) fn classify_deck_strategy(
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
    let mut subtype_counts: BTreeMap<u16, usize> = BTreeMap::new();

    for &cid in main.iter().chain(commanders) {
        let Some(def) = baylee_cards::by_index(cid) else {
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
                low_cmc_cmc_inc(&mut low_cmc_creatures);
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

fn low_cmc_cmc_inc(cnt: &mut usize) {
    *cnt += 1;
}

fn inspect_def_mechanics(
    def: &CardDef,
    token_makers: &mut usize,
    counter_synergy: &mut usize,
    graveyard_reanim: &mut usize,
    sacrifice_cards: &mut usize,
    blink_etb: &mut usize,
    ramp_mana: &mut usize,
) {
    let dsl = encode_card_face(def, 0);
    // Role tags live at summary[120..144] in carddsl.rs
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
    if roles[21] > 0.0 {
        *blink_etb += 1;
    }
}
