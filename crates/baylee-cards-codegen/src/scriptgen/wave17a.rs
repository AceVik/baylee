//! Wave 17a `scriptgen` readers:
//! 1. Generalized trigger-doubling (`S: Mode$ Panharmonicon`, `CR 603.2d`) ->
//!    `ReplacementRule::TriggerMultiplier` (for canonical unconditional shapes)
//!    and `ReplacementRule::TriggerMultiplierMatching { source_filter, cause, condition }`
//!    (filtered ETB/Dies/LTB/Sacrificed/Attack/SpellCastOrCopy/LifeGained/CardDrawn/
//!    DealsCombatDamageToPlayer/DealtDamage/TurnedFaceUp/BecomesTarget + conditions).
//! 2. Cast-provenance queries (`wasCast*` filter atoms, self-ETB triggers,
//!    `ConditionPresent$` clauses, and `Count$wasCast*` conditional amounts).
//! 3. `Effect::Learn` (`CR 701.46`: `AB$ Learn` / `SP$ Learn` / `DB$ Learn`).

use super::{Params, Tx};

enum PanharmoniconForm {
    Legacy(&'static str),
    Matching(String),
}

pub(super) enum SelfCastCond {
    Matched(Option<String>),
    Pass,
}

fn has_exact_attack_modes(valid_mode: &str) -> bool {
    let mut saw_attacks = false;
    let mut saw_declared = false;
    let mut saw_one_target = false;
    let mut count = 0usize;
    for part in valid_mode.split(',').map(str::trim) {
        count += 1;
        match part {
            "Attacks" => saw_attacks = true,
            "AttackersDeclared" => saw_declared = true,
            "AttackersDeclaredOneTarget" => saw_one_target = true,
            _ => return false,
        }
    }
    count == 3 && saw_attacks && saw_declared && saw_one_target
}

fn has_exact_combat_player_damage_modes(valid_mode: &str) -> bool {
    let mut saw_done = false;
    let mut saw_done_once = false;
    let mut saw_dealt_once = false;
    let mut saw_all = false;
    let mut count = 0usize;
    for part in valid_mode.split(',').map(str::trim) {
        count += 1;
        match part {
            "DamageDone" => saw_done = true,
            "DamageDoneOnce" => saw_done_once = true,
            "DamageDealtOnce" => saw_dealt_once = true,
            "DamageAll" => saw_all = true,
            _ => return false,
        }
    }
    count == 4 && saw_done && saw_done_once && saw_dealt_once && saw_all
}

fn has_dealt_damage_modes(valid_mode: &str) -> bool {
    let parts: Vec<&str> = valid_mode.split(',').map(str::trim).collect();
    !parts.is_empty()
        && parts
            .iter()
            .all(|m| matches!(*m, "DamageDone" | "DamageDoneOnce"))
}

fn parse_spell_cast_copy_modes(valid_mode: &str) -> Option<bool> {
    let mut saw_cast = false;
    let mut saw_copy = false;
    let mut saw_both = false;
    for part in valid_mode.split(',').map(str::trim) {
        match part {
            "SpellCast" => saw_cast = true,
            "SpellCopy" => saw_copy = true,
            "SpellCastOrCopy" => saw_both = true,
            _ => return None,
        }
    }
    if saw_cast && saw_copy && saw_both {
        Some(true)
    } else if saw_cast && !saw_copy && !saw_both {
        Some(false)
    } else {
        None
    }
}

fn panharmonicon_player_rel(raw: Option<&str>) -> Option<&'static str> {
    Some(match raw.map(str::trim) {
        None | Some("Player") => "PlayerRel::EachPlayer",
        Some("You") => "PlayerRel::You",
        Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
        Some("Player.Other") => "PlayerRel::EachOtherPlayer",
        _ => return None,
    })
}

impl Tx<'_> {
    fn panharmonicon_cause_filter(&mut self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed == "Permanent" {
            return Some("&Filter::Any".to_string());
        }
        let expr = self.filter_expr(trimmed)?;
        Some(match expr.as_str() {
            "Filter::Any" => "&Filter::Any".to_string(),
            "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
            "Filter::ARTIFACT" => "&Filter::ARTIFACT".to_string(),
            "Filter::ENCHANTMENT" => "&Filter::ENCHANTMENT".to_string(),
            "Filter::LAND" => "&Filter::LAND".to_string(),
            "Filter::INSTANT_OR_SORCERY" => "&Filter::INSTANT_OR_SORCERY".to_string(),
            "Filter::YOUR_CREATURE" => "&Filter::YOUR_CREATURE".to_string(),
            "Filter::YOUR_ARTIFACT" => "&Filter::YOUR_ARTIFACT".to_string(),
            "Filter::YOUR_LAND" => "&Filter::YOUR_LAND".to_string(),
            _ => format!("&{}", self.body.filter_static("TRIGGER_CAUSE", &expr)),
        })
    }

    fn panharmonicon_spell_filter(&mut self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if matches!(trimmed, "Card" | "Spell") {
            return Some("&Filter::Any".to_string());
        }
        let expr = self.filter_expr_in_zone(trimmed, true)?;
        Some(match expr.as_str() {
            "Filter::Any" => "&Filter::Any".to_string(),
            "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
            "Filter::INSTANT_OR_SORCERY" => "&Filter::INSTANT_OR_SORCERY".to_string(),
            "Filter::ARTIFACT" => "&Filter::ARTIFACT".to_string(),
            "Filter::ENCHANTMENT" => "&Filter::ENCHANTMENT".to_string(),
            _ => format!("&{}", self.body.filter_static("TRIGGER_CAUSE", &expr)),
        })
    }

    /// Generalized `S: Mode$ Panharmonicon` reader (`CR 603.2d`).
    ///
    /// Preserves canonical `ReplacementRule::TriggerMultiplier` for unconditional
    /// `Any` / `Permanent` ETB / `Creature` Attack shapes, and emits
    /// `ReplacementRule::TriggerMultiplierMatching` for cause-filtered and
    /// conditional trigger-doubling rules.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave17a_panharmonicon_static(&mut self, mut p: Params) -> Option<()> {
        let deny = |tx: &Self| tx.deny("static ability `S: Mode$ Panharmonicon`".to_string());
        p.drop_prose();
        if let Some(sec) = p.take("Secondary")
            && sec != "True"
        {
            return deny(self);
        }
        if let Some(vz) = p.take("ValidZone")
            && vz != "Battlefield"
        {
            return deny(self);
        }
        let Some(valid_card) = p.take("ValidCard") else {
            return deny(self);
        };
        let valid_mode = p.take("ValidMode");

        let form = match valid_mode.as_deref() {
            None => PanharmoniconForm::Legacy("TriggerEventKind::Any"),
            Some("ChangesZone" | "ChangesZoneAll" | "ChangesZone,ChangesZoneAll") => {
                let origin = p.take("Origin");
                let dest = p.take("Destination");
                let Some(cause_raw) = p.take("ValidCause") else {
                    return deny(self);
                };
                match (origin.as_deref(), dest.as_deref()) {
                    (None | Some("Any"), Some("Battlefield")) => {
                        if cause_raw.trim() == "Permanent" {
                            PanharmoniconForm::Legacy("TriggerEventKind::EntersBattlefield")
                        } else {
                            let Some(cf) = self.panharmonicon_cause_filter(&cause_raw) else {
                                return deny(self);
                            };
                            PanharmoniconForm::Matching(format!(
                                "TriggerCause::EntersBattlefield({cf})"
                            ))
                        }
                    }
                    (Some("Battlefield"), Some("Graveyard")) => {
                        let Some(cf) = self.panharmonicon_cause_filter(&cause_raw) else {
                            return deny(self);
                        };
                        PanharmoniconForm::Matching(format!("TriggerCause::Dies({cf})"))
                    }
                    (Some("Battlefield"), None | Some("Any")) => {
                        let Some(cf) = self.panharmonicon_cause_filter(&cause_raw) else {
                            return deny(self);
                        };
                        PanharmoniconForm::Matching(format!(
                            "TriggerCause::LeavesBattlefield({cf})"
                        ))
                    }
                    _ => return deny(self),
                }
            }
            Some("Sacrificed") => {
                let cause_raw = p
                    .take("ValidCause")
                    .unwrap_or_else(|| "Permanent".to_string());
                let Some(who) = panharmonicon_player_rel(p.take("ValidPlayer").as_deref()) else {
                    return deny(self);
                };
                let Some(cf) = self.panharmonicon_cause_filter(&cause_raw) else {
                    return deny(self);
                };
                PanharmoniconForm::Matching(format!(
                    "TriggerCause::Sacrificed {{ filter: {cf}, who: {who} }}"
                ))
            }
            Some("LifeGained") => {
                let Some(who) = panharmonicon_player_rel(p.take("ValidPlayer").as_deref()) else {
                    return deny(self);
                };
                PanharmoniconForm::Matching(format!("TriggerCause::LifeGained({who})"))
            }
            Some("Drawn") => {
                let Some(who) = panharmonicon_player_rel(p.take("ValidPlayer").as_deref()) else {
                    return deny(self);
                };
                PanharmoniconForm::Matching(format!("TriggerCause::CardDrawn({who})"))
            }
            Some("TurnFaceUp") => {
                let Some(turned_raw) = p.take("ValidTurned").or_else(|| p.take("ValidCause"))
                else {
                    return deny(self);
                };
                let Some(cf) = self.panharmonicon_cause_filter(&turned_raw) else {
                    return deny(self);
                };
                PanharmoniconForm::Matching(format!("TriggerCause::TurnedFaceUp({cf})"))
            }
            Some("BecomesTarget,BecomesTargetOnce" | "BecomesTargetOnce,BecomesTarget") => {
                let Some(tgt_raw) = p.take("ValidTarget") else {
                    return deny(self);
                };
                if p.has("ValidSource") {
                    return deny(self);
                }
                let Some(cf) = self.panharmonicon_cause_filter(&tgt_raw) else {
                    return deny(self);
                };
                PanharmoniconForm::Matching(format!("TriggerCause::BecomesTarget({cf})"))
            }
            Some(other) if has_exact_attack_modes(other) => {
                if p.has("Origin") || p.has("Destination") {
                    return deny(self);
                }
                let Some(cause_raw) = p.take("ValidCause") else {
                    return deny(self);
                };
                if cause_raw.trim() == "Creature" {
                    PanharmoniconForm::Legacy("TriggerEventKind::Attack")
                } else {
                    let Some(cf) = self.panharmonicon_cause_filter(&cause_raw) else {
                        return deny(self);
                    };
                    PanharmoniconForm::Matching(format!("TriggerCause::Attack({cf})"))
                }
            }
            Some(other) if parse_spell_cast_copy_modes(other).is_some() => {
                let allow_copy = parse_spell_cast_copy_modes(other).unwrap_or(false);
                let cause_raw = p.take("ValidCause").unwrap_or_else(|| "Card".to_string());
                let Some(who) = panharmonicon_player_rel(p.take("ValidActivator").as_deref())
                else {
                    return deny(self);
                };
                let Some(cf) = self.panharmonicon_spell_filter(&cause_raw) else {
                    return deny(self);
                };
                PanharmoniconForm::Matching(format!(
                    "TriggerCause::SpellCastOrCopy {{ filter: {cf}, who: {who}, allow_copy: {allow_copy} }}"
                ))
            }
            Some(other) if has_exact_combat_player_damage_modes(other) => {
                if p.take("CombatDamage").as_deref() != Some("True")
                    || p.take("ValidTarget").as_deref() != Some("Player")
                {
                    return deny(self);
                }
                let Some(src_raw) = p.take("ValidSource") else {
                    return deny(self);
                };
                let Some(cf) = self.panharmonicon_cause_filter(&src_raw) else {
                    return deny(self);
                };
                PanharmoniconForm::Matching(format!(
                    "TriggerCause::DealsCombatDamageToPlayer({cf})"
                ))
            }
            Some(other) if has_dealt_damage_modes(other) => {
                if p.has("ValidSource") || p.has("CombatDamage") {
                    return deny(self);
                }
                let Some(tgt_raw) = p.take("ValidTarget") else {
                    return deny(self);
                };
                let Some(cf) = self.panharmonicon_cause_filter(&tgt_raw) else {
                    return deny(self);
                };
                PanharmoniconForm::Matching(format!("TriggerCause::DealtDamage({cf})"))
            }
            Some(_) => return deny(self),
        };

        let cond_raw = self.condition_any(&mut p)?;
        if !p.exhausted() {
            return deny(self);
        }
        let cond_inner = cond_raw
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'));

        let expr = self.filter_expr(&valid_card)?;
        let filter = self.body.filter_static("TRIGGER_MULT", &expr);

        match (form, cond_inner) {
            (PanharmoniconForm::Legacy(event), None) => {
                self.body.abilities.push(format!(
                    "AbilityDef::Replacement(ReplacementRule::TriggerMultiplier {{ \
                     source_filter: &{filter}, event: {event} }})"
                ));
            }
            (form, cond_opt) => {
                let cause_expr = match form {
                    PanharmoniconForm::Legacy("TriggerEventKind::Any") => {
                        "TriggerCause::Any".to_string()
                    }
                    PanharmoniconForm::Legacy("TriggerEventKind::EntersBattlefield") => {
                        "TriggerCause::EntersBattlefield(&Filter::Any)".to_string()
                    }
                    PanharmoniconForm::Legacy("TriggerEventKind::Attack") => {
                        "TriggerCause::Attack(&Filter::CREATURE)".to_string()
                    }
                    PanharmoniconForm::Legacy(_) => unreachable!(),
                    PanharmoniconForm::Matching(c) => c,
                };
                let cond_field = match cond_opt {
                    None => "None".to_string(),
                    Some(c) => format!("Some({c})"),
                };
                self.body.abilities.push(format!(
                    "AbilityDef::Replacement(ReplacementRule::TriggerMultiplierMatching {{ \
                     source_filter: &{filter}, cause: {cause_expr}, condition: {cond_field} }})"
                ));
            }
        }
        Some(())
    }

    /// `AB$ Learn` / `SP$ Learn` / `DB$ Learn` (`CR 701.46`).
    pub(super) fn wave17a_learn_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        prior_target_used: bool,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || (target.is_some() && !prior_target_used)
            || !matches!(p.take("Defined").as_deref(), None | Some("You"))
        {
            return self.deny("effect `Learn`".to_string());
        }
        Some(vec!["Effect::Learn".to_string()])
    }

    /// Cast-provenance filter atoms (`wasCast*` / `!wasCast*`).
    pub(super) fn wave17a_was_cast_atom(atom: &str) -> Option<String> {
        Some(match atom {
            "wasCast" => "Filter::WasCast".to_string(),
            "!wasCast" => "Filter::Not(&Filter::WasCast)".to_string(),
            "wasCastByYou" => "Filter::WasCastByYou".to_string(),
            "!wasCastByYou" => "Filter::Not(&Filter::WasCastByYou)".to_string(),
            "wasCastFromGraveyard" => "Filter::CastFromZone(ZoneRef::Graveyard)".to_string(),
            "!wasCastFromGraveyard" => {
                "Filter::Not(&Filter::CastFromZone(ZoneRef::Graveyard))".to_string()
            }
            "wasCastFromGraveyardByYou" => {
                "Filter::And(&[Filter::CastFromZone(ZoneRef::Graveyard), Filter::WasCastByYou])"
                    .to_string()
            }
            "wasCastFromYourGraveyard" => {
                "Filter::And(&[Filter::CastFromZone(ZoneRef::Graveyard), Filter::OwnedByYou])"
                    .to_string()
            }
            "wasCastFromYourGraveyardByYou" => {
                "Filter::And(&[Filter::CastFromZone(ZoneRef::Graveyard), Filter::OwnedByYou, Filter::WasCastByYou])"
                    .to_string()
            }
            "wasCastFromExile" => "Filter::CastFromZone(ZoneRef::Exile)".to_string(),
            "!wasCastFromExile" => "Filter::Not(&Filter::CastFromZone(ZoneRef::Exile))".to_string(),
            "wasCastFromExileByYou" => {
                "Filter::And(&[Filter::CastFromZone(ZoneRef::Exile), Filter::WasCastByYou])"
                    .to_string()
            }
            "wasCastFromYourLibrary" => {
                "Filter::And(&[Filter::CastFromZone(ZoneRef::Library), Filter::OwnedByYou])"
                    .to_string()
            }
            "wasCastFromLibrary" => "Filter::CastFromZone(ZoneRef::Library)".to_string(),
            "wasCastFromCommand" => "Filter::CastFromZone(ZoneRef::Command)".to_string(),
            "wasCastFromYourHand" => {
                "Filter::And(&[Filter::CastFromZone(ZoneRef::Hand), Filter::OwnedByYou])"
                    .to_string()
            }
            "!wasCastFromYourHand" => {
                "Filter::Not(&Filter::And(&[Filter::CastFromZone(ZoneRef::Hand), Filter::OwnedByYou]))"
                    .to_string()
            }
            "wasCastFromYourHandByYou" => {
                "Filter::And(&[Filter::WasCastByYou, Filter::CastFromZone(ZoneRef::Hand), Filter::OwnedByYou])"
                    .to_string()
            }
            "!wasCastFromYourHandByYou" => {
                "Filter::Not(&Filter::And(&[Filter::WasCastByYou, Filter::CastFromZone(ZoneRef::Hand), Filter::OwnedByYou]))"
                    .to_string()
            }
            _ => return None,
        })
    }

    /// Whether a `T:` line's `ValidCard$` is a self-ETB cast-provenance trigger
    /// routed to `protection52::cast_self_protection_trigger`.
    pub(super) fn wave17a_routes_to_cast_self_trigger(mode: &str, valid: &str) -> bool {
        if valid.contains(',') || !valid.contains("Self") {
            return mode != "ChangesZone"
                && valid.contains("wasCastByYou")
                && valid.contains("Self");
        }
        mode == "ChangesZone"
            && (valid.contains("wasCastByYou")
                || valid.contains("wasCastFromYourHandByYou")
                || matches!(
                    valid.trim(),
                    "Card.Self+wasCast"
                        | "Card.wasCast+Self"
                        | "Creature.Self+wasCast"
                        | "Creature.wasCast+Self"
                        | "Card.Self+wasCastFromGraveyard"
                        | "Card.wasCastFromGraveyard+Self"
                        | "Creature.Self+wasCastFromGraveyard"
                        | "Creature.wasCastFromGraveyard+Self"
                        | "Card.Self+wasCastFromYourGraveyard"
                        | "Card.wasCastFromYourGraveyard+Self"
                        | "Creature.Self+wasCastFromYourGraveyard"
                        | "Creature.wasCastFromYourGraveyard+Self"
                        | "Card.Self+wasCastFromGraveyardByYou"
                        | "Card.wasCastFromGraveyardByYou+Self"
                        | "Creature.Self+wasCastFromGraveyardByYou"
                        | "Creature.wasCastFromGraveyardByYou+Self"
                        | "Card.Self+wasCastFromYourGraveyardByYou"
                        | "Card.wasCastFromYourGraveyardByYou+Self"
                        | "Creature.Self+wasCastFromYourGraveyardByYou"
                        | "Creature.wasCastFromYourGraveyardByYou+Self"
                        | "Card.Self+wasCastFromExile"
                        | "Card.wasCastFromExile+Self"
                        | "Creature.Self+wasCastFromExile"
                        | "Creature.wasCastFromExile+Self"
                ))
    }

    /// Condition clauses for `protection52::cast_self_protection_trigger`.
    pub(super) fn wave17a_cast_self_etb_clauses(&self, valid: &str) -> Option<Vec<String>> {
        match valid.trim() {
            "Card.wasCastByYou+Self"
            | "Card.Self+wasCastByYou"
            | "Creature.wasCastByYou+Self"
            | "Creature.Self+wasCastByYou" => Some(vec!["Condition::SourceWasCastByYou".into()]),
            "Card.wasCastFromYourHandByYou+Self"
            | "Card.Self+wasCastFromYourHandByYou"
            | "Creature.wasCastFromYourHandByYou+Self"
            | "Creature.Self+wasCastFromYourHandByYou" => Some(vec![
                "Condition::SourceWasCastByYou".into(),
                "Condition::SourceMatches(&Filter::CastFromHand)".into(),
            ]),
            "Card.Self+wasCast"
            | "Card.wasCast+Self"
            | "Creature.Self+wasCast"
            | "Creature.wasCast+Self" => {
                if self.oracle.to_ascii_lowercase().contains("if you cast") {
                    Some(vec!["Condition::SourceWasCastByYou".into()])
                } else {
                    Some(vec!["Condition::SourceWasCast".into()])
                }
            }
            "Card.Self+wasCastFromGraveyard"
            | "Card.wasCastFromGraveyard+Self"
            | "Creature.Self+wasCastFromGraveyard"
            | "Creature.wasCastFromGraveyard+Self" => Some(vec![
                "Condition::SourceCastFromZone(ZoneRef::Graveyard)".into(),
            ]),
            "Card.Self+wasCastFromYourGraveyard"
            | "Card.wasCastFromYourGraveyard+Self"
            | "Creature.Self+wasCastFromYourGraveyard"
            | "Creature.wasCastFromYourGraveyard+Self" => Some(vec![
                "Condition::SourceCastFromZone(ZoneRef::Graveyard)".into(),
                "Condition::SourceMatches(&Filter::OwnedByYou)".into(),
            ]),
            "Card.Self+wasCastFromGraveyardByYou"
            | "Card.wasCastFromGraveyardByYou+Self"
            | "Creature.Self+wasCastFromGraveyardByYou"
            | "Creature.wasCastFromGraveyardByYou+Self" => Some(vec![
                "Condition::SourceWasCastByYou".into(),
                "Condition::SourceCastFromZone(ZoneRef::Graveyard)".into(),
            ]),
            "Card.Self+wasCastFromYourGraveyardByYou"
            | "Card.wasCastFromYourGraveyardByYou+Self"
            | "Creature.Self+wasCastFromYourGraveyardByYou"
            | "Creature.wasCastFromYourGraveyardByYou+Self" => Some(vec![
                "Condition::SourceWasCastByYou".into(),
                "Condition::SourceCastFromZone(ZoneRef::Graveyard)".into(),
                "Condition::SourceMatches(&Filter::OwnedByYou)".into(),
            ]),
            "Card.Self+wasCastFromExile"
            | "Card.wasCastFromExile+Self"
            | "Creature.Self+wasCastFromExile"
            | "Creature.wasCastFromExile+Self" => {
                Some(vec!["Condition::SourceCastFromZone(ZoneRef::Exile)".into()])
            }
            _ => None,
        }
    }

    /// Chain/effect `ConditionPresent$` reader for self cast-provenance conditions
    /// (`Condition::CastFromOtherThanHand`, `Condition::SourceWasCast`,
    /// `Condition::SourceCastFromZone`).
    pub(super) fn wave17a_self_cast_condition(&mut self, p: &mut Params) -> SelfCastCond {
        if !matches!(p.peek("ConditionDefined"), None | Some("Self"))
            || p.has("PresentPlayer")
            || p.has("PresentZone")
        {
            return SelfCastCond::Pass;
        }
        let Some(present) = p.peek("ConditionPresent") else {
            return SelfCastCond::Pass;
        };
        let (base_cond, positive_present) = match present {
            "Card.wasCast+!wasCastFromYourHand"
            | "Card.!wasCastFromYourHand+wasCast"
            | "Card.Self+wasCast+!wasCastFromYourHand"
            | "Card.wasCast+!wasCastFromHand"
            | "Card.!wasCastFromHand+wasCast" => {
                ("Condition::CastFromOtherThanHand".to_string(), true)
            }
            "Card.wasCast" | "Card.Self+wasCast" | "Card.wasCast+Self" => {
                ("Condition::SourceWasCast".to_string(), true)
            }
            "Card.!wasCast" | "Card.Self+!wasCast" | "Card.!wasCast+Self" => {
                ("Condition::SourceWasCast".to_string(), false)
            }
            "Card.wasCastFromGraveyard"
            | "Card.Self+wasCastFromGraveyard"
            | "Card.wasCastFromGraveyard+Self" => (
                "Condition::SourceCastFromZone(ZoneRef::Graveyard)".to_string(),
                true,
            ),
            "Card.!wasCastFromGraveyard" | "Card.Self+!wasCastFromGraveyard" => (
                "Condition::SourceCastFromZone(ZoneRef::Graveyard)".to_string(),
                false,
            ),
            "Card.wasCastFromExile"
            | "Card.Self+wasCastFromExile"
            | "Card.wasCastFromExile+Self" => (
                "Condition::SourceCastFromZone(ZoneRef::Exile)".to_string(),
                true,
            ),
            "Card.!wasCastFromExile" | "Card.Self+!wasCastFromExile" => (
                "Condition::SourceCastFromZone(ZoneRef::Exile)".to_string(),
                false,
            ),
            _ => return SelfCastCond::Pass,
        };
        p.take("ConditionPresent");
        p.take("ConditionDefined");
        let cmp = p.take("PresentCompare");
        let positive = match cmp.as_deref() {
            None | Some("GE1" | "EQ1" | "GT0") => positive_present,
            Some("EQ0" | "LT1") => !positive_present,
            Some(other) => {
                return SelfCastCond::Matched(
                    self.deny(format!("wasCast condition compared `{other}`")),
                );
            }
        };
        let cast_cond = if positive {
            base_cond
        } else {
            format!("Condition::Not(&{base_cond})")
        };
        if p.has("ConditionPhases") {
            let mut clauses = vec![cast_cond];
            if Self::push_main_phase_clauses(p, &mut clauses).is_none() {
                return SelfCastCond::Matched(None);
            }
            SelfCastCond::Matched(Some(format!("Condition::All(&[{}])", clauses.join(", "))))
        } else {
            SelfCastCond::Matched(Some(cast_cond))
        }
    }

    /// Conditional `Count$wasCast*` `Amount` reader (`Increasing Devotion`,
    /// `Delayed Blast Fireball`, `Fblthp, the Lost`, `The Final Days`, etc.).
    pub(super) fn wave17a_count_expr(
        &mut self,
        def: &str,
        depth: u8,
        static_only: bool,
    ) -> Option<String> {
        if static_only || depth >= 3 || self.in_delayed {
            return None;
        }
        let def = def.trim();
        let (cond_expr, rest) = if let Some(r) = def.strip_prefix("Count$wasCastFromGraveyard.") {
            ("Condition::SourceCastFromZone(ZoneRef::Graveyard)", r)
        } else if let Some(r) = def.strip_prefix("Count$wasCastFromYourGraveyard.") {
            (
                "Condition::All(&[Condition::SourceCastFromZone(ZoneRef::Graveyard), Condition::SourceMatches(&Filter::OwnedByYou)])",
                r,
            )
        } else if let Some(r) = def.strip_prefix("Count$wasCastFromExile.") {
            ("Condition::SourceCastFromZone(ZoneRef::Exile)", r)
        } else if let Some(r) = def.strip_prefix("Count$wasCastFromYourLibrary.") {
            (
                "Condition::All(&[Condition::SourceCastFromZone(ZoneRef::Library), Condition::SourceMatches(&Filter::OwnedByYou)])",
                r,
            )
        } else if let Some(r) = def.strip_prefix("Count$wasCastFromLibrary.") {
            ("Condition::SourceCastFromZone(ZoneRef::Library)", r)
        } else if let Some(r) = def.strip_prefix("Count$wasCastFromCommand.") {
            ("Condition::SourceCastFromZone(ZoneRef::Command)", r)
        } else if let Some(r) = def.strip_prefix("Count$wasCastFromYourHandByYou.") {
            (
                "Condition::All(&[Condition::SourceWasCastByYou, Condition::SourceCastFromZone(ZoneRef::Hand), Condition::SourceMatches(&Filter::OwnedByYou)])",
                r,
            )
        } else if let Some(r) = def.strip_prefix("Count$wasCastFromYourHand.") {
            (
                "Condition::All(&[Condition::SourceCastFromZone(ZoneRef::Hand), Condition::SourceMatches(&Filter::OwnedByYou)])",
                r,
            )
        } else {
            let r = def.strip_prefix("Count$wasCast.")?;
            ("Condition::SourceWasCast", r)
        };

        let (then_raw, else_raw) = rest.split_once('.')?;
        if else_raw.contains('.') {
            return None;
        }
        let parse_branch = |tx: &mut Self, raw: &str| -> Option<String> {
            let s = raw.trim();
            if let Ok(n) = s.parse::<u32>() {
                return Some(format!("Amount::Fixed({n})"));
            }
            if !static_only
                && s == "X"
                && (tx.has_x || (tx.is_etb_self_trigger && tx.has_x_mana_cost))
                && tx.svars.get("X").map(String::as_str) == Some("Count$xPaid")
            {
                return Some("Amount::X".to_string());
            }
            let inner = tx.svars.get(s)?.clone();
            tx.count_expr_depth(&inner, depth + 1, static_only)
        };
        let then_amt = parse_branch(self, then_raw)?;
        let else_amt = parse_branch(self, else_raw)?;
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond_expr}, then: &{then_amt}, otherwise: &{else_amt} }}"
        ))
    }
}
