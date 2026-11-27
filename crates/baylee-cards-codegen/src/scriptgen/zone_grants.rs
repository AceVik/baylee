//! Hand-zone grants, off-board madness grants, and `ZonePlayGrant` companion
//! rider pairing (`CR 118.9`, `CR 601.3e`, `CR 702.35a`, `CR 702.74a`,
//! `CR 702.94a`, `CR 702.143a`, `CR 702.185a`, `CR 702.187a`).

use super::statics::{StaticEffectZone, parse_fixed_alt_mana_price, static_phase_condition};
use super::{CardScript, Params, Tx};
use std::collections::{BTreeMap, BTreeSet};

/// Companion `MayPlaySource` / `CantBeBeamedUp` metadata scanned per face before
/// lowering individual rules.
#[derive(Debug, Default)]
pub(super) struct PairedMayPlayState {
    /// Companion riders keyed by the primary `S:` rule index they belong to.
    pub(super) by_static_idx: BTreeMap<usize, PairedStaticKind>,
    /// Rule indices of paired companion `T:`/`R:`/`S:` lines already verified
    /// against an active static rule.
    pub(super) companion_indices: BTreeSet<usize>,
    /// Rule indices of `S:Mode$ Continuous` rules that actually consumed their
    /// paired companion entry.
    pub(super) consumed_statics: BTreeSet<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HandGrantOutcome {
    Pass,
    Handled(Option<()>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PairedStaticKind {
    ZonePlayRider(&'static str),
    ZonePlayDeathExileGainLife(u32),
    CantBeBeamedUpSelf,
}

impl PairedMayPlayState {
    pub(super) fn detect(script: &CardScript) -> Self {
        let mut state = Self::default();
        let mut finality_triggers = Vec::new();
        let mut animate_death_exile = Vec::new();
        let mut animate_leave_exile = Vec::new();
        let mut move_stack_to_exile = Vec::new();
        let mut cant_be_beamed_up = Vec::new();

        for (idx, (kind, spec)) in script.rules.iter().enumerate() {
            let Some((head, mut p)) = Params::parse(spec) else {
                continue;
            };
            match (*kind, head.as_str()) {
                ('T', "SpellCast")
                    if p.peek("ValidCard") == Some("Card.CastSa Spell.MayPlaySource") =>
                {
                    if is_finality_spell_cast_trigger(&p, &script.svars) {
                        finality_triggers.push(idx);
                    }
                }
                ('T', "SpellCast") if p.peek("ValidSA") == Some("Spell.MayPlaySource") => {
                    if let Some((exec, kind)) = classify_animate_spell_cast(&p, &script.svars) {
                        animate_death_exile.push((idx, exec, kind));
                    }
                }
                ('T', "LandPlayed") if p.peek("ValidSA") == Some("SpellAbility.MayPlaySource") => {
                    if let Some((exec, kind)) = classify_animate_land_played(&p, &script.svars) {
                        animate_leave_exile.push((idx, exec, kind));
                    }
                }
                ('R', "Moved") if p.peek("ValidLKI") == Some("Card.CastSa Spell.MayPlaySource") => {
                    p.drop_prose();
                    if is_stack_to_exile_replacement(&mut p, &script.svars) {
                        move_stack_to_exile.push(idx);
                    }
                }
                ('S', "CantBeBeamedUp") => {
                    p.drop_prose();
                    if is_cant_be_beamed_up_rule(&mut p) {
                        cant_be_beamed_up.push(idx);
                    }
                }
                _ => {}
            }
        }

        state.pair_single_static(
            script,
            &finality_triggers,
            PairedStaticKind::ZonePlayRider("ZonePlayCastRider::EntryFinality"),
            |mode, p| {
                mode == "Continuous"
                    && p.peek("MayPlay") == Some("True")
                    && matches!(p.peek("AffectedZone"), Some("Graveyard" | "Exile"))
                    && !p.has("ReplaceGraveyard")
            },
        );
        state.pair_animate_triggers(script, &animate_death_exile, &animate_leave_exile);
        state.pair_single_static(
            script,
            &move_stack_to_exile,
            PairedStaticKind::ZonePlayRider("ZonePlayCastRider::ExileInsteadOfGraveyard"),
            |mode, p| {
                mode == "Continuous"
                    && p.peek("EffectZone") == Some("Graveyard")
                    && p.peek("AffectedZone") == Some("Graveyard")
                    && p.peek("Affected") == Some("Card.Self")
                    && p.peek("MayPlay") == Some("True")
            },
        );
        state.pair_single_static(
            script,
            &cant_be_beamed_up,
            PairedStaticKind::CantBeBeamedUpSelf,
            |mode, p| {
                mode == "Continuous"
                    && p.peek("AffectedZone") == Some("Graveyard")
                    && p.peek("AddKeyword") == Some("Beam me up")
            },
        );
        state
    }

    fn pair_single_static(
        &mut self,
        script: &CardScript,
        companions: &[usize],
        kind: PairedStaticKind,
        matches_static: impl Fn(&str, &Params) -> bool,
    ) {
        if companions.len() != 1 {
            return;
        }
        let matching: Vec<usize> = script
            .rules
            .iter()
            .enumerate()
            .filter_map(|(idx, (k, spec))| {
                if *k != 'S' {
                    return None;
                }
                let (mode, p) = Params::parse(spec)?;
                matches_static(&mode, &p).then_some(idx)
            })
            .collect();
        if matching.len() == 1 && !self.by_static_idx.contains_key(&matching[0]) {
            self.companion_indices.insert(companions[0]);
            self.by_static_idx.insert(matching[0], kind);
        }
    }

    fn pair_animate_triggers(
        &mut self,
        script: &CardScript,
        spell_casts: &[(usize, String, PairedStaticKind)],
        land_plays: &[(usize, String, PairedStaticKind)],
    ) {
        if spell_casts.len() != 1 || land_plays.len() != 1 {
            return;
        }
        let (t1, ref ex1, k1) = spell_casts[0];
        let (t2, ref ex2, k2) = land_plays[0];
        if ex1 != ex2 || k1 != k2 {
            return;
        }
        let matching: Vec<usize> = script
            .rules
            .iter()
            .enumerate()
            .filter_map(|(idx, (k, spec))| {
                if *k != 'S' {
                    return None;
                }
                let (mode, p) = Params::parse(spec)?;
                (mode == "Continuous"
                    && p.peek("MayPlay") == Some("True")
                    && p.peek("AffectedZone") == Some("Graveyard")
                    && !p.has("ReplaceGraveyard"))
                .then_some(idx)
            })
            .collect();
        if matching.len() == 1 && !self.by_static_idx.contains_key(&matching[0]) {
            self.companion_indices.insert(t1);
            self.companion_indices.insert(t2);
            self.by_static_idx.insert(matching[0], k1);
        }
    }
}
fn is_finality_spell_cast_trigger(p_in: &Params, svars: &BTreeMap<String, String>) -> bool {
    let mut p = p_in.clone();
    p.drop_prose();
    if p.take("ValidCard").as_deref() != Some("Card.CastSa Spell.MayPlaySource")
        || p.take("Static").as_deref() != Some("True")
        || p.take("TriggerZones").as_deref() != Some("Battlefield")
    {
        return false;
    }
    let _ = p.take("Secondary");
    let Some(exec) = p.take("Execute") else {
        return false;
    };
    if !p.exhausted() {
        return false;
    }
    let Some(trig_spec) = svars.get(&exec) else {
        return false;
    };
    let Some((trig_api, mut tp)) = Params::parse(trig_spec) else {
        return false;
    };
    tp.drop_prose();
    if trig_api != "Effect"
        || tp.take("RememberObjects").as_deref() != Some("TriggeredCard")
        || !matches!(tp.take("Duration").as_deref(), None | Some("Permanent"))
    {
        return false;
    }
    let Some(rep_name) = tp.take("ReplacementEffects") else {
        return false;
    };
    if !tp.exhausted() {
        return false;
    }
    let Some(rep_spec) = svars.get(&rep_name) else {
        return false;
    };
    let Some((rep_mode, mut rp)) = Params::parse(rep_spec) else {
        return false;
    };
    rp.drop_prose();
    if rep_mode != "Moved"
        || rp.take("Origin").as_deref() != Some("Stack")
        || rp.take("Destination").as_deref() != Some("Battlefield")
        || rp.take("ValidCard").as_deref() != Some("Card.IsRemembered")
        || rp.take("ReplacementResult").as_deref() != Some("Updated")
    {
        return false;
    }
    let Some(etb_name) = rp.take("ReplaceWith") else {
        return false;
    };
    if !rp.exhausted() {
        return false;
    }
    let Some(etb_spec) = svars.get(&etb_name) else {
        return false;
    };
    let Some((etb_api, mut ep)) = Params::parse(etb_spec) else {
        return false;
    };
    ep.drop_prose();
    if etb_api != "PutCounter"
        || ep.take("ETB").as_deref() != Some("True")
        || ep.take("Defined").as_deref() != Some("ReplacedCard")
        || ep.take("CounterType").as_deref() != Some("FINALITY")
        || ep.take("CounterNum").as_deref() != Some("1")
    {
        return false;
    }
    if let Some(rem_name) = ep.take("SubAbility") {
        let Some(rem_spec) = svars.get(&rem_name) else {
            return false;
        };
        let Some((rem_api, mut remp)) = Params::parse(rem_spec) else {
            return false;
        };
        remp.drop_prose();
        if rem_api != "ChangeZone"
            || remp.take("Defined").as_deref() != Some("Self")
            || remp.take("Origin").as_deref() != Some("Command")
            || remp.take("Destination").as_deref() != Some("Exile")
            || !remp.exhausted()
        {
            return false;
        }
    }
    ep.exhausted()
}

fn classify_animate_spell_cast(
    p_in: &Params,
    svars: &BTreeMap<String, String>,
) -> Option<(String, PairedStaticKind)> {
    let mut p = p_in.clone();
    p.drop_prose();
    if p.take("ValidCard").as_deref() != Some("Card")
        || p.take("ValidSA").as_deref() != Some("Spell.MayPlaySource")
        || p.take("Static").as_deref() != Some("True")
        || p.take("TriggerZones").as_deref() != Some("Battlefield")
    {
        return None;
    }
    let _ = p.take("Secondary");
    let exec = p.take("Execute")?;
    if !p.exhausted() {
        return None;
    }
    let kind = classify_animate_svar(&exec, svars)?;
    Some((exec, kind))
}

fn classify_animate_land_played(
    p_in: &Params,
    svars: &BTreeMap<String, String>,
) -> Option<(String, PairedStaticKind)> {
    let mut p = p_in.clone();
    p.drop_prose();
    if p.take("ValidCard").as_deref() != Some("Land")
        || p.take("ValidSA").as_deref() != Some("SpellAbility.MayPlaySource")
        || p.take("Static").as_deref() != Some("True")
        || p.take("TriggerZones").as_deref() != Some("Battlefield")
    {
        return None;
    }
    let _ = p.take("Secondary");
    let exec = p.take("Execute")?;
    if !p.exhausted() {
        return None;
    }
    let kind = classify_animate_svar(&exec, svars)?;
    Some((exec, kind))
}

fn classify_animate_svar(exec: &str, svars: &BTreeMap<String, String>) -> Option<PairedStaticKind> {
    let anim_spec = svars.get(exec)?;
    let (api, mut ap) = Params::parse(anim_spec)?;
    ap.drop_prose();
    if api != "Animate"
        || ap.take("Defined").as_deref() != Some("TriggeredCardLKICopy")
        || ap.take("Duration").as_deref() != Some("Permanent")
    {
        return None;
    }
    if let Some(trig_name) = ap.take("Triggers") {
        if !ap.exhausted() {
            return None;
        }
        let trig_spec = svars.get(&trig_name)?;
        let (mode, mut tp) = Params::parse(trig_spec)?;
        tp.drop_prose();
        if mode != "ChangesZone"
            || tp.take("Origin").as_deref() != Some("Battlefield")
            || tp.take("Destination").as_deref() != Some("Graveyard")
            || tp.take("ValidCard").as_deref() != Some("Card.Self")
        {
            return None;
        }
        let ex_name = tp.take("Execute")?;
        if !tp.exhausted() {
            return None;
        }
        let ex_spec = svars.get(&ex_name)?;
        let (ex_api, mut ep) = Params::parse(ex_spec)?;
        ep.drop_prose();
        if ex_api != "ChangeZone"
            || ep.take("Defined").as_deref() != Some("TriggeredNewCardLKICopy")
            || ep.take("Origin").as_deref() != Some("Graveyard")
            || ep.take("Destination").as_deref() != Some("Exile")
        {
            return None;
        }
        let gl_name = ep.take("SubAbility")?;
        if !ep.exhausted() {
            return None;
        }
        let gl_spec = svars.get(&gl_name)?;
        let (gl_api, mut gp) = Params::parse(gl_spec)?;
        gp.drop_prose();
        if gl_api != "GainLife" {
            return None;
        }
        let life: u32 = gp.take("LifeAmount")?.parse().ok()?;
        if !matches!(gp.take("Defined").as_deref(), None | Some("You")) || !gp.exhausted() {
            return None;
        }
        return Some(PairedStaticKind::ZonePlayDeathExileGainLife(life));
    }
    if let Some(rep_name) = ap.take("Replacements") {
        if !ap.exhausted() {
            return None;
        }
        let rep_spec = svars.get(&rep_name)?;
        let (mode, mut rp) = Params::parse(rep_spec)?;
        rp.drop_prose();
        if mode != "Moved"
            || rp.take("ActiveZones").as_deref() != Some("Battlefield")
            || rp.take("Origin").as_deref() != Some("Battlefield")
            || rp.take("ExcludeDestination").as_deref() != Some("Exile")
            || rp.take("ValidCard").as_deref() != Some("Card.Self")
        {
            return None;
        }
        let rw_name = rp.take("ReplaceWith")?;
        if !rp.exhausted() {
            return None;
        }
        let rw_spec = svars.get(&rw_name)?;
        let (rw_api, mut rwp) = Params::parse(rw_spec)?;
        rwp.drop_prose();
        if rw_api != "ChangeZone"
            || rwp.take("Origin").as_deref() != Some("Battlefield")
            || rwp.take("Destination").as_deref() != Some("Exile")
            || rwp.take("Defined").as_deref() != Some("ReplacedCard")
            || !rwp.exhausted()
        {
            return None;
        }
        return Some(PairedStaticKind::ZonePlayRider(
            "ZonePlayCastRider::GrantLeaveExileReplacement",
        ));
    }
    None
}

fn is_stack_to_exile_replacement(p: &mut Params, svars: &BTreeMap<String, String>) -> bool {
    if p.take("ValidLKI").as_deref() != Some("Card.CastSa Spell.MayPlaySource")
        || p.take("Origin").as_deref() != Some("Stack")
        || p.take("Destination").as_deref() != Some("Graveyard")
    {
        return false;
    }
    let Some(rw_name) = p.take("ReplaceWith") else {
        return false;
    };
    if !p.exhausted() {
        return false;
    }
    let Some(rw_spec) = svars.get(&rw_name) else {
        return false;
    };
    let Some((rw_api, mut rwp)) = Params::parse(rw_spec) else {
        return false;
    };
    rwp.drop_prose();
    rw_api == "ChangeZone"
        && rwp.take("Defined").as_deref() == Some("ReplacedCard")
        && rwp.take("Origin").as_deref() == Some("Stack")
        && rwp.take("Destination").as_deref() == Some("Exile")
        && rwp.exhausted()
}

fn is_cant_be_beamed_up_rule(p: &mut Params) -> bool {
    p.take("ValidCard").as_deref() == Some("Card.Self")
        && p.take("ValidSA").as_deref() == Some("Spell.KeywordSource")
        && p.take("Secondary").as_deref() == Some("True")
        && p.exhausted()
}

fn is_supported_hand_grant_kw(kw: &str) -> bool {
    let kw = kw.trim();
    kw == "Foretell"
        || kw.starts_with("Miracle:")
        || kw.starts_with("Warp:")
        || kw.starts_with("Evoke:")
}

fn strip_without_foretell_affected(affected: &str) -> Option<String> {
    let mut cleaned = Vec::new();
    for raw_alt in affected.split(',') {
        let alt = raw_alt.trim();
        let (base, qual) = alt.split_once('.')?;
        let mut saw_without_foretell = false;
        let mut saw_nonland = false;
        let mut kept = Vec::new();
        for atom in qual.split('+').map(str::trim) {
            if atom == "withoutForetell" {
                saw_without_foretell = true;
            } else {
                if atom == "nonLand" {
                    saw_nonland = true;
                }
                kept.push(atom);
            }
        }
        if !saw_without_foretell || !saw_nonland {
            return None;
        }
        if kept.is_empty() {
            cleaned.push(base.trim().to_string());
        } else {
            cleaned.push(format!("{}.{}", base.trim(), kept.join("+")));
        }
    }
    Some(cleaned.join(","))
}

impl Tx<'_> {
    pub(super) fn is_consumed_may_play_companion(&self, rule_idx: usize) -> bool {
        self.paired_may_play.companion_indices.contains(&rule_idx)
    }

    pub(super) fn has_paired_may_play_on_current_rule(&self) -> bool {
        self.current_rule_idx
            .is_some_and(|idx| self.paired_may_play.by_static_idx.contains_key(&idx))
    }

    pub(super) fn has_cant_be_beamed_up_for_current_rule(&mut self) -> bool {
        let Some(idx) = self.current_rule_idx else {
            return false;
        };
        let matched = self
            .paired_may_play
            .by_static_idx
            .get(&idx)
            .is_some_and(|&kind| kind == PairedStaticKind::CantBeBeamedUpSelf);
        if matched {
            self.paired_may_play.consumed_statics.insert(idx);
        }
        matched
    }

    pub(super) fn claim_paired_zone_play_rider(&mut self) -> Option<String> {
        let idx = self.current_rule_idx?;
        let kind = *self.paired_may_play.by_static_idx.get(&idx)?;
        let rider = match kind {
            PairedStaticKind::ZonePlayRider(expr) => expr.to_string(),
            PairedStaticKind::ZonePlayDeathExileGainLife(n) => {
                format!("ZonePlayCastRider::GrantDeathExileAndGainLife({n})")
            }
            PairedStaticKind::CantBeBeamedUpSelf => return None,
        };
        self.paired_may_play.consumed_statics.insert(idx);
        Some(rider)
    }

    pub(super) fn paired_may_play_all_consumed(&mut self) -> Option<()> {
        for &s_idx in self.paired_may_play.by_static_idx.keys() {
            if !self.paired_may_play.consumed_statics.contains(&s_idx) {
                return self.deny("unpaired `MayPlaySource` companion rule".to_string());
            }
        }
        Some(())
    }

    pub(super) fn cant_be_beamed_up_static(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !is_cant_be_beamed_up_rule(&mut p) {
            return self.deny("static ability `S: Mode$ CantBeBeamedUp`".to_string());
        }
        if !self
            .body
            .abilities
            .iter()
            .any(|a| a.contains("Modifier::GrantsBeamMeUpManaCost { cant_return_source: true }"))
        {
            return self.deny("`S: Mode$ CantBeBeamedUp` without `Beam me up` grant".to_string());
        }
        Some(())
    }

    /// Hand-zone continuous static grants (`RevealOpponentsHands`, `GrantMiracle`,
    /// `GrantForetell`, `GrantWarp`, `GrantEvoke`). Returns `None` outside these
    /// families so normal `reveal_all_hands` or refusal reporting still applies.
    pub(super) fn try_hand_grant_static(
        &mut self,
        p: &mut Params,
        effect_zone: Option<&'static str>,
    ) -> HandGrantOutcome {
        if p.peek("AffectedZone") != Some("Hand") || p.has("MayPlay") {
            return HandGrantOutcome::Pass;
        }
        if p.peek("MayLookAt") == Some("Player") && p.peek("Affected") == Some("Card.OppOwn") {
            return HandGrantOutcome::Handled(
                self.reveal_opponents_hands_static(p.clone(), effect_zone),
            );
        }
        if p.peek("AddKeyword").is_some_and(is_supported_hand_grant_kw) {
            return HandGrantOutcome::Handled(
                self.hand_keyword_grant_static(p.clone(), effect_zone),
            );
        }
        HandGrantOutcome::Pass
    }

    fn reveal_opponents_hands_static(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
    ) -> Option<()> {
        p.take("AffectedZone");
        p.take("Affected");
        p.take("MayLookAt");
        let cond = self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        let zone = match effect_zone {
            None => StaticEffectZone::Battlefield,
            Some(z) => StaticEffectZone::Explicit(z),
        };
        self.body.abilities.push(Self::format_static_expr(
            "Filter::Any",
            "Modifier::RevealOpponentsHands",
            zone,
            &cond,
        ));
        Some(())
    }

    fn hand_keyword_grant_static(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
    ) -> Option<()> {
        p.take("AffectedZone");
        let Some(raw_kw) = p.take("AddKeyword") else {
            return self.deny("static ability reaching `AffectedZone$ Hand`".to_string());
        };
        let kw = raw_kw.trim();
        let Some(affected) = p.take("Affected") else {
            return self.deny("static ability reaching `AffectedZone$ Hand`".to_string());
        };
        let (modifier, filter) = if kw == "Foretell" {
            let Some(cleaned) = strip_without_foretell_affected(&affected) else {
                return self.deny("static ability reaching `AffectedZone$ Hand`".to_string());
            };
            let filter = self.filter_expr_in_zone(&cleaned, true)?;
            (
                "Modifier::GrantForetell { reduce_generic: 2 }".to_string(),
                filter,
            )
        } else if let Some(spec) = kw.strip_prefix("Miracle:") {
            let grant = if let Some(less_raw) = spec.strip_prefix("ManaCost:") {
                let Ok(n) = less_raw.trim().parse::<u32>() else {
                    return self.deny(format!("`Continuous.AddKeyword$ {kw}`"));
                };
                format!("MiracleGrant::ManaCostLessGeneric({n})")
            } else if let Some(price) = parse_fixed_alt_mana_price(spec)
                && let Some(mc) = price
                    .strip_prefix("AltCastPrice::Mana(")
                    .and_then(|s| s.strip_suffix(')'))
            {
                format!("MiracleGrant::Fixed({mc})")
            } else {
                return self.deny(format!("`Continuous.AddKeyword$ {kw}`"));
            };
            let filter = self.filter_expr_in_zone(&affected, true)?;
            (format!("Modifier::GrantMiracle({grant})"), filter)
        } else if let Some(spec) = kw.strip_prefix("Warp:") {
            let cost = self.cost_expr(spec.trim())?;
            let filter = self.filter_expr_in_zone(&affected, true)?;
            (format!("Modifier::GrantWarp({cost})"), filter)
        } else if let Some(spec) = kw.strip_prefix("Evoke:") {
            let cost = self.cost_expr(spec.trim())?;
            let filter = self.filter_expr_in_zone(&affected, true)?;
            (format!("Modifier::GrantEvoke({cost})"), filter)
        } else {
            return self.deny("static ability reaching `AffectedZone$ Hand`".to_string());
        };
        let cond = self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() {
            return self.deny("static ability reaching `AffectedZone$ Hand`".to_string());
        }
        let zone = match effect_zone {
            None => StaticEffectZone::Battlefield,
            Some(z) => StaticEffectZone::Explicit(z),
        };
        self.body
            .abilities
            .push(Self::format_static_expr(&filter, &modifier, zone, &cond));
        Some(())
    }

    /// Off-battlefield Madness grant (`AffectedZone$ Hand,Library,Graveyard,Exile,Command,Stack |
    /// AddKeyword$ Madness:<ManaCost|CardManaCost>`, e.g. *Falkenrath Gorger*).
    pub(super) fn offboard_madness_static(
        &mut self,
        mut p: Params,
        effect_zone: Option<&'static str>,
        raw_zone: &str,
    ) -> Option<()> {
        let deny = |tx: &Self| {
            tx.deny(format!(
                "static ability reaching `AffectedZone$ {raw_zone}`"
            ))
        };
        if p.has("AffectedDefined")
            || !matches!(
                p.take("AddKeyword").as_deref().map(str::trim),
                Some("Madness:ManaCost" | "Madness:CardManaCost")
            )
        {
            return deny(self);
        }
        let Some(affected) = p.take("Affected") else {
            return deny(self);
        };
        let base = self.filter_expr_in_zone(&affected, true)?;
        let filter = if base == "Filter::Any" {
            "Filter::InZone(ZoneRef::NotBattlefield)".to_string()
        } else {
            format!("Filter::And(&[{base}, Filter::InZone(ZoneRef::NotBattlefield)])")
        };
        let cond = self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        self.strip_ai_hint_add_svar(&mut p);
        if !p.exhausted() {
            return deny(self);
        }
        let zone = match effect_zone {
            None => StaticEffectZone::Battlefield,
            Some(z) => StaticEffectZone::Explicit(z),
        };
        self.body.abilities.push(Self::format_static_expr(
            &filter,
            "Modifier::GrantMadnessManaCost",
            zone,
            &cond,
        ));
        Some(())
    }

    /// Self graveyard alternative cast with a paired stack-to-graveyard exile
    /// replacement (*Glimpse the Cosmos*).
    pub(super) fn continuous_graveyard_self_alt_play(&mut self, mut p: Params) -> Option<()> {
        let deny =
            |tx: &Self| tx.deny("static ability reaching `AffectedZone$ Graveyard`".to_string());
        p.take("AffectedZone");
        if p.take("Affected").as_deref() != Some("Card.Self")
            || p.take("MayPlay").as_deref() != Some("True")
        {
            return deny(self);
        }
        let Some(rider) = self.claim_paired_zone_play_rider() else {
            return deny(self);
        };
        let Some(price) = self.take_alt_cast_price(&mut p) else {
            return deny(self);
        };
        let cond = self.static_conditions(&mut p, "unclaimed parameter `Continuous.CheckSVar`")?;
        if !p.exhausted() {
            return deny(self);
        }
        let modifier = format!(
            "Modifier::ZonePlayGrant(ZonePlayGrant {{ who: PlayerRel::You, zones: AltCastZones::Zones(&[ZoneRef::Graveyard]), grant_zone_permission: true, allow_lands: false, with_flash: false, max_mana_value: None, once_per_turn: false, price: {price}, mana_spending: ZonePlayManaSpending::Exact, rider: {rider}, your_turn_only: false, as_sorcery_only: false, mana_cost_must_contain: None }})"
        );
        self.body.abilities.push(Self::format_static_expr(
            "Filter::This",
            &modifier,
            StaticEffectZone::Explicit("ZoneRef::Graveyard"),
            &cond,
        ));
        Some(())
    }

    pub(super) fn parse_valid_after_stack_clause(raw: &str) -> Option<ValidAfterStackClause> {
        let qual = raw.trim().strip_prefix("Spell.")?;
        if qual == "nonCreature" {
            return Some(ValidAfterStackClause::NonCreature);
        }
        if let Some(rhs) = qual.strip_prefix("cmcLE")
            && !rhs.is_empty()
        {
            return Some(ValidAfterStackClause::CmcLe(rhs.to_string()));
        }
        None
    }

    pub(super) fn take_zone_play_mana_spending(
        &mut self,
        p: &mut Params,
        desc: Option<&str>,
    ) -> Option<&'static str> {
        let ignore_color = match p.take("MayPlayIgnoreColor").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`Continuous.MayPlayIgnoreColor$ {other}`"));
            }
        };
        let ignore_type = match p.take("MayPlayIgnoreType").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`Continuous.MayPlayIgnoreType$ {other}`"));
            }
        };
        let snow_ignore_color = match p.take("MayPlaySnowIgnoreColor").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`Continuous.MayPlaySnowIgnoreColor$ {other}`"));
            }
        };
        if snow_ignore_color {
            if ignore_color || ignore_type {
                return self.deny("conflicting `MayPlayIgnore*` parameters".to_string());
            }
            return Some("ZonePlayManaSpending::SnowAsAnyColor");
        }
        if ignore_color {
            return Some("ZonePlayManaSpending::AnyColor");
        }
        if ignore_type {
            let desc_lower = desc.map_or_default(str::to_ascii_lowercase);
            let oracle = self.oracle.to_ascii_lowercase();
            let text = if desc_lower.is_empty() {
                &oracle
            } else {
                &desc_lower
            };
            if text.contains("any type") {
                return Some("ZonePlayManaSpending::AnyType");
            }
            if text.contains("any color") {
                return Some("ZonePlayManaSpending::AnyColor");
            }
            return self.deny(
                "`Continuous.MayPlayIgnoreType` without explicit spending scope".to_string(),
            );
        }
        Some("ZonePlayManaSpending::Exact")
    }
}

pub(super) enum ValidAfterStackClause {
    NonCreature,
    CmcLe(String),
}

pub(super) fn has_phases_cond(p: &mut Params) -> Result<Option<&'static str>, ()> {
    let Some(raw) = p.take("Phases") else {
        return Ok(None);
    };
    static_phase_condition(&raw).map(Some).ok_or(())
}
