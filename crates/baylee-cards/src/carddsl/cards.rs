/// Encodes one face of a [`CardDef`] into its typed [`CardDslFeatures`].
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "Contiguous versioned card feature layout"
)]
pub fn encode_card_face(def: &CardDef, face_idx: usize) -> CardDslFeatures {
    let mut out = CardDslFeatures::default();
    let Some(face) = def.faces.get(face_idx).or_else(|| def.faces.first()) else {
        return out;
    };
    let s = &mut out.summary;
    // [0..10]: mana cost
    let mc = encode_mana_cost(&face.mana_cost);
    s[0..10].copy_from_slice(&mc);
    // [10..15]: face colors (from mana cost or color indicator)
    let face_colors = if face.mana_cost.colors().is_empty() {
        face.color_indicator
    } else {
        face.mana_cost.colors()
    };
    s[10..15].copy_from_slice(&encode_color_set(face_colors));
    // [15..20]: card color identity (CR 903.4)
    s[15..20].copy_from_slice(&encode_color_set(def.color_identity));
    // [20..29]: card types
    s[20..29].copy_from_slice(&encode_type_set(face.types));
    // [29..33]: supertypes
    s[29..33].copy_from_slice(&encode_supertype_set(face.supertypes));
    // [33..41]: printed stats (power, toughness, loyalty, defense)
    s[33] = f32::from(u8::from(face.power.is_some()));
    s[34] = face.power.map_or(0.0, |p| signed_unit(f32::from(p), 12.0));
    s[35] = f32::from(u8::from(face.toughness.is_some()));
    s[36] = face
        .toughness
        .map_or(0.0, |t| signed_unit(f32::from(t), 12.0));
    s[37] = f32::from(u8::from(face.loyalty.is_some()));
    s[38] = face.loyalty.map_or(0.0, |l| unit(f32::from(l), 8.0));
    s[39] = f32::from(u8::from(face.defense.is_some()));
    s[40] = face.defense.map_or(0.0, |d| unit(f32::from(d), 8.0));

    // [41..65]: 24 core keywords on this face
    let kw = def.keywords_for_face(face_idx);
    for (k, flag) in CORE_KEYWORDS.iter().enumerate() {
        s[41 + k] = f32::from(u8::from(kw.contains(*flag)));
    }

    // [65..80]: multi-face, commander, partner, alternative/additional cast mechanics
    s[65] = f32::from(u8::from(def.faces.len() > 1));
    s[66] = f32::from(u8::from(face_idx > 0));
    s[67] = f32::from(u8::from(face.castable_from_hand));
    s[68] = f32::from(u8::from(!matches!(
        def.commander,
        CommanderRule::NotEligible
    )));
    s[69] = f32::from(u8::from(!matches!(def.partner, PartnerKind::None)));
    s[70] = f32::from(u8::from(face.flashback.is_some() || face.disturb));
    s[71] = f32::from(u8::from(face.escape.is_some()));
    s[72] = f32::from(u8::from(face.adventure || face.prototype.is_some()));
    s[73] = f32::from(u8::from(face.morph.is_some() || face.disguise.is_some()));
    s[74] = f32::from(u8::from(
        face.miracle.is_some() || face.dash.is_some() || face.mutate.is_some(),
    ));
    s[75] = f32::from(u8::from(
        face.delve || face.convoke || face.improvise || face.waterbend,
    ));
    s[76] = f32::from(u8::from(face.cost_reduction.is_some()));
    s[77] = unit(face.additional_costs.len() as f32, 3.0);
    s[78] = unit(face.alternative_costs.len() as f32, 3.0);
    s[79] = unit(face.enter_modifiers.len() as f32, 3.0);

    // [80..88]: enter-modifier flags
    for em in face.enter_modifiers {
        match em {
            EnterModifier::Tapped => s[80] = 1.0,
            EnterModifier::TappedUnless(_)
            | EnterModifier::TappedUnlessCount { .. }
            | EnterModifier::TappedUnlessAtMost { .. }
            | EnterModifier::TappedUnlessCondition(_)
            | EnterModifier::TappedUnlessOpponents { .. }
            | EnterModifier::TappedUnlessSomeoneAtOrBelow { .. }
            | EnterModifier::TappedUnlessReveal(_)
            | EnterModifier::TappedUnlessRevealOrControl(_) => s[81] = 1.0,
            EnterModifier::TappedOrPayLife(_) => s[82] = 1.0,
            EnterModifier::WithCounters { .. } | EnterModifier::ChooseCounterPerEntryCount(_) => {
                s[83] = 1.0;
            }
            EnterModifier::ChooseColor | EnterModifier::ChooseColorExcept(_) => s[84] = 1.0,
            EnterModifier::ChooseSubtype | EnterModifier::ChooseSubtypeFrom(_) => s[85] = 1.0,
            EnterModifier::ChooseCardName
            | EnterModifier::ChooseCardType
            | EnterModifier::ChooseOpponent => s[86] = 1.0,
            _ => s[87] = 1.0,
        }
    }

    // Encode individual abilities + aggregate counts & effect channels
    let abilities = def.abilities_for_face(face_idx);
    let mut agg_ch = EffectChannels::default();
    let mut count_act = 0.0_f32;
    let mut count_mana = 0.0_f32;
    let mut count_trig = 0.0_f32;
    let mut count_stat = 0.0_f32;
    let mut count_spell = 0.0_f32;
    let mut count_repl = 0.0_f32;
    let mut count_kw = 0.0_f32;
    let mut count_chap = 0.0_f32;

    for (idx, ab) in abilities.iter().enumerate() {
        let row = encode_ability_def(ab);
        if idx < MAX_ABILITIES {
            out.abilities[idx] = row;
            out.ability_count = (idx + 1) as u8;
        }
        count_act += row[0];
        count_mana += row[1];
        count_trig += row[2];
        count_stat += row[3];
        count_spell += row[4];
        count_repl += row[5];
        count_kw += row[6];
        count_chap += row[7];
        // Accumulate into agg_ch from ability effects
        match ab {
            AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }
            | AbilityDef::Loyalty { effects, .. }
            | AbilityDef::Triggered { effects, .. }
            | AbilityDef::Spell { effects, .. }
            | AbilityDef::SagaChapter { effects, .. } => walk_effects(effects, &mut agg_ch),
            AbilityDef::ModalSpell { modes, .. }
            | AbilityDef::ModalActivated { modes, .. }
            | AbilityDef::ModalTriggered { modes, .. } => {
                for m in *modes {
                    walk_effects(m.effects, &mut agg_ch);
                }
            }
            AbilityDef::Static(sa) => encode_static_into(sa, &mut agg_ch),
            AbilityDef::Replacement(r) => encode_replacement_into(r, &mut agg_ch),
            _ => {}
        }
    }

    // [88..96]: ability category counts
    s[88] = unit(count_act, 4.0);
    s[89] = unit(count_mana, 4.0);
    s[90] = unit(count_trig, 4.0);
    s[91] = unit(count_stat, 4.0);
    s[92] = unit(count_spell, 2.0);
    s[93] = unit(count_repl, 2.0);
    s[94] = unit(count_kw, 3.0);
    s[95] = unit(count_chap, 4.0);

    // [96..120]: 24 aggregate semantic effect channels
    let eff_arr = agg_ch.to_array();
    s[96..120].copy_from_slice(&eff_arr);

    // [120..144]: 24 synergy & deckbuilding role tags (shared with Expert 2)
    populate_synergy_tags(def, face, kw, &agg_ch, count_mana, &mut s[120..144]);

    // [144..160]: 16 deterministic subtype coordinates from `SubtypeId::raw()`
    encode_subtypes_into(face.subtypes, &mut s[144..160]);

    out
}

/// Encodes a [`TokenDef`] into the same [`CardDslFeatures`] layout as a card face.
#[must_use]
pub fn encode_token_def(tok: &TokenDef) -> CardDslFeatures {
    let mut out = CardDslFeatures::default();
    let s = &mut out.summary;
    s[10..15].copy_from_slice(&encode_color_set(tok.colors));
    s[15..20].copy_from_slice(&encode_color_set(tok.colors));
    s[20..29].copy_from_slice(&encode_type_set(tok.types));
    s[29..33].copy_from_slice(&encode_supertype_set(tok.supertypes));
    s[33] = f32::from(u8::from(tok.power.is_some()));
    s[34] = tok.power.map_or(0.0, |p| signed_unit(f32::from(p), 12.0));
    s[35] = f32::from(u8::from(tok.toughness.is_some()));
    s[36] = tok
        .toughness
        .map_or(0.0, |t| signed_unit(f32::from(t), 12.0));
    s[37] = f32::from(u8::from(tok.loyalty.is_some()));
    s[38] = tok.loyalty.map_or(0.0, |l| unit(f32::from(l), 8.0));
    s[39] = f32::from(u8::from(tok.defense.is_some()));
    s[40] = tok.defense.map_or(0.0, |d| unit(f32::from(d), 8.0));
    for (k, flag) in CORE_KEYWORDS.iter().enumerate() {
        s[41 + k] = f32::from(u8::from(tok.keywords.contains(*flag)));
    }
    let mut agg_ch = EffectChannels::default();
    let mut count_act = 0.0_f32;
    let mut count_mana = 0.0_f32;
    let mut count_trig = 0.0_f32;
    let mut count_stat = 0.0_f32;
    for (idx, ab) in tok.abilities.iter().enumerate() {
        let row = encode_ability_def(ab);
        if idx < MAX_ABILITIES {
            out.abilities[idx] = row;
            out.ability_count = (idx + 1) as u8;
        }
        count_act += row[0];
        count_mana += row[1];
        count_trig += row[2];
        count_stat += row[3];
        if let AbilityDef::Activated { effects, .. } | AbilityDef::Triggered { effects, .. } = ab {
            walk_effects(effects, &mut agg_ch);
        }
    }
    s[88] = unit(count_act, 4.0);
    s[89] = unit(count_mana, 4.0);
    s[90] = unit(count_trig, 4.0);
    s[91] = unit(count_stat, 4.0);
    s[96..120].copy_from_slice(&agg_ch.to_array());
    encode_subtypes_into(tok.subtypes, &mut s[144..160]);
    out
}

#[allow(clippy::too_many_lines, reason = "Explicit synergy tag schema")]
fn populate_synergy_tags(
    def: &CardDef,
    face: &FaceDef,
    kw: KeywordSet,
    ch: &EffectChannels,
    count_mana: f32,
    dst: &mut [f32],
) {
    let is_land = face.types.contains(TypeSet::LAND);
    let is_creature = face.types.contains(TypeSet::CREATURE);
    let is_instant_sorcery =
        face.types.contains(TypeSet::INSTANT) || face.types.contains(TypeSet::SORCERY);
    let mv = face.mana_cost.cmc();

    // 0: mana_dork_or_rock (non-land mana producer)
    dst[0] = f32::from(u8::from(
        !is_land && (count_mana > 0.0 || ch.mana_add > 0.0),
    ));
    // 1: land_ramp_or_fetcher
    dst[1] = f32::from(u8::from(!is_land && ch.tutor_search > 0.0 && mv <= 4));
    // 2: card_draw_engine
    dst[2] = f32::from(u8::from(ch.draw > 0.0));
    // 3: cantrip (cheap spell that draws/scries)
    dst[3] = f32::from(u8::from(
        is_instant_sorcery && mv <= 2 && (ch.draw > 0.0 || ch.scry_surveil > 0.0),
    ));
    // 4: spot_removal
    dst[4] = f32::from(u8::from(
        ch.destroy_single > 0.0 || ch.exile_single > 0.0 || ch.damage_any >= 2.0,
    ));
    // 5: board_wipe
    dst[5] = f32::from(u8::from(
        ch.destroy_all > 0.0 || ch.exile_all_or_gy > 0.0 || ch.damage_board >= 2.0,
    ));
    // 6: stack_countermagic
    dst[6] = f32::from(u8::from(ch.counter_spell > 0.0));
    // 7: graveyard_filler (self-mill / surveil / entomb)
    dst[7] = f32::from(u8::from(ch.mill > 0.0 || ch.scry_surveil > 0.0));
    // 8: graveyard_payoff (reanimate / flashback / escape)
    dst[8] = f32::from(u8::from(
        ch.reanimate > 0.0 || face.flashback.is_some() || face.escape.is_some() || face.disturb,
    ));
    // 9: token_generator
    dst[9] = f32::from(u8::from(ch.tokens_create > 0.0));
    // 10: sacrifice_outlet (activated ability with sacrifice cost)
    let has_sac_outlet = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Activated { cost, .. }
                if cost.parts.iter().any(|p| matches!(p, CostPart::Sacrifice { .. } | CostPart::SacrificeX { .. }))
        )
    });
    dst[10] = f32::from(u8::from(has_sac_outlet));
    // 11: death_trigger_payoff
    let has_death_trig = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::Dies(_)
                    | Trigger::DiesNotSacrificed(_)
                    | Trigger::DiesAfterDamageByThis(_)
                    | Trigger::LeavesBattlefield(_),
                ..
            }
        )
    });
    dst[11] = f32::from(u8::from(has_death_trig));
    // 12: counters_synergy
    dst[12] = f32::from(u8::from(ch.counters_add > 0.0));
    // 13: spellslinger_payoff (prowess or triggers on spell cast)
    let has_spell_trig = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::SpellCast(_) | Trigger::SpellCastOrCopied(_),
                ..
            }
        )
    });
    dst[13] = f32::from(u8::from(kw.contains(KeywordSet::PROWESS) || has_spell_trig));
    // 14: artifact_synergy
    let mentions_artifact = def.abilities_for_face(0).iter().any(|ab| match ab {
        AbilityDef::Static(sa) => filter_mentions_type(&sa.filter, TypeSet::ARTIFACT),
        _ => false,
    });
    dst[14] = f32::from(u8::from(
        face.types.contains(TypeSet::ARTIFACT) || mentions_artifact || face.improvise,
    ));
    // 15: enchantment_synergy
    dst[15] = f32::from(u8::from(face.types.contains(TypeSet::ENCHANTMENT)));
    // 16: evasion_threat
    let evasion_mask = KeywordSet::FLYING
        .union(KeywordSet::MENACE)
        .union(KeywordSet::TRAMPLE)
        .union(KeywordSet::UNBLOCKABLE);
    dst[16] = f32::from(u8::from(
        is_creature && (kw.bits() & evasion_mask.bits() != 0),
    ));
    // 17: combat_trick
    dst[17] = f32::from(u8::from(
        face.types.contains(TypeSet::INSTANT) && ch.pump_pt != 0.0,
    ));
    // 18: tutor
    dst[18] = f32::from(u8::from(ch.tutor_search > 0.0));
    // 19: lifegain_payoff_or_source
    dst[19] = f32::from(u8::from(
        ch.life_gain > 0.0 || kw.contains(KeywordSet::LIFELINK),
    ));
    // 20: burn_or_drain
    dst[20] = f32::from(u8::from(ch.damage_any > 0.0 || ch.life_lose > 0.0));
    // 21: blink_or_etb_value
    let has_etb = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::EntersBattlefield(_)
                    | Trigger::EnteredThisFrom(_)
                    | Trigger::EntersBattlefieldEvoked,
                ..
            }
        )
    });
    dst[21] = f32::from(u8::from(is_creature && has_etb));
    // 22: lord_or_anthem
    let is_anthem = def.abilities_for_face(0).iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Static(sa)
                if !matches!(sa.filter, Filter::This)
                    && matches!(sa.modifier, Modifier::ModifyPT(_, _) | Modifier::AddKeyword(_))
        )
    });
    dst[22] = f32::from(u8::from(is_anthem));
    // 23: finisher (high-impact threat or extra turn / win condition)
    dst[23] = f32::from(u8::from(
        ch.extra_turn_or_win > 0.0 || (is_creature && mv >= 6),
    ));
}

fn encode_subtypes_into(subtypes: &[baylee_core::ids::SubtypeId], dst: &mut [f32]) {
    dst[0] = unit(subtypes.len() as f32, 4.0);
    for &st in subtypes {
        let id = u32::from(st.get());
        let b1 = 1 + ((id.wrapping_mul(2_654_435_761)) as usize % 15);
        let b2 = 1 + ((id.wrapping_mul(1_597_334_677)) as usize % 15);
        dst[b1] = (dst[b1] + 0.5).clamp(0.0, 1.0);
        dst[b2] = (dst[b2] + 0.5).clamp(0.0, 1.0);
    }
}

/// Builds a flat `N * CARD_DSL_WIDTH` table of front-face card summaries for a
/// vocabulary slice of `CardIndex` values (`cards[0]` is typically `0` =
/// unknown/hidden card, which stays all zeros).
#[must_use]
pub fn card_summary_table(cards: &[u32]) -> Vec<f32> {
    let mut out = vec![0.0_f32; cards.len() * CARD_DSL_WIDTH];
    for (row, &card_id) in cards.iter().enumerate() {
        if card_id == UNKNOWN_CARD {
            continue;
        }
        let Some(def) = crate::by_index(CardIndex::new(card_id)) else {
            continue;
        };
        let encoded = encode_card_face(def, 0);
        let start = row * CARD_DSL_WIDTH;
        out[start..start + CARD_DSL_WIDTH].copy_from_slice(&encoded.summary);
    }
    out
}

/// Builds a flat `N * CARD_DSL_WIDTH` table of back-face (`face_id = 1`) card
/// summaries for a vocabulary slice of `CardIndex` values (`cards[0]` stays all
/// zeros; single-faced cards mirror `encode_card_face(def, 1)`).
#[must_use]
pub fn card_back_summary_table(cards: &[u32]) -> Vec<f32> {
    let mut out = vec![0.0_f32; cards.len() * CARD_DSL_WIDTH];
    for (row, &card_id) in cards.iter().enumerate() {
        if card_id == UNKNOWN_CARD {
            continue;
        }
        let Some(def) = crate::by_index(CardIndex::new(card_id)) else {
            continue;
        };
        let encoded = encode_card_face(def, 1);
        let start = row * CARD_DSL_WIDTH;
        out[start..start + CARD_DSL_WIDTH].copy_from_slice(&encoded.summary);
    }
    out
}

/// Builds a flat `(ALL.len() + 1) * CARD_DSL_WIDTH` table of token DSL
/// summaries where row `0` is all zeros (non-token) and row `1 + token_id` is
/// `encode_token_def(ALL[token_id]).summary`.
#[must_use]
pub fn token_summary_table() -> Vec<f32> {
    let n = crate::tokens::ALL.len() + 1;
    let mut out = vec![0.0_f32; n * CARD_DSL_WIDTH];
    for (idx, &tok) in crate::tokens::ALL.iter().enumerate() {
        let encoded = encode_token_def(tok);
        let start = (idx + 1) * CARD_DSL_WIDTH;
        out[start..start + CARD_DSL_WIDTH].copy_from_slice(&encoded.summary);
    }
    out
}

/// Builds a flat `N * MAX_ABILITIES * ABILITY_DSL_WIDTH` table of structured
/// per-ability features for a vocabulary slice of `CardIndex` values.
#[must_use]
pub fn card_ability_table(cards: &[u32]) -> Vec<f32> {
    let stride = MAX_ABILITIES * ABILITY_DSL_WIDTH;
    let mut out = vec![0.0_f32; cards.len() * stride];
    for (row, &card_id) in cards.iter().enumerate() {
        if card_id == UNKNOWN_CARD {
            continue;
        }
        let Some(def) = crate::by_index(CardIndex::new(card_id)) else {
            continue;
        };
        let encoded = encode_card_face(def, 0);
        let base = row * stride;
        for (a_idx, ab_row) in encoded.abilities.iter().enumerate() {
            let start = base + a_idx * ABILITY_DSL_WIDTH;
            out[start..start + ABILITY_DSL_WIDTH].copy_from_slice(ab_row);
        }
    }
    out
}

