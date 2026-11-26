//! Wave 29a pure-`scriptgen` lowerings across `Effect`, `ChangeZone`,
//! `ChangeZoneAll`, `RememberChanged`, and `TargetMin`.
//!
//! Every rule here targets an existing, engine-verified `baylee-cards-dsl`
//! construct with strict honest refusal boundaries: no silent parameter drops,
//! no `Origin$ Sideboard` conflation with `WishToHand`, no dynamic `0..=X`
//! target count approximations, and no opponent-emblem misattribution.

use super::effects::EffectSubject;
use super::{Chain, Params, Tx, counter_kind};

impl Tx<'_> {
    // =========================================================================
    // 1. Filter atoms (`dealtDamageThisTurn` / `!dealtDamageThisTurn`)
    // =========================================================================

    pub(super) fn wave29a_filter_atom(&self, atom: &str, card_zone: bool) -> Option<String> {
        let _ = self;
        if card_zone {
            return None;
        }
        match atom {
            "dealtDamageThisTurn" => Some("Filter::DealtDamageThisTurn".to_string()),
            "!dealtDamageThisTurn" => Some("Filter::Not(&Filter::DealtDamageThisTurn)".to_string()),
            _ => None,
        }
    }

    // =========================================================================
    // 2. `Effect | Triggers$` -> `Effect::WhenNextSpellCastThisTurn`
    //    (*Summon: Fenrir* Ch II, *Yuna, Grand Summoner*)
    // =========================================================================

    pub(super) fn wave29a_when_next_spell_cast_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some()
            || p.has("StaticAbilities")
            || p.has("ReplacementEffects")
            || p.has("Abilities")
            || p.has("RememberObjects")
            || p.has("Boon")
            || p.has("EffectOwner")
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
        if let Some(s) = p.take("Stackable")
            && !matches!(s.as_str(), "True" | "False")
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
        if mode != "SpellCast"
            || tp.take("OneOff").as_deref() != Some("True")
            || tp.take("ValidActivatingPlayer").as_deref() != Some("You")
            || !matches!(tp.take("TriggerZones").as_deref(), None | Some("Command"))
            || !matches!(tp.take("ValidSA").as_deref(), None | Some("Spell"))
        {
            return None;
        }
        let valid_card = tp.take("ValidCard").unwrap_or_else(|| "Card".to_string());
        let exec_name = tp.take("Execute")?;
        if !tp.exhausted() {
            return None;
        }
        let n = self.wave29a_parse_etb_p1p1_effect_svar(exec_name.trim())?;
        let cleaned =
            Self::strip_hand_owner_ctrl(&valid_card, "YouOwn", "YouCtrl").unwrap_or(valid_card);
        let filter_ref = match cleaned.as_str() {
            "Card" => "&Filter::Any".to_string(),
            "Creature" => "&Filter::CREATURE".to_string(),
            other => {
                let expr = self.filter_expr_in_zone(other, true)?;
                format!("&{}", self.body.filter_static("NEXT_SPELL", &expr))
            }
        };
        Some(vec![format!(
            "Effect::WhenNextSpellCastThisTurn {{ filter: {filter_ref}, \
             effects: &[Effect::EventSpellEntersWithP1P1 {{ n: {n} }}] }}"
        )])
    }

    fn wave29a_parse_etb_p1p1_effect_svar(&self, svar_name: &str) -> Option<u16> {
        let body = self.svars.get(svar_name)?.clone();
        let (api, mut ep) = Params::parse(&body)?;
        ep.drop_prose();
        if api != "Effect" {
            return None;
        }
        if !matches!(
            ep.take("Duration").as_deref(),
            None | Some("UntilEndOfTurn" | "EndOfTurn" | "Permanent")
        ) {
            return None;
        }
        if !matches!(
            ep.take("RememberObjects").as_deref(),
            Some("TriggeredSpellAbility" | "TriggeredCard" | "TriggeredCardLKICopy")
        ) {
            return None;
        }
        ep.take("Name");
        let _ = ep.take("ExileOnMoved").or_else(|| ep.take("ForgetOnMoved"));
        let rep_name = ep.take("ReplacementEffects")?;
        if rep_name.contains(',') {
            return None;
        }
        if let Some(after) = ep.take("SubAbility")
            && !self.wave29a_is_command_self_exile(after.trim())
        {
            return None;
        }
        if !ep.exhausted() {
            return None;
        }
        let rep_body = self.svars.get(rep_name.trim())?;
        let (event, mut rp) = Params::parse(rep_body)?;
        rp.drop_prose();
        if event != "Moved"
            || rp.take("Destination").as_deref() != Some("Battlefield")
            || !matches!(
                rp.take("ValidCard").as_deref(),
                Some("Card.IsRemembered" | "Creature.IsRemembered" | "Permanent.IsRemembered")
            )
            || !matches!(rp.take("ActiveZones").as_deref(), None | Some("Command"))
            || !matches!(
                rp.take("ReplacementResult").as_deref(),
                None | Some("Updated")
            )
        {
            return None;
        }
        let rw = rp.take("ReplaceWith")?;
        if !rp.exhausted() {
            return None;
        }
        let rw_body = self.svars.get(rw.trim())?;
        let (rw_api, mut cp) = Params::parse(rw_body)?;
        cp.drop_prose();
        if rw_api != "PutCounter"
            || cp.take("ETB").as_deref() != Some("True")
            || !matches!(
                cp.take("Defined").as_deref(),
                Some("ReplacedCard" | "Remembered")
            )
            || counter_kind(cp.take("CounterType")?.trim()).as_deref() != Some("CounterKind::P1P1")
        {
            return None;
        }
        let raw_num = cp.take("CounterNum").unwrap_or_else(|| "1".to_string());
        if let Some(after_put) = cp.take("SubAbility")
            && !self.wave29a_is_command_self_exile(after_put.trim())
        {
            return None;
        }
        if !cp.exhausted() {
            return None;
        }
        let n: u16 = self.plain_num(&raw_num)?.try_into().ok()?;
        (n >= 1).then_some(n)
    }

    fn wave29a_is_command_self_exile(&self, svar_name: &str) -> bool {
        let Some(body) = self.svars.get(svar_name) else {
            return false;
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return false;
        };
        p.drop_prose();
        api == "ChangeZone"
            && matches!(p.take("Defined").as_deref(), None | Some("Self"))
            && p.take("Origin").as_deref() == Some("Command")
            && p.take("Destination").as_deref() == Some("Exile")
            && p.exhausted()
    }

    // =========================================================================
    // 3. `Effect` API extensions (`Trial of a Time Lord`, `Martha Jones`,
    //    `Tatsunari, Toad Rider`, `Inspired Idea`, `Hex Magic`, `Ignite the
    //    Future`, `Daredevil, Man Without Fear`, `Gale's Redirection`, ...)
    // =========================================================================

    pub(super) fn wave29a_effect_api(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        // 3a. Bare no-op `DB$ Effect` (e.g. `DBInnocent: DB$ Effect` vote option
        // on *Trial of a Time Lord*).
        if !self.on_a_spell_root_only(p)
            && target.is_none()
            && !p.has("StaticAbilities")
            && !p.has("ReplacementEffects")
            && !p.has("Triggers")
            && !p.has("Abilities")
            && !p.has("Keywords")
            && !p.has("AllSuffixKeywords")
            && !p.has("RememberObjects")
            && !p.has("Defined")
            && !p.has("Duration")
            && !p.has("Boon")
        {
            let mut probe = p.clone();
            probe.take("Name");
            if probe
                .take("Stackable")
                .is_none_or(|s| matches!(s.as_str(), "True" | "False"))
                && Self::wave29a_only_condition_keys_left(&probe)
            {
                *p = probe;
                return Some(Vec::new());
            }
        }

        // 3b. Dual `Self & Targeted` / `Targeted & Self` unblockable (`Martha Jones`).
        if !self.on_a_spell
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && matches!(
                p.peek("RememberObjects"),
                Some("Self & Targeted" | "Targeted & Self")
            )
            && let Some(fx) = self.wave29a_dual_self_and_target_cant_block_by(p)
        {
            return Some(fx);
        }

        // 3c. Paired `EffectSource` + `IsRemembered` `CantBlockBy` (`Tatsunari, Toad Rider`).
        if !self.on_a_spell
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && p.peek("RememberObjects") == Some("Targeted")
            && p.peek("StaticAbilities")
                .is_some_and(|s| s.split(',').count() == 2)
            && let Some(fx) = self.wave29a_paired_source_and_target_evasion(p)
        {
            return Some(fx);
        }

        // 3d. `Continuous | Affected$ You | SetMaxHandSize$ Unlimited` (`Inspired Idea`).
        if target.is_none()
            && !p.has("ReplacementEffects")
            && !p.has("Triggers")
            && !p.has("Abilities")
            && !p.has("RememberObjects")
            && let Some(fx) = self.wave29a_no_max_hand_size_effect(p)
        {
            return Some(fx);
        }

        // 3e. Inside `CaptureAffected::then`: grant exile play/cast permission
        // on remembered exiled cards (`Hex Magic`, `Ignite the Future`,
        // `Daredevil, Man Without Fear`, `Gale's Redirection`, `Thada Adel`, ...).
        if self.capture_scope.is_some()
            && let Some(fx) = self.wave29a_capture_exile_play_effect(p)
        {
            return Some(fx);
        }

        None
    }

    pub(super) fn wave29a_is_capture_exile_play_effect(&self, p: &Params) -> bool {
        self.capture_scope
            .as_ref()
            .is_some_and(|s| !s.from_battlefield)
            && !self.has_linked_exile_reader()
            && matches!(
                p.peek("RememberObjects"),
                Some("Remembered" | "RememberedCard")
            )
            && p.has("StaticAbilities")
            && !p.has("Triggers")
            && !p.has("ReplacementEffects")
            && !p.has("Abilities")
    }

    fn wave29a_capture_exile_play_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if self
            .capture_scope
            .as_ref()
            .is_none_or(|s| s.from_battlefield)
            || self.has_linked_exile_reader()
        {
            return None;
        }
        let mut probe = p.clone();
        if !matches!(
            probe.take("RememberObjects").as_deref(),
            Some("Remembered" | "RememberedCard")
        ) || !matches!(probe.take("EffectOwner").as_deref(), None | Some("You"))
            || !matches!(
                probe
                    .take("ExileOnMoved")
                    .or_else(|| probe.take("ForgetOnMoved"))
                    .as_deref(),
                None | Some("Exile")
            )
        {
            return None;
        }
        probe.take("Name");
        probe.take("IsCurse");
        if let Some(st) = probe.take("Stackable")
            && !matches!(st.as_str(), "True" | "False")
        {
            return None;
        }
        let raw_duration = probe.take("Duration");
        let static_name = probe.take("StaticAbilities")?;
        if static_name.contains(',') || !Self::wave29a_only_condition_keys_left(&probe) {
            return None;
        }
        let static_body = self.svars.get(static_name.trim())?.clone();
        if !static_body.contains("IsRemembered") {
            return None;
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let Some(spec) = self.parse_exile_play_static(
            static_name.trim(),
            super::effects::ExileAffectedMarker::Remembered,
            true,
            raw_duration.as_deref(),
        ) else {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return None;
        };
        *p = probe;
        Some(vec![format!(
            "Effect::ForEachRemembered {{ filter: &Filter::Any, \
             effects: &[Effect::ExileWithPermission {{ target: TargetSpec::EventObject, \
             until: None, permission: {} }}] }}",
            spec.to_expr()
        )])
    }

    fn on_a_spell_root_only(&self, p: &Params) -> bool {
        self.on_a_spell && p.entries.is_empty() && self.svars.is_empty()
    }

    fn wave29a_effect_duration_str(raw: Option<&str>, on_a_spell: bool) -> Option<&'static str> {
        match raw {
            None | Some("UntilEndOfTurn") => Some("Duration::UntilEndOfTurn"),
            Some("UntilEndOfCombat") => Some("Duration::UntilEndOfCombat"),
            Some("UntilYourNextTurn") => Some("Duration::UntilYourNextTurn"),
            Some("UntilYourNextUpkeep") => Some("Duration::UntilYourNextUpkeep"),
            Some("Permanent") => Some("Duration::Indefinitely"),
            Some("LeavesPlay" | "UntilHostLeavesPlay") if !on_a_spell => {
                Some("Duration::WhileSourceOnBattlefield")
            }
            Some("AsLongAsControl") if !on_a_spell => Some("Duration::WhileYouControlSource"),
            _ => None,
        }
    }

    fn wave29a_dual_self_and_target_cant_block_by(
        &mut self,
        p: &mut Params,
    ) -> Option<Vec<String>> {
        let mut probe = p.clone();
        let rem = probe.take("RememberObjects")?;
        if !matches!(rem.as_str(), "Self & Targeted" | "Targeted & Self") {
            return None;
        }
        let duration =
            Self::wave29a_effect_duration_str(probe.take("Duration").as_deref(), self.on_a_spell)?;
        let ends = probe
            .take("ExileOnMoved")
            .or_else(|| probe.take("ForgetOnMoved"));
        if !matches!(ends.as_deref(), None | Some("Battlefield")) {
            return None;
        }
        probe.take("IsCurse");
        probe.take("Name");
        let statics = probe.take("StaticAbilities")?;
        if statics.contains(',') || !probe.exhausted() {
            return None;
        }
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CantBlockBy"
            || !matches!(st.take("EffectZone").as_deref(), None | Some("Command"))
            || !matches!(
                st.take("ValidAttacker").as_deref(),
                Some("Card.IsRemembered" | "Creature.IsRemembered")
            )
        {
            return None;
        }
        let valid_blocker = st.take("ValidBlocker");
        if !st.exhausted() {
            return None;
        }
        let modifier = match valid_blocker.as_deref() {
            None | Some("Creature" | "Card") => {
                "Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)".to_string()
            }
            Some(vb) => {
                let expr = self.filter_expr(vb)?;
                let f_name = self.body.filter_static("BLOCKER", &expr);
                format!("Modifier::CantBeBlockedBy(&{f_name})")
            }
        };
        *p = probe;
        Some(vec![
            format!("Effect::ModifySource {{ modifier: {modifier}, duration: {duration} }}"),
            format!("Effect::ModifyTargets {{ modifier: {modifier}, duration: {duration} }}"),
        ])
    }

    fn wave29a_paired_source_and_target_evasion(&mut self, p: &mut Params) -> Option<Vec<String>> {
        let mut probe = p.clone();
        if probe.take("RememberObjects").as_deref() != Some("Targeted") {
            return None;
        }
        let duration =
            Self::wave29a_effect_duration_str(probe.take("Duration").as_deref(), self.on_a_spell)?;
        let ends = probe
            .take("ExileOnMoved")
            .or_else(|| probe.take("ForgetOnMoved"));
        if !matches!(ends.as_deref(), None | Some("Battlefield")) {
            return None;
        }
        probe.take("IsCurse");
        probe.take("Name");
        let statics = probe.take("StaticAbilities")?;
        let parts: Vec<&str> = statics.split(',').map(str::trim).collect();
        if parts.len() != 2 || !probe.exhausted() {
            return None;
        }
        let mut saw_source = false;
        let mut saw_remembered = false;
        let mut blocker_raw: Option<Option<String>> = None;
        for name in parts {
            let body = self.svars.get(name)?.clone();
            let (mode, mut st) = Params::parse(&body)?;
            st.drop_prose();
            if mode != "CantBlockBy"
                || !matches!(st.take("EffectZone").as_deref(), None | Some("Command"))
            {
                return None;
            }
            match st.take("ValidAttacker").as_deref() {
                Some(
                    "Card.EffectSource" | "Creature.EffectSource" | "Card.Self" | "Creature.Self",
                ) => {
                    saw_source = true;
                }
                Some("Card.IsRemembered" | "Creature.IsRemembered") => {
                    saw_remembered = true;
                }
                _ => return None,
            }
            let vb = st.take("ValidBlocker");
            if !st.exhausted() {
                return None;
            }
            if let Some(prev) = &blocker_raw {
                if prev != &vb {
                    return None;
                }
            } else {
                blocker_raw = Some(vb);
            }
        }
        if !saw_source || !saw_remembered {
            return None;
        }
        let modifier = match blocker_raw.flatten().as_deref() {
            None | Some("Creature" | "Card") => {
                "Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)".to_string()
            }
            Some(vb) => {
                let expr = self.filter_expr(vb)?;
                let f_name = self.body.filter_static("BLOCKER", &expr);
                format!("Modifier::CantBeBlockedBy(&{f_name})")
            }
        };
        *p = probe;
        Some(vec![
            format!("Effect::ModifySource {{ modifier: {modifier}, duration: {duration} }}"),
            format!("Effect::ModifyTargets {{ modifier: {modifier}, duration: {duration} }}"),
        ])
    }

    fn wave29a_no_max_hand_size_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        let mut probe = p.clone();
        let duration =
            Self::wave29a_effect_duration_str(probe.take("Duration").as_deref(), self.on_a_spell)?;
        probe.take("Name");
        probe.take("IsCurse");
        if let Some(ns) = probe.take("NonStackingEffect")
            && ns != "True"
        {
            return None;
        }
        if let Some(st) = probe.take("Stackable")
            && !matches!(st.as_str(), "True" | "False")
        {
            return None;
        }
        let statics = probe.take("StaticAbilities")?;
        if statics.contains(',') || !probe.exhausted() {
            return None;
        }
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "Continuous"
            || !matches!(st.take("EffectZone").as_deref(), None | Some("Command"))
            || st.take("Affected").as_deref() != Some("You")
            || st.take("SetMaxHandSize").as_deref() != Some("Unlimited")
            || !st.exhausted()
        {
            return None;
        }
        *p = probe;
        Some(vec![format!(
            "Effect::continuous(&Filter::Any, Modifier::NoMaxHandSize, {duration})"
        )])
    }

    /// Also allow `Continuous | Affected$ You | SetMaxHandSize$ Unlimited` as one
    /// static inside a multi-static `Effect` (`wave3_static_effect_line_with_target`).
    pub(super) fn wave29a_static_effect_line(
        mode: &str,
        st: &mut Params,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        if mode == "Continuous"
            && subject == EffectSubject::Unbound
            && st.peek("Affected") == Some("You")
            && st.peek("SetMaxHandSize") == Some("Unlimited")
        {
            let mut probe = st.clone();
            probe.take("Affected");
            probe.take("SetMaxHandSize");
            if probe.exhausted() {
                *st = probe;
                return Some(vec![format!(
                    "Effect::continuous(&Filter::Any, Modifier::NoMaxHandSize, {duration})"
                )]);
            }
        }
        None
    }

    // =========================================================================
    // 4. `Necropotence`, `Imprint` exile-with-permission (`Savvy Trader`),
    //    `Surprise Deployment`, `Lat-Nam's Legacy`, and `Wormfang Drake`
    // =========================================================================

    pub(super) fn wave29a_necropotence_exile_top_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let mut probe = p.clone();
        if probe.take("Defined").as_deref() != Some("TopOfLibrary")
            || probe.take("Origin").as_deref() != Some("Library")
            || probe.take("Destination").as_deref() != Some("Exile")
            || probe.take("ExileFaceDown").as_deref() != Some("True")
            || probe.take("RememberChanged").as_deref() != Some("True")
            || !matches!(probe.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(probe.take("ChangeNum").as_deref(), None | Some("1"))
        {
            return None;
        }
        probe.take("AILifeThreshold");
        if !probe.exhausted() {
            return None;
        }
        let del_name = sub.as_deref()?.trim();
        let del_body = self.svars.get(del_name)?.clone();
        let (del_api, mut dp) = Params::parse(&del_body)?;
        dp.drop_prose();
        if del_api != "DelayedTrigger"
            || dp.take("Mode").as_deref() != Some("Phase")
            || !matches!(
                dp.take("Phase").as_deref(),
                Some("End of Turn" | "End Of Turn")
            )
            || dp.take("ValidPlayer").as_deref() != Some("You")
            || !matches!(
                dp.take("RememberObjects").as_deref(),
                Some("Remembered" | "RememberedLKI")
            )
        {
            return None;
        }
        let exec_name = dp.take("Execute")?;
        let after_del = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        let ret_body = self.svars.get(exec_name.trim())?;
        let (ret_api, mut rp) = Params::parse(ret_body)?;
        rp.drop_prose();
        if ret_api != "ChangeZone"
            || rp.take("Origin").as_deref() != Some("Exile")
            || rp.take("Destination").as_deref() != Some("Hand")
            || !matches!(
                rp.take("Defined").as_deref(),
                Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
            )
            || !rp.exhausted()
        {
            return None;
        }
        let mut trailing: Option<String> = None;
        if let Some(clean_name) = after_del {
            let clean_body = self.svars.get(clean_name.trim())?;
            let (c_api, mut cp) = Params::parse(clean_body)?;
            cp.drop_prose();
            if c_api != "Cleanup" || cp.take("ClearRemembered").as_deref() != Some("True") {
                return None;
            }
            trailing = cp.take("SubAbility");
            if !cp.exhausted() {
                return None;
            }
        }
        *p = probe;
        *sub = trailing;
        Some(vec![
            "Effect::ExileTopFaceDownUntilYourNextEndStep".to_string(),
        ])
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29a_imprint_exile_with_permission(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if self.has_linked_exile_reader() {
            return None;
        }
        let mut probe = p.clone();
        let origin = probe.take("Origin")?;
        if !matches!(origin.as_str(), "Battlefield" | "Graveyard")
            || probe.take("Destination").as_deref() != Some("Exile")
            || probe.take("Imprint").as_deref() != Some("True")
            || probe.has("RememberChanged")
        {
            return None;
        }
        probe.take("IsCurse");
        let defined = probe.take("Defined");
        let target_spec = match (origin.as_str(), defined.as_deref()) {
            ("Battlefield", None) if targets_here => target
                .filter(|t| t.starts_with("TargetSpec::Object("))?
                .to_string(),
            ("Graveyard", None) if targets_here => target
                .filter(|t| t.starts_with("TargetSpec::CardInGraveyard("))?
                .to_string(),
            _ => return None,
        };
        if !probe.exhausted() {
            return None;
        }
        let eff_name = sub.as_deref()?.trim().to_string();
        for (k, v) in self.svars {
            if k == &eff_name {
                continue;
            }
            if v.contains("Imprinted") || v.contains("IsImprinted") {
                let is_clear = Params::parse(v).is_some_and(|(api, mut cp)| {
                    cp.drop_prose();
                    api == "Cleanup" && cp.take("ClearImprinted").as_deref() == Some("True")
                });
                if !is_clear {
                    return None;
                }
            }
        }
        if self
            .body
            .abilities
            .iter()
            .any(|a| a.contains("Imprinted") || a.contains("LinkedExile"))
        {
            return None;
        }
        let eff_body = self.svars.get(&eff_name)?.clone();
        let (eff_api, mut ep) = Params::parse(&eff_body)?;
        ep.drop_prose();
        if eff_api != "Effect"
            || ep.take("RememberObjects").as_deref() != Some("Imprinted")
            || !matches!(ep.take("EffectOwner").as_deref(), None | Some("You"))
            || !matches!(
                ep.take("ExileOnMoved")
                    .or_else(|| ep.take("ForgetOnMoved"))
                    .as_deref(),
                None | Some("Exile")
            )
        {
            return None;
        }
        let raw_duration = ep.take("Duration");
        let static_name = ep.take("StaticAbilities")?;
        if static_name.contains(',') {
            return None;
        }
        ep.take("Name");
        let next_sub = ep.take("SubAbility")?;
        if !ep.exhausted() {
            return None;
        }
        let clean_body = self.svars.get(next_sub.trim())?.clone();
        let (c_api, mut cp) = Params::parse(&clean_body)?;
        cp.drop_prose();
        if c_api != "Cleanup" || cp.take("ClearImprinted").as_deref() != Some("True") {
            return None;
        }
        let after_clean = cp.take("SubAbility");
        if !cp.exhausted() {
            return None;
        }
        let static_body = self.svars.get(static_name.trim())?.clone();
        if !static_body.contains("Card.IsRemembered") {
            return None;
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let Some(spec) = self.parse_exile_play_static(
            static_name.trim(),
            super::effects::ExileAffectedMarker::Remembered,
            true,
            raw_duration.as_deref(),
        ) else {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return None;
        };
        *p = probe;
        *sub = after_clean;
        Some(vec![format!(
            "Effect::ExileWithPermission {{ target: {target_spec}, until: None, \
             permission: {} }}",
            spec.to_expr()
        )])
    }

    pub(super) fn wave29a_is_capture_delayed_trigger(&self, p: &Params) -> bool {
        let _ = self;
        p.peek("Mode") == Some("Phase")
            && !p.has("NextTurn")
            && !p.has("OptionalDecider")
            && !p.has("ThisTurn")
            && matches!(
                p.peek("Phase"),
                Some("End of Turn" | "End Of Turn" | "EndCombat")
            )
            && matches!(
                p.peek("RememberObjects"),
                Some("Remembered" | "RememberedLKI")
            )
    }

    pub(super) fn wave29a_capture_delayed_trigger(
        &mut self,
        p: &mut Params,
    ) -> Option<Vec<String>> {
        self.capture_scope.as_ref()?;
        let mut probe = p.clone();
        if probe.take("Mode").as_deref() != Some("Phase")
            || probe.has("NextTurn")
            || probe.has("OptionalDecider")
            || probe.has("ThisTurn")
        {
            return None;
        }
        let step = match probe.take("Phase").as_deref() {
            Some("End of Turn" | "End Of Turn") => "DelayedStep::EndStep",
            Some("EndCombat") => "DelayedStep::CombatEnd",
            _ => return None,
        };
        let whose = match probe.take("ValidPlayer").as_deref() {
            None | Some("Player") => "None",
            Some("You") => "Some(PlayerRel::You)",
            _ => return None,
        };
        if !matches!(
            probe.take("RememberObjects").as_deref(),
            Some("Remembered" | "RememberedLKI")
        ) {
            return None;
        }
        let exec_name = probe.take("Execute")?;
        if !Self::wave29a_only_condition_keys_left(&probe) {
            return None;
        }
        let exec_body = self.svars.get(exec_name.trim())?.clone();
        let (exec_api, mut ep) = Params::parse(&exec_body)?;
        ep.drop_prose();
        let delayed_fx = match exec_api.as_str() {
            "ChangeZone" => {
                if ep.take("Origin").as_deref() != Some("Battlefield")
                    || !matches!(
                        ep.take("Defined").as_deref(),
                        Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
                    )
                {
                    return None;
                }
                let fx = match ep.take("Destination").as_deref() {
                    Some("Hand") => "Effect::ReturnToHand { target: TargetSpec::EventObject }",
                    Some("Exile") if !self.has_linked_exile_reader() => {
                        "Effect::exile(TargetSpec::EventObject)"
                    }
                    _ => return None,
                };
                if !ep.exhausted() {
                    return None;
                }
                fx
            }
            "Sacrifice" => {
                if !matches!(
                    ep.take("Defined")
                        .or_else(|| ep.take("SacValid"))
                        .as_deref(),
                    Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
                ) || !matches!(ep.take("Controller").as_deref(), None | Some("You"))
                    || !ep.exhausted()
                {
                    return None;
                }
                "Effect::SacrificeEvent"
            }
            _ => return None,
        };
        *p = probe;
        Some(vec![format!(
            "Effect::ForEachRemembered {{ filter: &Filter::Any, \
             effects: &[Effect::RegisterDelayedTrigger {{ \
             when: DelayedTriggerWhen::AtStep {{ step: {step}, whose: {whose}, \
             this_turn_only: false, recurring: false }}, condition: None, targets: None, \
             remember: TriggerRemember::EventObject, effects: &[{delayed_fx}] }}] }}"
        )])
    }

    /// `Lat-Nam's Legacy`: `ChangeZone | Origin$ Hand | Destination$ Library |
    /// Shuffle$ True | ChangeType$ Card | RememberChanged$ True | SubAbility$ DelTrigSlowtrip`
    /// where `DelTrigSlowtrip` is `DelayedTrigger | NextTurn$ True | Mode$ Phase |
    /// Phase$ Upkeep | ValidPlayer$ Player | Execute$ <Draw>`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29a_hand_shuffle_slowtrip_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if target.is_some() || targets_here {
            return None;
        }
        let mut probe = p.clone();
        if probe.take("Origin").as_deref() != Some("Hand")
            || probe.take("Destination").as_deref() != Some("Library")
            || probe.take("Shuffle").as_deref() != Some("True")
            || probe.take("RememberChanged").as_deref() != Some("True")
            || !matches!(probe.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(probe.take("Chooser").as_deref(), None | Some("You"))
            || !matches!(probe.take("ChangeNum").as_deref(), None | Some("1"))
        {
            return None;
        }
        if let Some(h) = probe.take("Hidden")
            && h != "True"
        {
            return None;
        }
        if let Some(m) = probe.take("Mandatory")
            && m != "True"
        {
            return None;
        }
        probe.take("SelectPrompt");
        let reveal = match probe.take("Reveal").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let raw_valid = probe.take("ChangeType")?;
        let cleaned =
            Self::strip_hand_owner_ctrl(&raw_valid, "YouOwn", "YouCtrl").unwrap_or(raw_valid);
        if !reveal && cleaned != "Card" {
            return None;
        }
        if !probe.exhausted() {
            return None;
        }
        let del_name = sub.as_deref()?.trim();
        let del_body = self.svars.get(del_name)?.clone();
        let (del_api, mut dp) = Params::parse(&del_body)?;
        dp.drop_prose();
        if del_api != "DelayedTrigger"
            || dp.take("NextTurn").as_deref() != Some("True")
            || dp.take("Mode").as_deref() != Some("Phase")
            || dp.take("Phase").as_deref() != Some("Upkeep")
            || !matches!(dp.take("ValidPlayer").as_deref(), None | Some("Player"))
        {
            return None;
        }
        let exec_name = dp.take("Execute")?;
        let after_del = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        let mut trailing: Option<String> = None;
        if let Some(clean_name) = after_del {
            let clean_body = self.svars.get(clean_name.trim())?;
            let (c_api, mut cp) = Params::parse(clean_body)?;
            cp.drop_prose();
            if c_api != "Cleanup" || cp.take("ClearRemembered").as_deref() != Some("True") {
                return None;
            }
            trailing = cp.take("SubAbility");
            if !cp.exhausted() {
                return None;
            }
        }
        let exec_body = self.svars.get(exec_name.trim())?.clone();
        if exec_body.contains("Remembered") || exec_body.contains("DelayTrigger") {
            return None;
        }
        let saved_in_delayed = std::mem::replace(&mut self.in_delayed, true);
        let mut exec_chain = Chain::default();
        let ok = self.chain(&exec_body, &mut exec_chain);
        self.in_delayed = saved_in_delayed;
        ok?;
        if exec_chain.target.is_some()
            || exec_chain.second_target.is_some()
            || exec_chain.effects.is_empty()
        {
            return None;
        }
        let filter_ref = if cleaned == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&cleaned, true)?;
            format!("&{}", self.body.filter_static("HAND_SHUFFLE", &expr))
        };
        *p = probe;
        *sub = trailing;
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::ChooseCardEx {{ chooser: PlayerRel::You, \
             zone: ChooseZone::Hand(PlayerRel::You), filter: {filter_ref}, \
             min: 1, max: Amount::Fixed(1), reveal_chosen: {reveal}, \
             movement: ChooseMove::TopOfLibrary, not_chosen_movement: ChooseMove::None, \
             then: &[], for_unchosen: &[] }}], \
             then: &[Effect::IfCondition {{ \
             condition: Condition::AmountCompare {{ left: &Amount::RememberedCount, \
             op: AmountCmp::AtLeast, right: &Amount::Fixed(1) }}, \
             then: &[Effect::ShuffleLibrary {{ who: PlayerRel::You }}, \
             Effect::AtNextTurnsUpkeep {{ effects: &[{}] }}], otherwise: &[] }}] }}",
            exec_chain.effects.join(", ")
        )])
    }

    /// Whole-card check for `Wormfang Drake` (paired ETB optional exile-other
    /// you control or sacrifice self + LTB return exiled card to battlefield
    /// under owner's control, identical to `Effect::Champion` + `Effect::ReturnChampioned`).
    #[allow(clippy::too_many_lines)]
    fn wave29a_card_champion_pair(&self) -> Option<(String, String, String)> {
        if self.on_a_spell {
            return None;
        }
        let lower = self.oracle.to_ascii_lowercase();
        if !lower.contains("sacrifice it unless you exile ")
            || !lower.contains("leaves the battlefield, return the exiled card to the battlefield")
        {
            return None;
        }
        let mut etb_pair: Option<(String, String, String)> = None;
        let mut ltb_pair: Option<(String, Option<String>)> = None;
        for (name, body) in self.svars {
            let Some((api, mut p)) = Params::parse(body) else {
                continue;
            };
            p.drop_prose();
            if api == "ChangeZone"
                && p.peek("Origin") == Some("Battlefield")
                && p.peek("Destination") == Some("Exile")
                && p.peek("RememberChanged") == Some("True")
                && p.peek("Hidden") == Some("True")
                && p.peek("Optional") == Some("True")
            {
                if etb_pair.is_some() {
                    return None;
                }
                p.take("Origin");
                p.take("Destination");
                p.take("RememberChanged");
                p.take("Hidden");
                p.take("Optional");
                p.take("SelectPrompt");
                if !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
                    || !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
                {
                    return None;
                }
                let valid = p.take("ChangeType")?;
                let sac_name = p.take("SubAbility")?;
                if !p.exhausted() {
                    return None;
                }
                let stripped = Self::wave29a_strip_other_you_ctrl(&valid)?;
                let sac_body = self.svars.get(sac_name.trim())?;
                let (sac_api, mut sp) = Params::parse(sac_body)?;
                sp.drop_prose();
                if sac_api != "Sacrifice"
                    || !matches!(
                        sp.take("Defined")
                            .or_else(|| sp.take("SacValid"))
                            .as_deref(),
                        None | Some("Self" | "Card.Self")
                    )
                    || sp.take("ConditionDefined").as_deref() != Some("Remembered")
                    || !matches!(
                        sp.take("ConditionPresent").as_deref(),
                        Some("Card" | "Permanent" | "Creature")
                    )
                    || !matches!(
                        sp.take("ConditionCompare")
                            .or_else(|| sp.take("PresentCompare"))
                            .as_deref(),
                        Some("EQ0" | "LT1")
                    )
                    || !sp.exhausted()
                {
                    return None;
                }
                etb_pair = Some((name.clone(), sac_name.trim().to_string(), stripped));
            } else if api == "ChangeZone"
                && p.peek("Defined") == Some("Remembered")
                && p.peek("Origin") == Some("Exile")
                && p.peek("Destination") == Some("Battlefield")
            {
                if ltb_pair.is_some() {
                    return None;
                }
                p.take("Defined");
                p.take("Origin");
                p.take("Destination");
                let clean = p.take("SubAbility");
                if !p.exhausted() {
                    return None;
                }
                if let Some(ref c_name) = clean {
                    let c_body = self.svars.get(c_name.trim())?;
                    let (c_api, mut cp) = Params::parse(c_body)?;
                    cp.drop_prose();
                    if c_api != "Cleanup"
                        || cp.take("ClearRemembered").as_deref() != Some("True")
                        || !cp.exhausted()
                    {
                        return None;
                    }
                }
                ltb_pair = Some((name.clone(), clean.map(|s| s.trim().to_string())));
            }
        }
        let (etb_name, sac_name, stripped_valid) = etb_pair?;
        let (ltb_name, clean_name) = ltb_pair?;
        for (name, body) in self.svars {
            if name == &etb_name
                || name == &sac_name
                || name == &ltb_name
                || clean_name.as_ref() == Some(name)
            {
                continue;
            }
            if body.contains("Remembered")
                || body.contains("IsRemembered")
                || body.contains("ExiledWithSource")
            {
                return None;
            }
        }
        Some((etb_name, ltb_name, stripped_valid))
    }

    fn wave29a_strip_other_you_ctrl(valid: &str) -> Option<String> {
        let mut out = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            let (base, rest) = alt.split_once('.')?;
            let mut saw_other = false;
            let mut saw_you_ctrl = false;
            let mut kept = Vec::new();
            for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
                match atom {
                    "Other" | "StrictlyOther" => saw_other = true,
                    "YouCtrl" => saw_you_ctrl = true,
                    _ => kept.push(atom),
                }
            }
            if !saw_other || !saw_you_ctrl {
                return None;
            }
            if kept.is_empty() {
                out.push(base.to_string());
            } else {
                out.push(format!("{base}.{}", kept.join("+")));
            }
        }
        (!out.is_empty()).then(|| out.join(","))
    }

    pub(super) fn wave29a_champion_etb_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if !self.is_etb_self_trigger {
            return None;
        }
        let (_, _, stripped) = self.wave29a_card_champion_pair()?;
        let mut probe = p.clone();
        if probe.take("Origin").as_deref() != Some("Battlefield")
            || probe.take("Destination").as_deref() != Some("Exile")
            || probe.take("RememberChanged").as_deref() != Some("True")
            || probe.take("Hidden").as_deref() != Some("True")
            || probe.take("Optional").as_deref() != Some("True")
        {
            return None;
        }
        probe.take("SelectPrompt");
        probe.take("ChangeNum");
        probe.take("DefinedPlayer");
        probe.take("ChangeType")?;
        if !probe.exhausted() || sub.is_none() {
            return None;
        }
        let expr = self.filter_expr(&stripped)?;
        let filter_ref = match expr.as_str() {
            "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
            _ => format!("&{}", self.body.filter_static("CHAMPION", &expr)),
        };
        *p = probe;
        *sub = None;
        Some(vec![format!("Effect::Champion {{ filter: {filter_ref} }}")])
    }

    pub(super) fn wave29a_champion_ltb_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if self.is_etb_self_trigger
            || !self.trigger_is_self
            || self.trigger_mode.as_deref() != Some("ChangesZone")
        {
            return None;
        }
        let _ = self.wave29a_card_champion_pair()?;
        let mut probe = p.clone();
        if probe.take("Defined").as_deref() != Some("Remembered")
            || probe.take("Origin").as_deref() != Some("Exile")
            || probe.take("Destination").as_deref() != Some("Battlefield")
            || !probe.exhausted()
        {
            return None;
        }
        *p = probe;
        *sub = None;
        Some(vec!["Effect::ReturnChampioned".to_string()])
    }

    // =========================================================================
    // 5. `ChangeZone` event-object Graveyard -> Exile (`Necropotence` discard,
    //    `Purgatory`, `Chaos Shrine's Black Crystal`, `Promise of Tomorrow`, ...)
    //    & Slot-2 `CardInFirstTargetsGraveyard` shuffle into library
    //    (`Dwell on the Past`, `Krosan Reclamation`, `Memory's Journey`, ...)
    // =========================================================================

    pub(super) fn wave29a_event_graveyard_to_exile(
        &mut self,
        p: &mut Params,
        defined: &str,
    ) -> Option<Vec<String>> {
        if self.in_delayed || p.has("WithCountersType") || p.has("ExileFaceDown") {
            return None;
        }
        if self.trigger_mode.as_deref() == Some("Discarded")
            && defined == "TriggeredCard"
            && !self.has_linked_exile_reader()
            && self
                .oracle
                .to_ascii_lowercase()
                .contains("whenever you discard a card, exile that card from your graveyard")
        {
            let mut probe = p.clone();
            probe.take("IsCurse");
            if probe.exhausted() {
                *p = probe;
                return Some(vec![
                    "Effect::ExileDiscardedCardFromYourGraveyard".to_string(),
                ]);
            }
        }
        if !self.trigger_is_self
            && self.has_linked_exile_reader()
            && matches!(
                self.trigger_mode.as_deref(),
                Some("ChangesZone" | "Sacrificed" | "Discarded")
            )
            && matches!(
                defined,
                "TriggeredNewCardLKICopy"
                    | "TriggeredNewCard"
                    | "TriggeredCardLKICopy"
                    | "TriggeredCard"
            )
        {
            let mut probe = p.clone();
            probe.take("IsCurse");
            if let Some(imp) = probe.take("Imprint")
                && imp != "True"
            {
                return None;
            }
            if probe.exhausted() {
                *p = probe;
                return Some(vec![
                    "Effect::exile_linked(TargetSpec::EventObject)".to_string(),
                ]);
            }
        }
        None
    }

    pub(super) fn wave29a_first_target_graveyard_shuffle_into_library(
        p: &mut Params,
        target: &str,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if chain.target.as_deref() != Some("TargetSpec::AnyPlayer")
            && chain.target.as_deref() != Some("TargetSpec::AnyOpponent")
            && chain.target.as_deref() != Some("TargetSpec::Player(PlayerRel::Chosen)")
        {
            return None;
        }
        let mut probe = p.clone();
        if probe.take("Origin").as_deref() != Some("Graveyard")
            || probe.take("Destination").as_deref() != Some("Library")
            || probe.take("Shuffle").as_deref() != Some("True")
        {
            return None;
        }
        probe.take("IsCurse");
        if probe.peek("Mandatory") == Some("True") {
            probe.take("Mandatory");
        }
        if !matches!(
            probe.take("LibraryPosition").as_deref(),
            None | Some("0" | "-1")
        ) {
            return None;
        }
        if !probe.exhausted() {
            return None;
        }
        *p = probe;
        Some(vec![
            format!("Effect::PutOnBottomOfLibraryFromGraveyard {{ target: {target} }}"),
            "Effect::ShuffleLibrary { who: PlayerRel::FirstTargetPlayerOrController }".to_string(),
        ])
    }

    // =========================================================================
    // 6. Linked-exile helpers (`Purgatory`, `Skyship Weatherlight`,
    //    `Omenpath Journey`, `Negative Zone Portal`, `Trial of a Time Lord`)
    // =========================================================================

    pub(super) fn wave29a_linked_exile_all_you_own(&self) -> bool {
        let mut saw_producer = false;
        let check = |body: &str| -> Option<bool> {
            let (_, p) = Params::parse(body)?;
            if p.peek("Destination") != Some("Exile") && p.peek("DestinationZone") != Some("Exile")
            {
                return None;
            }
            if p.peek("Origin") == Some("Library")
                && matches!(p.peek("DefinedPlayer"), None | Some("You"))
            {
                return Some(true);
            }
            if p.peek("ChangeType")
                .is_some_and(|ct| Self::strip_your_graveyard_owner(ct).is_some())
            {
                return Some(true);
            }
            Some(false)
        };
        for v in self.svars.values() {
            if let Some(ok) = check(v) {
                if !ok {
                    return false;
                }
                saw_producer = true;
            }
        }
        for a in &self.body.abilities {
            if a.contains("Find::EXILE_LINKED") {
                saw_producer = true;
            } else if a.contains("ExileTargetsWithSource")
                || a.contains("ExileLinked")
                || a.contains("exile_linked")
                || a.contains("ExileAll")
            {
                if !a.contains("OwnedByYou") {
                    return false;
                }
                saw_producer = true;
            }
        }
        saw_producer
    }

    pub(super) fn wave29a_linked_exile_card_is_permanent(&self) -> bool {
        let lower = self.oracle.to_ascii_lowercase();
        (lower.contains("whenever a nontoken creature is put into your graveyard")
            || lower.contains("whenever a creature ")
            || lower.contains("exile target permanent")
            || lower.contains("exile target creature")
            || lower.contains("exile target artifact"))
            && !lower.contains("instant")
            && !lower.contains("sorcery")
            && !lower.contains("top card")
            && !lower.contains("from a graveyard")
            && !lower.contains("from an opponent's graveyard")
            && !lower.contains("from your hand")
    }

    pub(super) fn wave29a_linked_exile_constrained_zone(
        &mut self,
        origin: &str,
        raw_valid: &str,
    ) -> Option<(String, bool, String)> {
        if origin != "Exile"
            || self.on_a_spell
            || self.card_has_unlinked_exile_producer()
            || !raw_valid.contains("ExiledWithSource")
        {
            return None;
        }
        let stripped = Self::strip_exiled_with_source_filter(raw_valid)?;
        if stripped.contains("ExiledWith")
            || stripped.contains("IsImprinted")
            || stripped.contains("IsRemembered")
        {
            return None;
        }
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let is_own = self.wave29a_linked_exile_all_you_own();
        Some(("ChooseZone::LinkedExile".to_string(), is_own, expr))
    }

    // =========================================================================
    // 7. `ChangeZoneAll` extensions (`Trial of a Time Lord`, `River's Rebuke`,
    //    `Underworld Cerberus`, `Nurgle's Conscription`, `Hubris`, `Essence Reliquary`)
    // =========================================================================

    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29a_change_zone_all_extra(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        // 7a. `Origin$ Exile | Destination$ Library | LibraryPosition$ -1` on
        // `ExiledWithSource` (`Trial of a Time Lord` `DBGuilty`).
        if !self.on_a_spell
            && p.peek("Origin") == Some("Exile")
            && p.peek("Destination") == Some("Library")
            && p.peek("LibraryPosition") == Some("-1")
            && !p.has("Shuffle")
            && !self.card_has_unlinked_exile_producer()
        {
            let mut probe = p.clone();
            probe.take("Origin");
            probe.take("Destination");
            probe.take("LibraryPosition");
            probe.take("IsCurse");
            if probe.has("Defined") || (target.is_some() && !prior_target_used) {
                return None;
            }
            let random = match probe.take("RandomOrder").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let valid = probe.take("ChangeType")?;
            let stripped = Self::strip_exiled_with_source_filter(&valid)?;
            if !Self::wave29a_only_condition_keys_left(&probe) {
                return None;
            }
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            let filter_ref = if expr == "Filter::Any" {
                "&Filter::Any".to_string()
            } else {
                format!("&{}", self.body.filter_static("LINKED_BOTTOM", &expr))
            };
            let bottom_move = if random {
                "ChooseMove::BottomOfLibraryRandom"
            } else {
                "ChooseMove::BottomOfLibraryChosenOrder"
            };
            *p = probe;
            return Some(vec![format!(
                "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: ChooseZone::LinkedExile, \
                 filter: {filter_ref}, min: 0, max: Amount::Fixed(0), reveal_chosen: false, \
                 movement: ChooseMove::None, not_chosen_movement: {bottom_move}, \
                 then: &[], for_unchosen: &[] }}"
            )]);
        }

        // 7b. `ValidTgts$ Player | Origin$ Battlefield | Destination$ Hand` where
        // Oracle explicitly scopes to `"target player controls"` (`River's Rebuke`).
        if targets_a_player
            && p.peek("Origin") == Some("Battlefield")
            && p.peek("Destination") == Some("Hand")
            && self
                .oracle
                .to_ascii_lowercase()
                .contains("target player controls")
        {
            let mut probe = p.clone();
            probe.take("Origin");
            probe.take("Destination");
            probe.take("IsCurse");
            if probe.has("Defined") {
                return None;
            }
            let valid = probe.take("ChangeType")?;
            if valid.contains("Ctrl")
                || valid.contains("Own")
                || !Self::wave29a_only_condition_keys_left(&probe)
            {
                return None;
            }
            let base = self.filter_expr(&valid)?;
            let expr = if base == "Filter::Any" {
                "Filter::ControlledBy(PlayerRel::Chosen)".to_string()
            } else {
                format!("Filter::And(&[{base}, Filter::ControlledBy(PlayerRel::Chosen)])")
            };
            let name = self.body.filter_static("BOUNCE_ALL", &expr);
            *p = probe;
            return Some(vec![format!(
                "Effect::ReturnAllToHand {{ filter: &{name}, opponents_only: false }}"
            )]);
        }

        // 7c. Single-zone `Origin$ Graveyard | UseAllOriginZones$ True` with no
        // `Defined$` (`Underworld Cerberus`).
        if p.peek("Origin") == Some("Graveyard")
            && p.peek("UseAllOriginZones") == Some("True")
            && !p.has("Defined")
        {
            let mut probe = p.clone();
            probe.take("UseAllOriginZones");
            if let Some(effs) = self.wave28d_change_zone_all(
                &mut probe,
                target,
                targets_a_player,
                prior_target_used,
            ) {
                *p = probe;
                return Some(effs);
            }
        }

        // 7d. `Origin$ Graveyard | Destination$ Exile | Defined$ TargetedOwner`
        // after a prior `TargetSpec::CardInGraveyard` step (`Nurgle's Conscription`).
        if prior_target_used
            && p.peek("Origin") == Some("Graveyard")
            && p.peek("Destination") == Some("Exile")
            && p.peek("Defined") == Some("TargetedOwner")
            && target.is_some_and(|t| t.starts_with("TargetSpec::CardInGraveyard("))
        {
            let mut probe = p.clone();
            probe.take("Origin");
            probe.take("Destination");
            probe.take("Defined");
            probe.take("IsCurse");
            if matches!(probe.take("ChangeType").as_deref(), None | Some("Card"))
                && Self::wave29a_only_condition_keys_left(&probe)
            {
                *p = probe;
                return Some(vec![
                    "Effect::ExileGraveyard { player: PlayerRel::OwnerOfTarget }".to_string(),
                ]);
            }
        }

        // 7e. `Origin$ Battlefield | Destination$ Hand | ChangeType$ TargetedCard.Self,<Att>.AttachedTo Targeted`
        // (`Hubris`, `Essence Reliquary`). Safe because `ChangeZoneAll` has a
        // single battlefield target (CR 608.2b fizzles if illegal on resolution),
        // and `ReturnAllToHand` evaluates `Filter::Reference` against `res.targets[0]`.
        if !targets_a_player
            && !prior_target_used
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && p.peek("Origin") == Some("Battlefield")
            && p.peek("Destination") == Some("Hand")
        {
            let mut probe = p.clone();
            probe.take("Origin");
            probe.take("Destination");
            probe.take("IsCurse");
            if let Some(u) = probe.take("UseAllOriginZones")
                && u != "True"
            {
                return None;
            }
            if probe.has("Defined") {
                return None;
            }
            let valid = probe.take("ChangeType")?;
            let (first_alt, rest_alt) = valid.split_once(',')?;
            if !matches!(first_alt.trim(), "TargetedCard.Self" | "Targeted.Self")
                || rest_alt.contains(',')
                || !Self::wave29a_only_condition_keys_left(&probe)
            {
                return None;
            }
            let stripped_att = Self::wave29a_strip_attached_to_targeted(rest_alt.trim())?;
            let att_base = self.filter_expr(&stripped_att)?;
            let att_expr = if att_base == "Filter::Any" {
                "Filter::AttachedTo(&Filter::Reference)".to_string()
            } else {
                format!("Filter::And(&[{att_base}, Filter::AttachedTo(&Filter::Reference)])")
            };
            let full_expr = format!("Filter::Or(&[Filter::Reference, {att_expr}])");
            let name = self.body.filter_static("BOUNCE_WITH_ATTACHED", &full_expr);
            *p = probe;
            return Some(vec![format!(
                "Effect::ReturnAllToHand {{ filter: &{name}, opponents_only: false }}"
            )]);
        }

        // 7f. Inside `CaptureAffected::then`, `first` has already consumed any
        // outer target, so `ChangeZoneAll` (`Abyssal Harvester`) may run with
        // `prior_target_used = true`.
        if self.capture_scope.is_some() && !prior_target_used && !targets_a_player {
            let mut probe = p.clone();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics_len = self.body.statics.len();
            if let Some(effs) =
                self.shuffle_into_library_with_prior_inner(&mut probe, target, false, true)
            {
                *p = probe;
                return Some(effs);
            }
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
        }

        None
    }

    fn wave29a_strip_attached_to_targeted(alt: &str) -> Option<String> {
        let (base, rest) = alt.split_once('.')?;
        let mut saw = false;
        let mut kept = Vec::new();
        for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
            if matches!(atom, "AttachedTo Targeted" | "AttachedTo TargetedCard") {
                saw = true;
            } else {
                kept.push(atom);
            }
        }
        if !saw {
            return None;
        }
        if kept.is_empty() {
            Some(base.to_string())
        } else {
            Some(format!("{base}.{}", kept.join("+")))
        }
    }

    // =========================================================================
    // 8. `RememberChanged` follow-up extensions (`Anchor to Reality`,
    //    `Offspring's Revenge`, `Abyssal Harvester`)
    // =========================================================================

    /// Extended `Count$Compare <Y> <CMP>.<T>.<E>` inside `CaptureAffected::then`
    /// supporting `Remembered$CardManaCost` / `Power` / `Toughness` and RHS
    /// `SVar` comparisons against `Sacrificed$CardManaCost` (*Anchor to Reality*).
    pub(super) fn wave29a_remembered_compare_amount(&self, base_raw: &str) -> Option<String> {
        self.capture_scope.as_ref()?;
        let rest = base_raw.strip_prefix("Count$Compare ")?;
        let (lhs, else_raw) = rest.rsplit_once('.')?;
        let (cond_spec, then_raw) = lhs.rsplit_once('.')?;
        let then_n: u32 = then_raw.trim().parse().ok()?;
        let else_n: u32 = else_raw.trim().parse().ok()?;
        let (var, cmp) = cond_spec.trim().rsplit_once(' ')?;
        let var_def = self.svars.get(var.trim())?.trim();
        let left_amt = self.wave27c_remembered_amount(var_def)?;
        let cmp = cmp.trim();
        let cond = if let Some(c) = Self::wave15_amount_cmp(&left_amt, cmp) {
            c
        } else if cmp.len() > 2 {
            let (op_code, rhs_var) = cmp.split_at(2);
            let op = match op_code {
                "EQ" => "AmountCmp::Equal",
                "NE" => "AmountCmp::NotEqual",
                "LT" => "AmountCmp::LessThan",
                "LE" => "AmountCmp::AtMost",
                "GT" => "AmountCmp::GreaterThan",
                "GE" => "AmountCmp::AtLeast",
                _ => return None,
            };
            let rhs_def = self.svars.get(rhs_var.trim())?.trim();
            let right_amt = match rhs_def {
                "Sacrificed$CardManaCost" if self.cost_sacrifices_other => {
                    "Amount::SacrificedManaValue"
                }
                "Sacrificed$CardPower" if self.cost_sacrifices_other => "Amount::SacrificedPower",
                "Sacrificed$CardToughness" if self.cost_sacrifices_other => {
                    "Amount::SacrificedToughness"
                }
                _ => return None,
            };
            format!(
                "Condition::AmountCompare {{ left: &{left_amt}, op: {op}, right: &{right_amt} }}"
            )
        } else {
            return None;
        };
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond}, \
             then: &Amount::Fixed({then_n}), otherwise: &Amount::Fixed({else_n}) }}"
        ))
    }

    /// `Offspring's Revenge`: `CopyPermanent | Defined$ Remembered | ... |
    /// PumpKeywords$ Haste | PumpDuration$ UntilYourNextTurn` inside
    /// `CaptureAffected::then`. Wraps `CreateTokenCopy` in a nested
    /// `CaptureAffected` that grants `KeywordSet::HASTE` for
    /// `Duration::UntilYourNextTurn` to each created token.
    pub(super) fn wave29a_copy_permanent_remembered_haste_next_turn(
        &mut self,
        p: &mut Params,
        copy_target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("PumpKeywords") != Some("Haste")
            || p.peek("PumpDuration") != Some("UntilYourNextTurn")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Defined");
        probe.take("PumpKeywords");
        probe.take("PumpDuration");
        probe
            .entries
            .push(("Defined".to_string(), "DelayTriggerRemembered".to_string()));
        let saved_in_delayed = std::mem::replace(&mut self.in_delayed, true);
        let effs = self.copy_permanent_effect(&mut probe, copy_target, false, targets_a_player);
        self.in_delayed = saved_in_delayed;
        let effs = effs?;
        if effs.is_empty() || !Self::wave29a_only_condition_keys_left(&probe) {
            return None;
        }
        *p = probe;
        Some(vec![format!(
            "Effect::ForEachRemembered {{ filter: &Filter::Any, \
             effects: &[Effect::CaptureAffected {{ first: &[{}], \
             then: &[Effect::ForEachRemembered {{ filter: &Filter::Any, \
             effects: &[Effect::ModifyEvent {{ modifier: Modifier::AddKeyword(KeywordSet::HASTE), \
             duration: Duration::UntilYourNextTurn }}] }}] }}] }}",
            effs.join(", ")
        )])
    }

    /// True when a nested `CopyPermanent | Defined$ Remembered | RememberTokens$ True`
    /// inside a non-battlefield `CaptureAffected::then` (`Abyssal Harvester`) is
    /// followed only by battlefield `!IsRemembered` sweep(s) and `Cleanup | ClearRemembered$ True`,
    /// where shadowing the outer non-battlefield remembered card is 100% equivalent
    /// to appending the battlefield token!
    pub(super) fn wave29a_is_non_bf_token_capture_followup(
        &self,
        sp: &Params,
        sub_override: Option<&str>,
    ) -> bool {
        if self
            .capture_scope
            .as_ref()
            .is_none_or(|s| s.from_battlefield)
            || sp.peek("Defined") != Some("Remembered")
            || sp.peek("RememberTokens") != Some("True")
        {
            return false;
        }
        let Some(next_name) = sub_override.or_else(|| sp.peek("SubAbility")) else {
            return false;
        };
        let Some(next_body) = self.svars.get(next_name.trim()) else {
            return false;
        };
        let Some((next_api, mut np)) = Params::parse(next_body) else {
            return false;
        };
        np.drop_prose();
        if next_api != "ChangeZoneAll"
            || np.peek("Origin") != Some("Battlefield")
            || !np
                .peek("ChangeType")
                .is_some_and(|ct| ct.contains("!IsRemembered"))
        {
            return false;
        }
        let Some(clean_name) = np.take("SubAbility") else {
            return false;
        };
        let Some(clean_body) = self.svars.get(clean_name.trim()) else {
            return false;
        };
        let Some((c_api, mut cp)) = Params::parse(clean_body) else {
            return false;
        };
        cp.drop_prose();
        c_api == "Cleanup"
            && cp.take("ClearRemembered").as_deref() == Some("True")
            && cp.exhausted()
    }

    /// Pass 3: `Clone | Defined$ Remembered[.<Filter>] | [CloneTarget$ Self] |
    /// [Optional$ True] | [Duration$ UntilEndOfTurn|Permanent]` inside a
    /// single-object non-battlefield `CaptureAffected::then`
    /// (*Lazav, Familiar Stranger*).
    pub(super) fn wave29a_capture_clone_step(
        &mut self,
        p: &mut Params,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        let scope = self.capture_scope.as_ref()?;
        if !scope.single_object || scope.from_battlefield || self.on_a_spell || targets_here {
            return None;
        }
        let mut probe = p.clone();
        if !matches!(probe.take("CloneTarget").as_deref(), None | Some("Self")) {
            return None;
        }
        let defined = probe.take("Defined")?;
        let filter_ref = if defined == "Remembered" {
            "&Filter::Any".to_string()
        } else {
            let rest = defined.strip_prefix("Remembered.")?;
            if rest.is_empty() || rest.contains(',') || rest.contains("Remembered") {
                return None;
            }
            let expr = self.filter_expr_in_zone(&format!("Card.{rest}"), true)?;
            format!("&{}", self.body.filter_static("REM_CLONE", &expr))
        };
        let until_eot = match probe.take("Duration").as_deref() {
            None | Some("Permanent") => false,
            Some("UntilEndOfTurn") => true,
            Some(_) => return None,
        };
        let optional = match probe.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let mods = self.copy_mods(&mut probe, true)?;
        if !Self::wave29a_only_condition_keys_left(&probe) {
            return None;
        }
        let become_copy = format!(
            "Effect::BecomeCopy {{ subject: TargetSlot::This, subject_filter: None, \
             original: TargetSpec::EventObject, original_is_second_target: false, \
             until_eot: {until_eot}, mods: &[{}] }}",
            mods.join(", ")
        );
        let inner = if optional {
            format!("Effect::MayDo {{ effects: &[{become_copy}] }}")
        } else {
            become_copy
        };
        *p = probe;
        Some(vec![format!(
            "Effect::ForEachRemembered {{ filter: {filter_ref}, effects: &[{inner}] }}"
        )])
    }

    /// Pass 3: whole-card check for `Endless Horizons` where an `Origin$ Library |
    /// Destination$ Exile | RememberChanged$ True` ability (already emitted as
    /// `Find::EXILE_LINKED` because Oracle names `"exiled with <CARDNAME>"`) pairs
    /// exclusively with an `Origin$ Exile | Destination$ Hand | Hidden$ True |
    /// ChangeType$ <F>.IsRemembered[+YouOwn]` ability.
    pub(super) fn wave29a_is_linked_library_exile_remember_pair(&self) -> bool {
        if self.on_a_spell || self.card_has_unlinked_exile_producer() {
            return false;
        }
        let source_name = self
            .script_name
            .split(',')
            .next()
            .unwrap_or(self.script_name)
            .to_ascii_lowercase();
        if !self
            .oracle
            .to_ascii_lowercase()
            .contains(&format!("exiled with {source_name}"))
        {
            return false;
        }
        let mut search_svar: Option<&str> = None;
        let mut retrieve_svar: Option<&str> = None;
        for (name, body) in self.svars {
            let Some((api, mut sp)) = Params::parse(body) else {
                if body.contains("Remembered") || body.contains("IsRemembered") {
                    return false;
                }
                continue;
            };
            sp.drop_prose();
            if api == "ChangeZone"
                && sp.peek("Origin") == Some("Library")
                && sp.peek("Destination") == Some("Exile")
                && sp.peek("RememberChanged") == Some("True")
                && !sp.has("SubAbility")
                && matches!(sp.peek("DefinedPlayer"), None | Some("You"))
            {
                if search_svar.is_some() {
                    return false;
                }
                search_svar = Some(name.as_str());
            } else if api == "ChangeZone"
                && sp.peek("Origin") == Some("Exile")
                && matches!(sp.peek("Destination"), Some("Hand" | "Graveyard"))
                && sp.peek("Hidden") == Some("True")
                && !sp.has("SubAbility")
                && sp
                    .peek("ChangeType")
                    .is_some_and(|ct| ct.contains("IsRemembered") && !ct.contains("!IsRemembered"))
            {
                if retrieve_svar.is_some() {
                    return false;
                }
                retrieve_svar = Some(name.as_str());
            } else if body.contains("Remembered") || body.contains("IsRemembered") {
                return false;
            }
        }
        search_svar.is_some() && retrieve_svar.is_some()
    }

    fn wave29a_only_condition_keys_left(p: &Params) -> bool {
        p.entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
    }
}
