//! Codegen Wave 30 Lane D (`scriptgen`-only) lowerings & strict refusal boundaries:
//! - `DB$ Effect` with `RememberObjects$ Valid <F>` + `ReplacementEffects$ <R>`
//!   (`Event$ DamageDone | Prevent$ True | ValidTarget$ Card.IsRemembered`)
//!   -> `Effect::AddGlobalTempModifier` with `Modifier::PreventDamageFrom` /
//!   `PreventCombatDamageFrom` / `PreventNoncombatDamageFrom` (*Mutational Advantage*).
//! - `ChangeZone` (`Origin$ Graveyard`, `Hidden$ True`) targeting `Player` / `Opponent`
//!   with `Chooser$ Targeted` and `ChangeType$ ...TargetedPlayerOwn|TargetedPlayerCtrl`
//!   without a redundant `DefinedPlayer$` (*Obscura Confluence*, *Scrounge*).
//!
//! Pure Paper-MTG only: every candidate pattern that lacks an exact `baylee-cards-dsl`
//! or engine construct (e.g. player shroud, attack tax without planeswalkers, player
//! `OptionalDecider` with event-player counter placer, `withMorph`, graveyard-arrival
//! characteristic checks, and 2HG `OppOwn` team distinctions) remains strictly refused.

use super::{Params, Tx};

impl Tx<'_> {
    /// True when a `ChangeZone | Origin$ Graveyard | Hidden$ True` line targets
    /// `Player` or `Opponent`, asks the targeted player (`Chooser$ Targeted`), and
    /// scopes every `ChangeType$` alternative to `TargetedPlayerOwn` /
    /// `TargetedPlayerCtrl` rather than carrying a separate `DefinedPlayer$`.
    pub(super) fn wave30d_targeted_graveyard_player_choice(p: &Params) -> bool {
        p.peek("Chooser") == Some("Targeted")
            && p.peek("ChangeType").is_some_and(|ct| {
                !ct.trim().is_empty()
                    && ct.split(',').all(|alt| {
                        alt.split(['.', '+'])
                            .map(str::trim)
                            .any(|atom| matches!(atom, "TargetedPlayerOwn" | "TargetedPlayerCtrl"))
                    })
            })
    }

    /// Lower an untargeted `DB$ Effect` whose `RememberObjects$ Valid <F>`
    /// snapshots matching battlefield permanents for a turn-long `Prevent$ True`
    /// `DamageDone` replacement (*Mutational Advantage*).
    pub(super) fn wave30d_effect_remember_valid_prevention(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let (valid_raw, rep_name) = Self::parse_remember_valid_effect_header(p, target)?;
        let (require_creature, combat_only, noncombat_only, valid_source) =
            self.parse_damage_prevent_remembered_replacement(&rep_name)?;

        let base_expr = self.filter_expr(&valid_raw)?;
        if base_expr.contains("Filter::This") {
            return None;
        }
        let target_expr = if require_creature {
            format!("Filter::And(&[Filter::CREATURE, {base_expr}])")
        } else {
            base_expr
        };
        let filter_name = self.body.filter_static("EFFECT", &target_expr);

        let src_ref = match valid_source.as_deref() {
            None => "&Filter::Any".to_string(),
            Some(vs) => {
                if vs.contains("IsRemembered")
                    || vs.contains("EffectSource")
                    || vs.contains("RememberedPlayer")
                    || vs.contains("ChosenCard")
                    || Self::has_contextual_card_token(vs)
                {
                    return None;
                }
                let src_expr = self.filter_expr(vs)?;
                if src_expr.contains("Filter::This") {
                    return None;
                }
                if src_expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else {
                    format!("&{}", self.body.filter_static("SOURCE", &src_expr))
                }
            }
        };

        let modifier = match (combat_only, noncombat_only) {
            (true, false) => format!("Modifier::PreventCombatDamageFrom({src_ref})"),
            (false, true) => format!("Modifier::PreventNoncombatDamageFrom({src_ref})"),
            (false, false) => format!("Modifier::PreventDamageFrom({src_ref})"),
            (true, true) => unreachable!(),
        };

        // Snapshot matching permanents at resolution time via RepeatEach so that
        // PreventDamageFrom / PreventCombatDamageFrom / PreventNoncombatDamageFrom
        // bind to ObjectIs(id, version) rather than re-evaluating Filter::YouCtrl /
        // Filter::HasAnyCounter dynamically over the rest of the turn.
        Some(vec![format!(
            "Effect::RepeatEach {{ domain: RepeatDomain::EachMatchingPermanent(&{filter_name}), \
             effects: &[Effect::ModifyEvent {{ modifier: {modifier}, \
             duration: Duration::UntilEndOfTurn }}] }}"
        )])
    }

    fn has_contextual_card_token(s: &str) -> bool {
        s.contains("Triggered")
            || s.split([',', '.', '+', ' ', '!', '&', '|', ';'])
                .any(|tok| matches!(tok.trim(), "Self" | "StrictlySelf"))
    }

    fn parse_remember_valid_effect_header(
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<(String, String)> {
        if target.is_some()
            || p.peek("StaticAbilities").is_some()
            || p.peek("Triggers").is_some()
            || p.peek("Abilities").is_some()
            || p.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let valid_rem = p.take("RememberObjects")?;
        let valid_raw = valid_rem.strip_prefix("Valid ")?.trim().to_string();
        if valid_raw.is_empty()
            || valid_raw.contains("Remembered")
            || valid_raw.contains("EffectSource")
            || valid_raw.contains("ChosenCard")
            || valid_raw.contains("OppOwn")
            || valid_raw.contains("IsPresent")
            || Self::has_contextual_card_token(&valid_raw)
        {
            return None;
        }
        if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn")) {
            return None;
        }
        let moved = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if !matches!(moved.as_deref(), None | Some("Battlefield")) {
            return None;
        }
        p.drop_prose();
        p.take("Name");
        if let Some(non_stacking) = p.take("NonStackingEffect")
            && non_stacking != "True"
        {
            return None;
        }
        if let Some(stackable) = p.take("Stackable")
            && stackable != "False"
        {
            return None;
        }
        let rep_name = p.take("ReplacementEffects")?;
        if rep_name.contains(',') || !p.exhausted() {
            return None;
        }
        Some((valid_raw, rep_name))
    }

    fn parse_damage_prevent_remembered_replacement(
        &self,
        rep_name: &str,
    ) -> Option<(bool, bool, bool, Option<String>)> {
        let body = self.svars.get(rep_name.trim())?;
        let (event, mut r) = Params::parse(body)?;
        r.drop_prose();
        if event != "DamageDone"
            || r.take("Prevent").as_deref() != Some("True")
            || !matches!(
                r.take("ActiveZones").as_deref(),
                None | Some("Command" | "Battlefield")
            )
        {
            return None;
        }
        let require_creature = match r.take("ValidTarget").as_deref() {
            Some("Card.IsRemembered" | "Permanent.IsRemembered") => false,
            Some("Creature.IsRemembered") => true,
            _ => return None,
        };
        let (combat_only, noncombat_only) = match r.take("IsCombat").as_deref() {
            None => (false, false),
            Some("True") => (true, false),
            Some("False") => (false, true),
            Some(_) => return None,
        };
        let valid_source = r.take("ValidSource");
        r.exhausted()
            .then_some((require_creature, combat_only, noncombat_only, valid_source))
    }
}
