//! Wave 30a pure-`scriptgen` lowerings for `Effect`, `ChangeZone`,
//! `ChangeZoneAll`, `RememberChanged`, `TargetMin`, and `Battlefield -> Library`:
//!
//! 1. **`Effect` API extensions (`wave30a_effect_api`)**:
//!    - `Mode$ DamageAll | CombatDamage$ True | ValidTarget$ Player` ->
//!      `Effect::WheneverOneOrMoreDealCombatDamageToPlayerThisTurn` (*Forth Eorlingas!*).
//!    - `DB$/SP$/AB$ Effect | ImprintCards$ Targeted | Triggers$ ...` ->
//!      `Effect::RegisterDelayedTrigger` with `TriggerRemember::FirstTarget`
//!      (*Fire Giant's Fury*).
//! 2. **Amount extensions (`wave30a_amount_ext`)**:
//!    - Inline `Count$LastDieRoll` -> `Amount::LastDieRoll` (*Bag of Devouring*).
//!    - `TriggerCount$ScryNum` on `Mode$ Scry` triggers -> `Amount::EventAmount`
//!      (*Elminster*).
//! 3. **`RememberChanged` chains (`wave30a_remembered_chain`)**:
//!    - `Graveyard -> Exile | RememberChanged$ True` -> `Play | CopyCard$ True |
//!      Amount$ <N> | AllowRepeats$ True` -> `Effect::CaptureAffected` +
//!      `Effect::ForEachRemembered` + `Effect::RepeatEach(Times(N), Play(copy))`
//!      (*Mnemonic Deluge*).
//!    - `Library -> Graveyard | ChangeNum$ 1 | Shuffle$ True | RememberChanged$ True`
//!      -> `Clone | Defined$ Remembered` -> `Effect::CaptureAffected(SearchLibrary(GRAVEYARD))`
//!      + `Effect::ForEachRemembered(BecomeCopy)` (*Scion of the Ur-Dragon*).

use super::{Params, Tx};

impl Tx<'_> {
    pub(super) fn wave30a_amount_ext(&self, _raw: &str, resolved: &str) -> Option<String> {
        if resolved == "Count$LastDieRoll" {
            return Some("Amount::LastDieRoll".to_string());
        }
        if resolved == "TriggerCount$ScryNum" && self.trigger_mode.as_deref() == Some("Scry") {
            return Some("Amount::EventAmount".to_string());
        }
        None
    }

    pub(super) fn wave30a_effect_api(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if let Some(effects) = self.wave30a_combat_damage_to_player_effect(p, target) {
            return Some(effects);
        }
        if let Some(effects) = self.wave30a_imprint_effect_trigger(p, target) {
            return Some(effects);
        }
        None
    }

    /// `Effect | Triggers$ <DamageAll>` (`ValidTarget$ Player | CombatDamage$ True`)
    /// -> `Effect::WheneverOneOrMoreDealCombatDamageToPlayerThisTurn` (*Forth Eorlingas!*).
    fn wave30a_combat_damage_to_player_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some()
            || p.has("StaticAbilities")
            || p.has("ReplacementEffects")
            || p.has("Abilities")
            || p.has("RememberObjects")
            || p.has("ImprintCards")
            || p.has("Defined")
            || p.has("Boon")
            || p.has("EffectOwner")
            || p.has("SetChosenNumber")
            || p.has("Planeswalker")
            || p.has("ExileOnMoved")
            || p.has("ForgetOnMoved")
        {
            return None;
        }
        if !matches!(
            p.take("Duration").as_deref(),
            None | Some("UntilEndOfTurn" | "EndOfTurn" | "End of Turn")
        ) {
            return None;
        }
        p.take("Name");
        p.take("IsCurse");
        if let Some(s) = p.take("Stackable")
            && !matches!(s.as_str(), "True" | "False")
        {
            return None;
        }
        if let Some(ns) = p.take("NonStackingEffect")
            && ns != "True"
        {
            return None;
        }
        let triggers = p.take("Triggers")?;
        if triggers.contains(',') || !p.exhausted() {
            return None;
        }
        let trig_body = self.svars.get(triggers.trim())?.clone();
        let (mode, mut tp) = Params::parse(&trig_body)?;
        tp.drop_prose();
        if mode != "DamageAll"
            || tp.take("CombatDamage").as_deref() != Some("True")
            || tp.take("ValidTarget").as_deref() != Some("Player")
            || !matches!(tp.take("TriggerZones").as_deref(), None | Some("Command"))
        {
            return None;
        }
        if let Some(sec) = tp.take("Secondary")
            && sec != "True"
        {
            return None;
        }
        if !matches!(tp.take("OneOff").as_deref(), None | Some("False")) {
            return None;
        }
        let optional = match tp.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        let valid_source = tp.take("ValidSource")?;
        if valid_source.contains("IsRemembered")
            || valid_source.contains("EffectSource")
            || valid_source.contains("IsImprinted")
            || valid_source.contains("ChosenCard")
        {
            return None;
        }
        let exec_name = tp.take("Execute")?;
        let exec_raw = self.svars.get(exec_name.trim())?.clone();
        if !tp.exhausted()
            || exec_raw.contains("Remembered")
            || exec_raw.contains("Triggered")
            || exec_raw.contains("TriggerCount$")
            || exec_raw.contains("Imprinted")
        {
            return None;
        }
        let filter_expr = self.filter_expr(&valid_source)?;
        let filter_ref = match filter_expr.as_str() {
            "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
            "Filter::YOUR_CREATURE" => "&Filter::YOUR_CREATURE".to_string(),
            "Filter::OPPONENT_CREATURE" => "&Filter::OPPONENT_CREATURE".to_string(),
            _ => format!("&{}", self.body.filter_static("COMBAT_DMG", &filter_expr)),
        };
        let (effects, targets) = self.parse_delayed_body_str(&exec_raw, None, optional)?;
        if targets.is_some() {
            return None;
        }
        Some(vec![format!(
            "Effect::WheneverOneOrMoreDealCombatDamageToPlayerThisTurn {{ filter: {filter_ref}, \
             effects: &[{effects}] }}"
        )])
    }

    /// `DB$/SP$/AB$ Effect | ImprintCards$ Targeted | Triggers$ <T>`
    /// -> `Effect::RegisterDelayedTrigger` with `TriggerRemember::FirstTarget`
    /// (*Fire Giant's Fury*).
    #[allow(clippy::too_many_lines)]
    fn wave30a_imprint_effect_trigger(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let target_spec = target?;
        if !target_spec.starts_with("TargetSpec::Object(")
            || p.peek("ImprintCards") != Some("Targeted")
            || p.has("StaticAbilities")
            || p.has("ReplacementEffects")
            || p.has("Abilities")
            || p.has("RememberObjects")
            || p.has("Boon")
            || p.has("EffectOwner")
            || p.has("Planeswalker")
            || p.has("Defined")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("ImprintCards");
        let (repeat_dur, oneshot_dur) = match probe.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn" | "EndOfTurn" | "End of Turn") => (
                "DelayedDuration::ThisTurn",
                Some("DelayedDuration::OneShotThisTurn"),
            ),
            Some("UntilEndOfCombat") => ("DelayedDuration::ThisCombat", None),
            Some("UntilYourNextTurn") => ("DelayedDuration::UntilYourNextTurn", None),
            Some(_) => return None,
        };
        let ends = probe
            .take("ExileOnMoved")
            .or_else(|| probe.take("ForgetOnMoved"));
        if !matches!(ends.as_deref(), None | Some("Battlefield" | "Graveyard")) {
            return None;
        }
        probe.take("IsCurse");
        probe.take("Name");
        if let Some(ns) = probe.take("NonStackingEffect")
            && ns != "True"
        {
            return None;
        }
        let triggers = probe.take("Triggers")?;
        if !probe.exhausted() {
            return None;
        }

        let mut out = Vec::new();
        for svar_name in triggers.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let body = self.svars.get(svar_name)?.clone();
            let (mode, mut tp) = Params::parse(&body)?;
            tp.drop_prose();
            if !matches!(
                mode.as_str(),
                "ChangesZone"
                    | "DamageDone"
                    | "DamageDoneOnce"
                    | "DamageDealtOnce"
                    | "Attacks"
                    | "Blocks"
                    | "BecomesTarget"
                    | "Taps"
                    | "Untaps"
                    | "Sacrificed"
            ) {
                return None;
            }
            if !matches!(
                tp.take("TriggerZones").as_deref(),
                None | Some("Command" | "Battlefield")
            ) {
                return None;
            }
            if let Some(sec) = tp.take("Secondary")
                && sec != "True"
            {
                return None;
            }
            if ends.as_deref() == Some("Graveyard")
                && !(mode == "ChangesZone"
                    && tp.peek("Origin") == Some("Battlefield")
                    && tp.peek("Destination") == Some("Graveyard"))
            {
                return None;
            }
            let one_off = match (
                tp.take("OneOff").as_deref(),
                tp.take("TriggerOnce").as_deref(),
            ) {
                (None | Some("False"), None | Some("False")) => false,
                (Some("True"), None | Some("False")) | (None | Some("False"), Some("True")) => true,
                _ => return None,
            };
            let delayed_dur = if one_off { oneshot_dur? } else { repeat_dur };
            let optional = match tp.take("OptionalDecider").as_deref() {
                None => false,
                Some("You") => true,
                Some(_) => return None,
            };
            let mut uses_watch = false;
            for (k, v) in &mut tp.entries {
                if !matches!(k.as_str(), "ValidCard" | "ValidSource" | "ValidTarget") {
                    continue;
                }
                if v.contains("IsImprinted") {
                    *v = Self::rewrite_imprinted_atom(v)?;
                    uses_watch = true;
                }
            }
            if !uses_watch
                || tp.entries.iter().any(|(_, v)| {
                    v.contains("IsRemembered")
                        || v.contains("IsImprinted")
                        || v.contains("EffectSource")
                        || v.contains("RememberedPlayer")
                        || v.contains("ChosenCard")
                })
            {
                return None;
            }
            let trigger_expr = self.trigger_expr_in_zone(&mut tp, &mode, true)?;
            if !tp.entries.iter().all(|(k, _)| k == "Execute") {
                return None;
            }
            let execute = tp.take("Execute")?;
            let exec_raw = self.svars.get(execute.trim())?.clone();
            if exec_raw.contains("Imprinted") || exec_raw.contains("Defined$ Remembered") {
                return None;
            }
            let (effects, targets) = {
                let saved_imp = std::mem::replace(&mut self.wave30a_allow_clear_imprinted, true);
                let res = self.parse_delayed_body_str(&exec_raw, Some(mode), optional);
                self.wave30a_allow_clear_imprinted = saved_imp;
                res?
            };
            let targets_expr =
                targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
            out.push(format!(
                "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
                 trigger: &{trigger_expr}, watch: TriggerRemember::FirstTarget, duration: {delayed_dur} }}, \
                 condition: None, targets: {targets_expr}, remember: TriggerRemember::None, \
                 effects: &[{effects}] }}"
            ));
        }
        *p = probe;
        if out.is_empty() {
            return None;
        }
        self.wave30a_allow_clear_imprinted = true;
        Some(out)
    }

    fn rewrite_imprinted_atom(valid: &str) -> Option<String> {
        let mut out = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, rest) = alt.split_once('.').unwrap_or((alt, ""));
            let mut saw = false;
            let mut kept = Vec::new();
            for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                if atom == "IsImprinted" {
                    saw = true;
                } else {
                    kept.push(atom);
                }
            }
            if !saw {
                return None;
            }
            if kept.is_empty() {
                out.push(format!("{base}.Self"));
            } else {
                out.push(format!("{base}.Self+{}", kept.join("+")));
            }
        }
        Some(out.join(","))
    }

    pub(super) fn wave30a_remembered_chain(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        if api == "ChangeZone"
            && targets_here
            && single_target
            && p.peek("Origin") == Some("Graveyard")
            && p.peek("Destination") == Some("Exile")
            && p.peek("RememberChanged") == Some("True")
            && let Some(effs) = self.wave30a_exile_and_cast_n_copies_chain(p, sub, target)
        {
            return Some(effs);
        }
        if api == "ChangeZone"
            && !targets_here
            && target.is_none()
            && p.peek("Origin") == Some("Library")
            && p.peek("Destination") == Some("Graveyard")
            && p.peek("RememberChanged") == Some("True")
            && let Some(effs) = self.wave30a_tutor_graveyard_clone_chain(p, sub)
        {
            return Some(effs);
        }
        None
    }

    /// `ChangeZone | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True`
    /// -> `Play | Defined$ Remembered | CopyCard$ True | Amount$ N | AllowRepeats$ True |
    /// WithoutManaCost$ True | Optional$ True` ->
    /// `Effect::CaptureAffected` + `ForEachRemembered` + `RepeatEach(Times(N), Play(copy))`
    /// (*Mnemonic Deluge*).
    #[allow(clippy::too_many_lines)]
    fn wave30a_exile_and_cast_n_copies_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let tgt = target.filter(|t| t.starts_with("TargetSpec::CardInGraveyard("))?;
        let filter_expr = tgt
            .strip_prefix("TargetSpec::CardInGraveyard(")?
            .rsplit_once(',')?
            .0
            .trim();
        if !Self::filter_excludes_lands(filter_expr, &self.body.statics, 0) {
            return None;
        }
        if self.overload_cost.is_some() || p.has("Hidden") || self.has_linked_exile_reader() {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Origin");
        probe.take("Destination");
        probe.take("RememberChanged");
        probe.take("IsCurse");
        if let Some(forget) = probe.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        if !matches!(probe.take("ChangeNum").as_deref(), None | Some("1")) {
            return None;
        }
        if let Some(mandatory) = probe.take("Mandatory")
            && mandatory != "True"
        {
            return None;
        }
        if !probe.exhausted() {
            return None;
        }
        let next_name = sub.as_deref()?.trim().to_string();
        let next_body = self.svars.get(&next_name)?.clone();
        let (next_api, mut sp) = Params::parse(&next_body)?;
        sp.drop_prose();
        if next_api != "Play" {
            return None;
        }
        let targets_remembered = matches!(
            (sp.take("Valid").as_deref(), sp.take("Defined").as_deref()),
            (
                Some("Card.IsRemembered" | "Card.IsRemembered+ExiledWithSource"),
                None
            ) | (None, Some("Remembered"))
        );
        if !targets_remembered
            || !matches!(sp.take("ValidSA").as_deref(), None | Some("Spell"))
            || !matches!(sp.take("ValidZone").as_deref(), None | Some("Exile"))
            || !matches!(sp.take("Controller").as_deref(), None | Some("You"))
            || sp.take("CopyCard").as_deref() != Some("True")
            || sp.take("AllowRepeats").as_deref() != Some("True")
            || sp.take("WithoutManaCost").as_deref() != Some("True")
            || sp.take("Optional").as_deref() != Some("True")
        {
            return None;
        }
        let count_raw = sp.take("Amount")?;
        let count: u32 = self
            .plain_num(&count_raw)?
            .try_into()
            .ok()
            .filter(|&n| (2..=16).contains(&n))?;
        if let Some(show) = sp.take("ShowCard")
            && show != "True"
        {
            return None;
        }
        if let Some(copy_opt) = sp.take("CopyIsOptional")
            && copy_opt != "True"
        {
            return None;
        }
        let after = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        *p = probe;
        *sub = None;
        if let Some(aname) = after {
            if self.sub_is_api(Some(&aname), "Cleanup") {
                self.consume_optional_remember_cleanup(aname, sub)?;
            } else {
                *sub = Some(aname);
            }
        }
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::exile({tgt})], \
             then: &[Effect::ForEachRemembered {{ \
             filter: &Filter::Any, \
             effects: &[Effect::RepeatEach {{ \
             domain: RepeatDomain::Times(Amount::Fixed({count})), \
             effects: &[Effect::Play {{ \
             source: PlaySource::Subject(TargetSpec::EventObject), \
             filter: &Filter::Any, controller: PlayerRel::You, count: PlayCount::One, \
             cost: PlayCostMode::WithoutPayingManaCost, cast_only: true, copy: true, \
             exile_instead_of_graveyard: false, optional: true }}] \
             }}] \
             }}] \
             }}"
        )])
    }

    /// `ChangeZone | Origin$ Library | Destination$ Graveyard | ChangeNum$ 1 |
    /// Shuffle$ True | RememberChanged$ True` -> `Clone | Defined$ Remembered`
    /// -> `Effect::CaptureAffected(SearchLibrary(Find::GRAVEYARD))` +
    /// `ForEachRemembered(Effect::BecomeCopy)` (*Scion of the Ur-Dragon*).
    fn wave30a_tutor_graveyard_clone_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if self.on_a_spell || p.has("Defined") || p.has("ValidTgts") || p.has("DefinedPlayer") {
            return None;
        }
        let next_name = sub.as_deref()?.trim().to_string();
        let next_body = self.svars.get(&next_name)?.clone();
        let (next_api, mut sp) = Params::parse(&next_body)?;
        sp.drop_prose();
        if next_api != "Clone"
            || !matches!(
                sp.take("Defined").as_deref(),
                Some("Remembered" | "RememberedLKI")
            )
            || sp.has("ValidTgts")
            || sp.has("AddKeywords")
            || sp.has("AddTypes")
            || sp.has("SetPower")
            || sp.has("SetToughness")
            || sp.has("SetColor")
            || sp.has("SetName")
            || sp.has("AddAbilities")
            || sp.has("AddTriggers")
            || sp.has("AddStaticAbilities")
            || sp.has("Remove")
            || sp.has("GainControl")
            || sp.has("Choices")
        {
            return None;
        }
        let until_eot = match sp.take("Duration").as_deref() {
            None => false,
            Some("UntilEndOfTurn" | "EndOfTurn") => true,
            Some(_) => return None,
        };
        let after = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }

        let mut probe = p.clone();
        probe.take("Origin");
        probe.take("Destination");
        probe.take("RememberChanged");
        probe.take("IsCurse");
        if probe.take("Shuffle").as_deref() != Some("True") {
            return None;
        }
        if !matches!(probe.take("ChangeNum").as_deref(), None | Some("1")) {
            return None;
        }
        if let Some(forget) = probe.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        let optional = match (
            probe.take("Optional").as_deref(),
            probe.take("Mandatory").as_deref(),
            probe.take("ShuffleNonMandatory").as_deref(),
        ) {
            (Some("True"), None | Some("False"), _)
            | (None | Some("False"), None | Some("False"), Some("True")) => true,
            (None | Some("False"), _, None | Some("False")) => false,
            _ => return None,
        };
        probe.claim_label("ChangeTypeDesc", "ChangeType");
        let valid = probe.take("ChangeType")?;
        if valid.contains("Ctrl") || valid.contains("Own") {
            return None;
        }
        if let Some(h) = probe.take("Hidden")
            && h != "True"
        {
            return None;
        }
        if !probe.exhausted() {
            return None;
        }
        let expr = self.filter_expr_in_zone(&valid, true)?;
        let filter_ref = if expr == "Filter::Any" {
            "&Filter::Any".to_string()
        } else {
            format!("&{}", self.body.filter_static("SEARCH", &expr))
        };
        *p = probe;
        *sub = None;
        if let Some(aname) = after {
            self.consume_optional_remember_cleanup(aname, sub)?;
        }
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::SearchLibrary {{ filter: {filter_ref}, finds: &[Find::GRAVEYARD], optional: {optional} }}], \
             then: &[Effect::ForEachRemembered {{ \
             filter: &Filter::Any, \
             effects: &[Effect::BecomeCopy {{ subject: TargetSlot::This, subject_filter: None, \
             original: TargetSpec::EventObject, original_is_second_target: false, \
             until_eot: {until_eot}, mods: &[] }}] \
             }}] \
             }}"
        )])
    }

    /// True when the card reads its linked exile ONLY through `ExiledWithSource` /
    /// `Remembered.ExiledWithSource` + housekeeping link-cleanup triggers (so
    /// `RememberChanged$ True` on `Duration$ UntilHostLeavesPlay` is redundant
    /// bookkeeping rather than an unread follow-up).
    pub(super) fn wave30a_can_claim_linked_exile_remembered(&self) -> bool {
        let mut saw_link_housekeeping = false;
        let check_no_other_remembered = |body: &str| {
            let has_remembered_var = body.match_indices("Remembered$").any(|(i, _)| {
                !body[..i]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_ascii_alphanumeric())
            });
            !body.contains("Defined$ Remembered")
                && !has_remembered_var
                && !body.contains("RememberedLKI")
                && !body.contains("RememberedController")
                && !body.contains("RememberedOwner")
                && (!body.contains("IsRemembered") || body.contains("ExiledWithSource"))
        };
        if !self
            .body
            .abilities
            .iter()
            .all(|b| check_no_other_remembered(b))
        {
            return false;
        }
        for body in self.svars.values() {
            if body.contains("Remembered.ExiledWithSource")
                || body.contains("Card.IsRemembered+ExiledWithSource")
                || (body.contains("Bookkeeping") && body.contains("ForgetObjects$ TriggeredCard"))
            {
                saw_link_housekeeping = true;
            }
            if !check_no_other_remembered(body) {
                return false;
            }
        }
        saw_link_housekeeping && !self.card_has_unlinked_exile_producer()
    }
}
