fn encode_target_req(req: Option<&TargetReq>) -> [f32; 8] {
    let Some(req) = req else {
        return [0.0; 8];
    };
    let min_t = unit(f32::from(req.min), 4.0);
    let max_t = if req.max == u8::MAX || req.count_is_x {
        1.0
    } else {
        unit(f32::from(req.max), 4.0)
    };
    let up_to = f32::from(u8::from(req.min == 0 && req.max > 0));
    let mut creature = 0.0_f32;
    let mut player = 0.0_f32;
    let mut stack = 0.0_f32;
    let mut permanent = 0.0_f32;
    let mut graveyard = 0.0_f32;
    match req.spec {
        TargetSpec::AnyTarget => {
            creature = 1.0;
            player = 1.0;
            permanent = 1.0;
        }
        TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::OpponentWithMoreThanActivePlayer(_)
        | TargetSpec::OpponentWithMoreThanPlayer(..)
        | TargetSpec::Player(_)
        | TargetSpec::PlayerWhoAttackedThisTurn => {
            player = 1.0;
        }
        TargetSpec::OpponentOrObject(_) | TargetSpec::PlayerOrObject(_) => {
            player = 1.0;
            permanent = 1.0;
        }
        TargetSpec::Spell(_) | TargetSpec::AbilityOnStack(_) | TargetSpec::SpellOrAbility(_) => {
            stack = 1.0;
        }
        TargetSpec::StackOrBattlefield(_) => {
            stack = 1.0;
            permanent = 1.0;
        }
        TargetSpec::ObjectOrCardInAnyGraveyard(filter) => {
            permanent = 1.0;
            graveyard = 1.0;
            if filter_mentions_creature(filter) {
                creature = 1.0;
            }
        }
        TargetSpec::CardInGraveyard(_, _)
        | TargetSpec::CardInFirstTargetsGraveyard(_)
        | TargetSpec::CardInPlayersGraveyard(_, _)
        | TargetSpec::CardInGraveyardBelowEvent(_, _)
        | TargetSpec::CardInGraveyardBelowValue(_, _, _) => {
            graveyard = 1.0;
        }
        TargetSpec::Object(filter)
        | TargetSpec::ObjectOfEachOpponent(filter)
        | TargetSpec::ObjectOfFirstTargetsPlayer(filter)
        | TargetSpec::ObjectControlledBy(filter, _)
        | TargetSpec::ObjectOfEventPlayer(filter) => {
            permanent = 1.0;
            if filter_mentions_creature(filter) {
                creature = 1.0;
            }
        }
        TargetSpec::ThisObject | TargetSpec::EventObject => {
            permanent = 1.0;
        }
    }
    [
        min_t, max_t, up_to, creature, player, stack, permanent, graveyard,
    ]
}

fn filter_mentions_creature(filter: &Filter) -> bool {
    filter_mentions_type(filter, TypeSet::CREATURE)
}

fn filter_mentions_type(filter: &Filter, t: TypeSet) -> bool {
    match filter {
        Filter::HasType(ts) => ts.intersects(t),
        Filter::And(list) | Filter::Or(list) => list.iter().any(|f| filter_mentions_type(f, t)),
        Filter::Not(inner) => filter_mentions_type(inner, t),
        _ => false,
    }
}

/// Encodes one [`AbilityDef`] into `ABILITY_DSL_WIDTH` (`64`) floats:
/// - `[0..8]`: ability kind one-hot (`Activated`, `ManaAbility`, `Triggered`,
///   `Static`, `Spell`, `Replacement`, `KeywordParam`, `ChapterOrLevel`)
/// - `[8..22]`: activation & cost features (`sorcery_only`, `once_per_turn`,
///   `from_hand_or_gy`, `tap_self`, `untap_self`, `sac_self`, `sac_other`,
///   `pay_life`, `discard`, `exile_cost`, `remove_counter`, `loyalty_plus`,
///   `loyalty_minus`, `mana_cost_mv`)
/// - `[22..32]`: trigger features (`etb`, `dies`, `ltb`, `attacks`, `blocks`,
///   `combat_damage`, `spell_cast`, `step_upkeep_end`, `landfall_or_life`,
///   `counter_or_sac`)
/// - `[32..40]`: target requirement features (`encode_target_req`)
/// - `[40..64]`: 24 semantic effect channels (`EffectChannels::to_array`)
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "Explicit typed ability-to-channel schema"
)]
pub fn encode_ability_def(ab: &AbilityDef) -> [f32; ABILITY_DSL_WIDTH] {
    let mut out = [0.0_f32; ABILITY_DSL_WIDTH];
    let mut ch = EffectChannels::default();
    let mut targ: Option<&TargetReq> = None;

    match ab {
        AbilityDef::Activated {
            cost,
            effects,
            targets,
            timing,
            limit,
            zone,
            ..
        }
        | AbilityDef::ActivatedConditional {
            cost,
            effects,
            targets,
            timing,
            limit,
            zone,
            ..
        } => {
            let is_mana = ab.is_mana_ability()
                || crate::dsl::simple_mana(cost, effects).is_some()
                || crate::dsl::mana_offer(cost, effects).is_some();
            out[0] = f32::from(u8::from(!is_mana));
            out[1] = f32::from(u8::from(is_mana));
            out[8] = f32::from(u8::from(matches!(timing, ActivationTiming::SorcerySpeed)));
            out[9] = f32::from(u8::from(!matches!(limit, ActivationLimit::Unlimited)));
            out[10] = f32::from(u8::from(!matches!(zone, ActivationZone::Battlefield)));
            encode_cost_into(cost, &mut out[11..22]);
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::ModalActivated {
            cost,
            modes,
            timing,
            limit,
            zone,
            ..
        } => {
            let is_mana = ab.is_mana_ability();
            out[0] = f32::from(u8::from(!is_mana));
            out[1] = f32::from(u8::from(is_mana));
            out[8] = f32::from(u8::from(matches!(timing, ActivationTiming::SorcerySpeed)));
            out[9] = f32::from(u8::from(!matches!(limit, ActivationLimit::Unlimited)));
            out[10] = f32::from(u8::from(!matches!(zone, ActivationZone::Battlefield)));
            encode_cost_into(cost, &mut out[11..22]);
            for mode in *modes {
                if targ.is_none() {
                    targ = mode.targets.as_ref();
                }
                walk_effects(mode.effects, &mut ch);
            }
        }
        AbilityDef::Loyalty {
            cost: delta,
            effects,
            targets,
            ..
        } => {
            out[0] = 1.0;
            out[8] = 1.0; // sorcery speed
            out[9] = 1.0; // once per turn
            if *delta >= 0 {
                out[19] = unit(f32::from(*delta) + 1.0, 5.0);
            } else {
                out[20] = unit(f32::from(-*delta), 10.0);
            }
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::Triggered {
            trigger,
            effects,
            targets,
            ..
        } => {
            out[2] = 1.0;
            encode_trigger_into(trigger, &mut out[22..32]);
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::ModalTriggered { trigger, modes, .. } => {
            out[2] = 1.0;
            encode_trigger_into(trigger, &mut out[22..32]);
            for mode in *modes {
                if targ.is_none() {
                    targ = mode.targets.as_ref();
                }
                walk_effects(mode.effects, &mut ch);
            }
        }
        AbilityDef::Static(sa) => {
            out[3] = 1.0;
            encode_static_into(sa, &mut ch);
        }
        AbilityDef::Spell {
            effects, targets, ..
        } => {
            out[4] = 1.0;
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::ModalSpell { modes, .. } => {
            out[4] = 1.0;
            for mode in *modes {
                if targ.is_none() {
                    targ = mode.targets.as_ref();
                }
                walk_effects(mode.effects, &mut ch);
            }
        }
        AbilityDef::Replacement(rule) => {
            out[5] = 1.0;
            encode_replacement_into(rule, &mut ch);
        }
        AbilityDef::Echo { cost } | AbilityDef::Suspend { cost, .. } => {
            out[6] = 1.0;
            out[21] = unit(cost.cmc() as f32, 8.0);
        }
        AbilityDef::Ward { mana } => {
            out[6] = 1.0;
            out[21] = unit(f32::from(*mana), 6.0);
        }
        AbilityDef::Toxic { poison } => {
            out[6] = 1.0;
            ch.counters_add += f32::from(*poison);
        }
        AbilityDef::CopyOnEnter { .. } | AbilityDef::CopyOnEnterUntilEot { .. } => {
            out[6] = 1.0;
            ch.blink_copy_control += 1.0;
        }
        AbilityDef::SagaChapter {
            effects, targets, ..
        } => {
            out[7] = 1.0;
            targ = targets.as_ref();
            walk_effects(effects, &mut ch);
        }
        AbilityDef::Prepared { .. } | AbilityDef::StartYourEngines | AbilityDef::Unimplemented => {
            out[6] = 1.0;
        }
    }

    let targ_vec = encode_target_req(targ);
    out[32..40].copy_from_slice(&targ_vec);
    let eff_vec = ch.to_array();
    for (dst, src) in out[40..64].iter_mut().zip(eff_vec) {
        *dst = (*dst + src).clamp(-1.0, 1.0);
    }
    out
}

fn encode_cost_into(cost: &Cost, dst: &mut [f32]) {
    // dst has length 11: [tap, untap, sac_self, sac_other, pay_life, discard, exile, remove_counter, loyalty_plus, loyalty_minus, mana_mv]
    if cost.mana.cmc() > 0 {
        dst[10] = (dst[10] + unit(cost.mana.cmc() as f32, 8.0)).clamp(0.0, 1.0);
    }
    for part in cost.parts {
        match part {
            CostPart::TapSelf
            | CostPart::ExertSelf
            | CostPart::TapOther(_)
            | CostPart::TapX(_)
            | CostPart::TapN { .. }
            | CostPart::Crew(_)
            | CostPart::Saddle(_) => dst[0] = 1.0,
            CostPart::UntapSelf => dst[1] = 1.0,
            CostPart::SacrificeSelf | CostPart::ReturnSelfToHand => dst[2] = 1.0,
            CostPart::Sacrifice(_)
            | CostPart::SacrificeX(_)
            | CostPart::SacrificeAll(_)
            | CostPart::ReturnToHand(_) => dst[3] = 1.0,
            CostPart::PayLife(n) => dst[4] = unit(f32::from(*n), 10.0).max(0.2),
            CostPart::PayLifeX | CostPart::PayCommanderColors => dst[4] = 0.5,
            CostPart::Discard(_)
            | CostPart::DiscardHand
            | CostPart::DiscardSelf
            | CostPart::DiscardRandom
            | CostPart::DiscardLastDrawnThisTurn => dst[5] = 1.0,
            CostPart::ExileSelf
            | CostPart::ExileFromGraveyard(_)
            | CostPart::ExileFromHand(_)
            | CostPart::ExilePermanent(_)
            | CostPart::ExileFromSingleGraveyard { .. }
            | CostPart::ExileTopOfLibrary(_) => dst[6] = 1.0,
            CostPart::RemoveCounterSelf { .. }
            | CostPart::RemoveCounterSelfX { .. }
            | CostPart::PutCounterSelf { .. }
            | CostPart::PutCounterOnOther { .. }
            | CostPart::PayEnergy(_)
            | CostPart::PayEnergyX => dst[7] = 1.0,
            CostPart::PayAttachedManaCost => dst[10] = (dst[10] + 0.5).clamp(0.0, 1.0),
        }
    }
}

fn encode_trigger_into(trigger: &Trigger, dst: &mut [f32]) {
    // dst has length 10: [etb, dies, ltb, attacks, blocks, combat_damage, spell_cast, step_upkeep_end, landfall_or_life, counter_or_sac]
    match trigger {
        Trigger::EntersBattlefield(_)
        | Trigger::EnteredThisFrom(_)
        | Trigger::EntersBattlefieldEvoked => {
            dst[0] = 1.0;
        }
        Trigger::Dies(_) | Trigger::DiesNotSacrificed(_) | Trigger::DiesAfterDamageByThis(_) => {
            dst[1] = 1.0;
        }
        Trigger::LeavesBattlefield(_)
        | Trigger::ExiledFromBattlefield(_)
        | Trigger::LinkedCreatureLeaves => {
            dst[2] = 1.0;
        }
        Trigger::Attacks(_)
        | Trigger::OneOrMoreAttack(_)
        | Trigger::AttackWithAtLeast(_)
        | Trigger::AttackWithAtLeastMatching { .. }
        | Trigger::AttacksAndIsntBlocked(_)
        | Trigger::AttacksAlone(_) => {
            dst[3] = 1.0;
        }
        Trigger::Blocks(_)
        | Trigger::BlocksCreature(_)
        | Trigger::BecomesBlocked
        | Trigger::BecomesBlockedBy(_)
        | Trigger::BlocksOrBecomesBlockedBy(_)
        | Trigger::BlocksOrBecomesBlockedByOneOrMore(_)
        | Trigger::AttachedBlocksOrBecomesBlockedBy(_)
        | Trigger::Rampage => {
            dst[4] = 1.0;
        }
        Trigger::DealsCombatDamage(_)
        | Trigger::DealsCombatDamageToPlayer(_)
        | Trigger::DealsCombatDamageToYou(_)
        | Trigger::OneOrMoreDealCombatDamageToPlayer(_)
        | Trigger::DealsCombatDamageToOpponent(_)
        | Trigger::DealsCombatDamageToPlayerOrPlaneswalker(_)
        | Trigger::DealsCombatDamageToCreature(_) => {
            dst[5] = 1.0;
        }
        Trigger::DealsDamage(_)
        | Trigger::DealsDamageToOpponent(_)
        | Trigger::DealsDamageToPlayer { .. }
        | Trigger::DealtDamage(_)
        | Trigger::DealtCombatDamage(_)
        | Trigger::PlayerDealtDamage(_) => {
            dst[5] = 0.7;
        }
        Trigger::SpellCast(_)
        | Trigger::SpellCastUsingSourceMana(_)
        | Trigger::SpellCastOrCopied(_)
        | Trigger::CastThisWithMana(_)
        | Trigger::NthSpellCast { .. }
        | Trigger::InstantCastAfterFirst { .. }
        | Trigger::FirstNoncreatureSpellCast(_) => {
            dst[6] = 1.0;
        }
        Trigger::StepBegin { .. } | Trigger::StepBeginChosenOpponent { .. } => {
            dst[7] = 1.0;
        }
        Trigger::PlaysLand(_)
        | Trigger::PlaysAnotherLand
        | Trigger::GainsLife(_)
        | Trigger::LosesLife(_)
        | Trigger::Draws(_)
        | Trigger::DrawsExceptFirst(_)
        | Trigger::NthCardDrawnInTurn { .. } => {
            dst[8] = 1.0;
        }
        Trigger::CountersPlaced { .. }
        | Trigger::CountersReach { .. }
        | Trigger::LastCounterRemoved { .. }
        | Trigger::Discarded { .. }
        | Trigger::DiscardedThisByOpponent
        | Trigger::CycledThis => {
            dst[9] = 1.0;
        }
        _ => {
            dst[8] = 0.5;
        }
    }
}

fn encode_static_into(sa: &StaticAbility, ch: &mut EffectChannels) {
    match &sa.modifier {
        Modifier::ModifyPT(p, t) => {
            ch.pump_pt += f32::from(*p + *t) * 0.5;
        }
        Modifier::ModifyPTPerCount { .. }
        | Modifier::ModifyPTByCount(_)
        | Modifier::ReducePTByCount { .. }
        | Modifier::ModifyPTByAmount { .. }
        | Modifier::ModifyPTHalfCount(_)
        | Modifier::ModifyPTPerGraveyardCard { .. }
        | Modifier::SetPT(..)
        | Modifier::SetPower(_)
        | Modifier::SetToughness(_)
        | Modifier::SetPTToCount(_)
        | Modifier::CharacteristicPT { .. }
        | Modifier::CharacteristicPTSeparate { .. } => {
            ch.pump_pt += 2.0;
        }
        Modifier::AddKeyword(_) | Modifier::AddKeywordIfCountersAtLeast { .. } => {
            ch.pump_pt += 1.0;
        }
        Modifier::GrantActivated { mana_ability, .. } => {
            if *mana_ability {
                ch.mana_add += 1.0;
            }
        }
        Modifier::GrantActivatedDefinition(ab) => {
            let sub = encode_ability_def(ab);
            if sub[1] > 0.0 {
                ch.mana_add += 1.0;
            }
        }
        Modifier::SpellsCostLess(amount) | Modifier::AbilitiesCostLess { amount, .. } => {
            ch.mana_add += (*amount as f32).max(1.0) * 0.5;
        }
        Modifier::SpellsCostMore(_)
        | Modifier::SpellsCostMoreMana(_)
        | Modifier::AbilitiesCostMore(_)
        | Modifier::OpponentsCantCast(_)
        | Modifier::PlayersCantCast { .. }
        | Modifier::PlayersCantPlay(_)
        | Modifier::CantActivate
        | Modifier::ChosenNameCantActivate
        | Modifier::CantActivateArtifacts => {
            ch.counter_spell += 0.5;
        }
        _ => {}
    }
}

fn encode_replacement_into(rule: &ReplacementRule, ch: &mut EffectChannels) {
    match rule {
        ReplacementRule::DoubleCounterPlacement { .. } => {
            ch.counters_add += 2.0;
        }
        ReplacementRule::DoubleTokenCreation { .. }
        | ReplacementRule::MultiplyTypedTokens { .. }
        | ReplacementRule::OpponentsExtraDrawsCreateToken(_) => {
            ch.tokens_create += 2.0;
        }
        ReplacementRule::TriggerMultiplier { .. } => {
            ch.draw += 1.0;
            ch.tokens_create += 1.0;
        }
        ReplacementRule::ExileSelfInsteadOfGraveyard
        | ReplacementRule::ExileOpponentsGraveyard { .. } => {
            ch.exile_all_or_gy += 1.5;
        }
        _ => {}
    }
}
