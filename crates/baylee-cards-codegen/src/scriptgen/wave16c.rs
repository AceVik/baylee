//! Wave 16c scriptgen lowerings for atomic delayed observers (`DB$ Effect`),
//! `R:Event$ Moved` / `K:ETBReplacement:Other:` entry choices & replacements,
//! `ChoosePlayer` resolution chains, and `ChooseNumber` variable payments:
//!
//! - `Effect::EventObjectDealsDamageEach` for watched `DamageDoneOnce` ->
//!   `DamageAll | DamageSource$ Remembered` (*Arcbond*, CR 120.2b, CR 603.7),
//!   `Trigger::OneOrMoreAttack` batch count via `TriggerObjectsAttackers$Valid`
//!   (*Basri Ket*, CR 508.3a, CR 603.2c), source-excluding `!IsRemembered` and
//!   `DamageSource$ EffectSource` (*Electric Seaweed*, *Massacre Girl*),
//!   `AttackersDeclaredOneTarget` delayed triggers (*Garruk, Curse Breaker*),
//!   and player-targeted `CantBlock` / `CantAttack` (*Ember Gale*,
//!   *Headliner Scarlett*).
//! - `ReplacementRule::{EnterTappedIf, EnterUntappedIf}` (*Archelos, Lagoon
//!   Mystic*, CR 614.1c, CR 616.1), pre-counter `ChooseType` / `ChoosePlayer`
//!   chaining into `PutCounter` self-ETB replacements (*Banner of Kinship*,
//!   *Canker Abomination*, CR 614.12a, CR 614.16), `EnterModifier::BecomeDayIfNeither`
//!   (*Brimstone Vandal* and Innistrad day/night cycle, CR 726.2, CR 730.2),
//!   `EnterModifier::ChooseAnyPlayer` (*True-Name Nemesis*, *Stuffy Doll*,
//!   *Saskia the Unyielding*, *Sawhorn Nemesis*), `EnterModifier::LookAtOpponentHand`
//!   (*Anointed Peacekeeper*, *Sorcerous Spyglass*), and as-enters
//!   `EnterModifier::ChooseNumber` (*Talion, the Kindly Lord*, *Sanctum Prelate*).
//! - `Effect::ChoosePlayerAtRandom` (*Ruhan of the Fomori*, *Knight Rampager*,
//!   *Raving Dead*, CR 101.4) and multi-recipient / multi-step `ChoosePlayer`
//!   resolution chains (*Benevolent Offering*, *Intellectual Offering*,
//!   *Sandstone Oracle*, *Slithermuse*, *Discerning Financier*).
//! - `Effect::ChooseNumberEx` variable energy/mana/life payment chains paired
//!   with Forge's `UnlessCost$ [Mandatory ]PayEnergy<X>|<X>|PayLife<X> |
//!   UnlessPayer$ You | UnlessSwitched$ True` and optional `ConditionCheckSVar$ X`
//!   `"If you do"` / `"When you do"` gating (*Die Young*, *Harnessed Lightning*,
//!   *Aether Spike*, *Aether Refinery*, *Wrath of the Skies*, *Rampaging
//!   Aetherhood*, *Pia Nalaar, Chief Mechanic*, *Localized Destruction*,
//!   *Territorial Aetherkite*, *Well of Lost Dreams*, *Plunge into Darkness*,
//!   *Karn, Living Legacy*, *Vault 112: Sadistic Simulation*).

use super::effects::EffectSubject;
use super::saga_ward_convoke::ExtraOutcome;
use super::{Chain, Params, Tx};

enum ChosenStep {
    Handled(Vec<String>),
    Pass,
}

impl Tx<'_> {
    /// True when the permanent chooses a player (`ChooseOpponent` or
    /// `ChooseAnyPlayer`) as it enters the battlefield (`CR 614.12a`).
    pub(super) fn has_chosen_source_player(&self) -> bool {
        self.body
            .enter_modifiers
            .iter()
            .any(|m| m == "EnterModifier::ChooseOpponent" || m == "EnterModifier::ChooseAnyPlayer")
    }

    /// True when the permanent chooses a number (`EnterModifier::ChooseNumber`)
    /// as it enters the battlefield (`CR 614.12a`).
    pub(super) fn has_chosen_source_number(&self) -> bool {
        self.body
            .enter_modifiers
            .iter()
            .any(|m| m.starts_with("EnterModifier::ChooseNumber {"))
    }

    /// Wave 16c `K:ETBReplacement:Other:<svar>` handler for:
    /// - `DB$ ChoosePlayer | Choices$ Player` -> `EnterModifier::ChooseAnyPlayer`
    /// - `DB$ ChoosePlayer (Opponent) -> RevealHand (Look$ True) -> NameCard [-> Cleanup]`
    ///   -> `EnterModifier::LookAtOpponentHand` + `EnterModifier::ChooseCardName*`
    /// - `DB$ ChooseNumber` -> `EnterModifier::ChooseNumber { min, max }`
    ///
    /// Returns `Some(res)` when `<svar>` matches a Wave 16c shape (or fails inside
    /// one), or `None` to pass through to legacy handlers.
    pub(super) fn wave16c_etb_other(&mut self, svar: &str) -> ExtraOutcome {
        let Some(body) = self.svars.get(svar).cloned() else {
            return ExtraOutcome::Pass;
        };
        let Some((api, mut p)) = Params::parse(&body) else {
            return ExtraOutcome::Pass;
        };
        p.drop_prose();
        match api.as_str() {
            "ChoosePlayer" => {
                if p.has("Secretly") || p.has("RememberChosen") || p.has("Random") {
                    return ExtraOutcome::Pass;
                }
                if !matches!(p.peek("Defined"), None | Some("You")) {
                    return ExtraOutcome::Pass;
                }
                if p.peek("Choices") == Some("Player") && !p.has("SubAbility") {
                    p.take("Defined");
                    p.take("Choices");
                    p.take("ChoiceTitle");
                    if !p.exhausted() {
                        return ExtraOutcome::Handled(self.deny(format!(
                            "as-enters `ChoosePlayer` with `{}`",
                            p.first_key().unwrap_or_default()
                        )));
                    }
                    self.body
                        .enter_modifiers
                        .push("EnterModifier::ChooseAnyPlayer".to_string());
                    return ExtraOutcome::Handled(Some(()));
                }
                if matches!(p.peek("Choices"), Some("Player.Opponent" | "Opponent"))
                    && p.has("SubAbility")
                {
                    return ExtraOutcome::Handled(self.wave16c_etb_look_hand_and_name_card(p));
                }
                ExtraOutcome::Pass
            }
            "ChooseNumber" => ExtraOutcome::Handled(self.wave16c_etb_choose_number(p)),
            _ => ExtraOutcome::Pass,
        }
    }

    fn wave16c_etb_look_hand_and_name_card(&mut self, mut p: Params) -> Option<()> {
        p.take("Defined");
        p.take("Choices");
        p.take("ChoiceTitle");
        let look_svar = p.take("SubAbility")?;
        if !p.exhausted() {
            return self.deny(format!(
                "as-enters `ChoosePlayer` with `{}`",
                p.first_key().unwrap_or_default()
            ));
        }
        let look_body = self.svars.get(look_svar.trim())?.clone();
        let (look_api, mut lp) = Params::parse(&look_body)?;
        lp.drop_prose();
        if look_api != "RevealHand"
            || !matches!(
                lp.take("Defined").as_deref(),
                Some("ChosenPlayer" | "Player.Chosen")
            )
            || lp.take("Look").as_deref() != Some("True")
        {
            return self.deny("as-enters `ChoosePlayer` outside look-at-hand + `NameCard`".into());
        }
        let name_svar = lp.take("SubAbility")?;
        if !lp.exhausted() {
            return self.deny("as-enters `RevealHand` with extra parameters".into());
        }
        let name_body = self.svars.get(name_svar.trim())?.clone();
        let (name_api, mut np) = Params::parse(&name_body)?;
        np.drop_prose();
        if name_api != "NameCard" {
            return self.deny("as-enters `RevealHand` not followed by `NameCard`".into());
        }
        np.take("ValidDescription");
        if !matches!(np.take("Defined").as_deref(), None | Some("You")) {
            return self.deny("as-enters `NameCard` for another player".into());
        }
        let valid_cards = np.take("ValidCards");
        if let Some(clean_svar) = np.take("SubAbility") {
            let clean_body = self.svars.get(clean_svar.trim())?.clone();
            let (clean_api, mut cp) = Params::parse(&clean_body)?;
            cp.drop_prose();
            if clean_api != "Cleanup"
                || cp.take("ClearChosenPlayer").as_deref() != Some("True")
                || !cp.exhausted()
            {
                return self.deny("as-enters `NameCard` with unsupported `SubAbility$`".into());
            }
        }
        if !np.exhausted() {
            return self.deny(format!(
                "as-enters `NameCard` with `{}`",
                np.first_key().unwrap_or_default()
            ));
        }
        let name_mod = match valid_cards {
            None => "EnterModifier::ChooseCardName".to_string(),
            Some(vc) => {
                let Some(excluded) = Self::wave16c_excluded_card_types(&vc) else {
                    return self.deny(format!("as-enters `NameCard` filter `{vc}`"));
                };
                format!("EnterModifier::ChooseCardNameExcept({excluded})")
            }
        };
        self.body
            .enter_modifiers
            .push("EnterModifier::LookAtOpponentHand".to_string());
        self.body.enter_modifiers.push(name_mod);
        Some(())
    }

    fn wave16c_excluded_card_types(vc: &str) -> Option<String> {
        let rest = vc.trim().strip_prefix("Card.")?;
        if rest.contains(',') {
            return None;
        }
        let mut types = Vec::new();
        for atom in rest.split('+').map(str::trim).filter(|a| !a.is_empty()) {
            let ty_word = atom.strip_prefix("non")?;
            types.push(super::card_type_const(ty_word)?);
        }
        match types.as_slice() {
            [] => None,
            [single] => Some((*single).to_string()),
            many => {
                let mut expr = format!("{}.union({})", many[0], many[1]);
                for t in &many[2..] {
                    expr = format!("{expr}.union({t})");
                }
                Some(expr)
            }
        }
    }

    fn wave16c_etb_choose_number(&mut self, mut p: Params) -> Option<()> {
        if !matches!(p.take("Defined").as_deref(), None | Some("You")) {
            return self.deny("as-enters `ChooseNumber` for another player".into());
        }
        if p.has("Random") || p.has("Secretly") || p.has("SubAbility") || p.has("ListTitle") {
            return self.deny("unsupported as-enters `ChooseNumber` modifier".into());
        }
        let min: u8 = match p.take("Min") {
            None => 0,
            Some(raw) => raw
                .trim()
                .parse::<u8>()
                .ok()
                .or_else(|| self.deny(format!("as-enters `ChooseNumber` min `{raw}`")))?,
        };
        let max: u8 = match p.take("Max") {
            None => u8::MAX,
            Some(raw) => raw
                .trim()
                .parse::<u8>()
                .ok()
                .or_else(|| self.deny(format!("as-enters `ChooseNumber` max `{raw}`")))?,
        };
        if min > max || !p.exhausted() {
            return self.deny("invalid as-enters `ChooseNumber` bounds or parameters".into());
        }
        self.body.enter_modifiers.push(format!(
            "EnterModifier::ChooseNumber {{ min: {min}, max: {max} }}"
        ));
        Some(())
    }

    /// Atomic-rollback entry point for Wave 16c `R:Event$ Moved` replacements.
    pub(super) fn wave16c_replacement(&mut self, spec: &str) -> Option<()> {
        let saved_statics = self.body.statics.len();
        let saved_enter_mods = self.body.enter_modifiers.len();
        let saved_abilities = self.body.abilities.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let res = self.wave16c_replacement_inner(spec);
        if res.is_none() {
            self.body.statics.truncate(saved_statics);
            self.body.enter_modifiers.truncate(saved_enter_mods);
            self.body.abilities.truncate(saved_abilities);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
        }
        res
    }

    fn wave16c_replacement_inner(&mut self, spec: &str) -> Option<()> {
        let (event, p) = Params::parse(spec)?;
        if event != "Moved" {
            return None;
        }
        self.wave16c_conditional_external_enter_tap_untap(p.clone())
            .or_else(|| self.wave16c_self_etb_choice_and_counters(p.clone()))
            .or_else(|| self.wave16c_self_etb_daytime_neither(p))
    }

    /// `R:Event$ Moved | ValidCard$ <valid> | Destination$ Battlefield |
    /// IsPresent$ Card.Self+tapped|Card.Self+untapped | ReplaceWith$ <Tap|Untap> |
    /// ReplacementResult$ Updated | ActiveZones$ Battlefield`
    /// -> `ReplacementRule::{EnterTappedIf, EnterUntappedIf}` (*Archelos, Lagoon Mystic*).
    fn wave16c_conditional_external_enter_tap_untap(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("ReplacementResult").as_deref() != Some("Updated")
            || !matches!(p.take("Origin").as_deref(), None | Some("Any" | "All"))
            || !matches!(p.take("Secondary").as_deref(), None | Some("True"))
        {
            return None;
        }
        let valid = p.take("ValidCard")?;
        if valid.contains("Self") {
            return None;
        }
        let with = p.take("ReplaceWith")?;
        let cond = match (
            p.take("IsPresent").as_deref(),
            p.take("PresentCompare").as_deref(),
        ) {
            (Some("Card.Self+tapped"), None | Some("EQ1" | "GE1")) => {
                "Condition::SourceMatches(&Filter::Tapped)".to_string()
            }
            (Some("Card.Self+untapped"), None | Some("EQ1" | "GE1")) => {
                "Condition::SourceMatches(&Filter::Untapped)".to_string()
            }
            _ => return None,
        };
        if !p.exhausted() {
            return None;
        }
        let (api, mut body) = self.svars.get(with.trim()).and_then(|s| Params::parse(s))?;
        body.drop_prose();
        if body.take("ETB").as_deref() != Some("True")
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || !body.exhausted()
        {
            return None;
        }
        let rule = match api.as_str() {
            "Tap" => "EnterTappedIf",
            "Untap" => "EnterUntappedIf",
            _ => return None,
        };
        let cleaned_valid = valid.replace("+Other", "").replace(".Other", "");
        let filter_ref = if matches!(cleaned_valid.trim(), "Permanent" | "Card") {
            "Filter::Any".to_string()
        } else {
            let expr = self.filter_expr(&cleaned_valid)?;
            self.body.filter_static("ENTER", &expr)
        };
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::{rule} {{ \
             filter: &{filter_ref}, condition: {cond} }})"
        ));
        Some(())
    }

    /// Self-ETB `R:Event$ Moved` chaining `ChooseType` (`Creature` / `Basic Land`)
    /// or `ChoosePlayer` (`Opponent` / `Player`) into `PutCounter | ETB$ True`
    /// (*Banner of Kinship*, *Canker Abomination*; `CR 614.12a`, `CR 614.16`).
    fn wave16c_self_etb_choice_and_counters(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ReplacementResult").as_deref() != Some("Updated")
            || !matches!(p.take("Origin").as_deref(), None | Some("Any" | "All"))
            || !matches!(p.take("ActiveZones").as_deref(), None | Some("All"))
        {
            return None;
        }
        let with = p.take("ReplaceWith")?;
        if !p.exhausted() {
            return None;
        }
        let (api, mut cp) = self.svars.get(with.trim()).and_then(|s| Params::parse(s))?;
        cp.drop_prose();
        let choice_mod = match api.as_str() {
            "ChooseType" => {
                if !matches!(cp.take("Defined").as_deref(), None | Some("You")) {
                    return None;
                }
                match cp.take("Type").as_deref() {
                    Some("Creature") => "EnterModifier::ChooseSubtype",
                    Some("Basic Land") => "EnterModifier::ChooseBasicLandType",
                    _ => return None,
                }
            }
            "ChoosePlayer" => {
                if cp.has("Secretly")
                    || cp.has("RememberChosen")
                    || cp.has("Random")
                    || !matches!(cp.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                cp.take("ChoiceTitle");
                match cp.take("Choices").as_deref() {
                    Some("Player.Opponent" | "Opponent") => "EnterModifier::ChooseOpponent",
                    Some("Player") => "EnterModifier::ChooseAnyPlayer",
                    _ => return None,
                }
            }
            _ => return None,
        };
        let next_sub = cp.take("SubAbility")?;
        if !cp.exhausted() {
            return None;
        }
        // Push the choice modifier first so `self_etb_counter_chain` sees
        // `has_chosen_source_player()` / `ChosenType` when resolving `CounterNum$ X`.
        self.body.enter_modifiers.push(choice_mod.to_string());
        let counters = self.self_etb_counter_chain(&next_sub)?;
        self.body.enter_modifiers.extend(counters);
        Some(())
    }

    /// `R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield |
    /// DayTime$ Neither | ReplaceWith$ <DB$ DayTime | Value$ Day> |
    /// ReplacementResult$ Updated` -> `EnterModifier::BecomeDayIfNeither`
    /// (*Brimstone Vandal*, *The Celestus*, *Celestus Sanctifier*, etc.; `CR 726.2`).
    fn wave16c_self_etb_daytime_neither(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Card.Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("DayTime").as_deref() != Some("Neither")
            || !matches!(
                p.take("ReplacementResult").as_deref(),
                None | Some("Updated")
            )
            || !matches!(p.take("Origin").as_deref(), None | Some("Any" | "All"))
            || !matches!(p.take("ActiveZones").as_deref(), None | Some("All"))
        {
            return None;
        }
        let with = p.take("ReplaceWith")?;
        if !p.exhausted() {
            return None;
        }
        let (api, mut dp) = self.svars.get(with.trim()).and_then(|s| Params::parse(s))?;
        dp.drop_prose();
        if api != "DayTime" || dp.take("Value").as_deref() != Some("Day") || !dp.exhausted() {
            return None;
        }
        self.body
            .enter_modifiers
            .push("EnterModifier::BecomeDayIfNeither".to_string());
        Some(())
    }

    /// Wave 16c `DB$ Effect` delayed trigger extensions:
    /// - *Arcbond* (`DamageDoneOnce | ValidTarget$ Card.IsRemembered` ->
    ///   `DamageAll | DamageSource$ Remembered`)
    /// - *Basri Ket* `[-2]` (`AttackersDeclared | ValidAttackers$ <filter>` +
    ///   `TriggerObjectsAttackers$Valid <filter>`)
    /// - *Electric Seaweed* / *Massacre Girl* (`RememberObjects$ Self` +
    ///   `!IsRemembered` + `DamageSource$ EffectSource`)
    /// - *Garruk, Curse Breaker* `[-4]` (`AttackersDeclaredOneTarget` ->
    ///   `Pump | Defined$ TriggeredAttackers`)
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave16c_effect_triggers(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        let snap = p.clone();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        let res = self.wave16c_effect_triggers_inner(p, target, chain);
        if res.is_none() {
            *p = snap;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
        }
        res
    }

    #[allow(clippy::too_many_lines)]
    fn wave16c_effect_triggers_inner(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if p.has("Planeswalker")
            || p.has("Abilities")
            || p.has("StaticAbilities")
            || p.has("ReplacementEffects")
        {
            return None;
        }
        let dur_raw = p.take("Duration");
        let (repeat_dur, oneshot_dur) = match dur_raw.as_deref() {
            None | Some("UntilEndOfTurn" | "EndOfTurn" | "End of Turn") => (
                "DelayedDuration::ThisTurn",
                Some("DelayedDuration::OneShotThisTurn"),
            ),
            Some("UntilEndOfCombat") => ("DelayedDuration::ThisCombat", None),
            Some("UntilYourNextTurn") => ("DelayedDuration::UntilYourNextTurn", None),
            Some("Permanent") => (
                "DelayedDuration::Indefinitely",
                Some("DelayedDuration::OneShot"),
            ),
            Some(_) => return None,
        };
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if !matches!(ends.as_deref(), None | Some("Battlefield")) {
            return None;
        }
        p.take("IsCurse");
        p.take("Name");
        if let Some(ns) = p.take("NonStackingEffect")
            && ns != "True"
        {
            return None;
        }
        let raw_rem = p.peek("RememberObjects").map(str::to_string);
        let _subject = self.effect_remember_subject(p, target)?;
        let base_watch = match raw_rem.as_deref() {
            None => "TriggerRemember::None",
            Some(rem) => self.delayed_remember(Some(rem), target, chain, true)?,
        };
        if ends.is_some() && base_watch == "TriggerRemember::None" {
            return None;
        }
        let triggers = p.take("Triggers")?;
        if triggers.contains(',')
            || !p
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
        {
            return None;
        }
        let trig_body = self.svars.get(triggers.trim())?.clone();
        let (mode, mut tp) = Params::parse(&trig_body)?;
        tp.drop_prose();
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

        // 1. Arcbond: watched `DamageDoneOnce | ValidTarget$ Card.IsRemembered`
        //    -> `DamageAll | DamageSource$ Remembered`.
        if mode == "DamageDoneOnce"
            && base_watch != "TriggerRemember::None"
            && matches!(
                tp.peek("ValidTarget"),
                Some("Card.IsRemembered" | "Creature.IsRemembered")
            )
            && !tp.has("ValidSource")
            && !tp.has("CombatDamage")
        {
            tp.take("ValidTarget");
            let cond = self.delayed_optional_condition(&mut tp)?;
            let exec_name = tp.take("Execute")?;
            if !tp.exhausted() {
                return None;
            }
            let exec_body = self.svars.get(exec_name.trim())?.clone();
            let eff = self.wave16c_arcbond_damage_all(&exec_body, optional)?;
            return Some(vec![format!(
                "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
                 trigger: &Trigger::DealtDamage(&Filter::This), watch: {base_watch}, \
                 duration: {delayed_dur} }}, condition: {cond}, targets: None, \
                 remember: TriggerRemember::None, effects: &[{eff}] }}"
            )]);
        }

        // 2. Basri Ket [-2]: `AttackersDeclared | ValidAttackers$ <vf>` with
        //    `TokenAmount$ <var>` where `<var>: TriggerObjectsAttackers$Valid <vf>`.
        if mode == "AttackersDeclared"
            && base_watch == "TriggerRemember::None"
            && !tp.has("AttackingPlayer")
            && !tp.has("AttackedTarget")
            && !tp.has("ValidAttackersAmount")
            && let Some(valid_attackers) = tp.peek("ValidAttackers").map(str::to_string)
        {
            let matching_svar = self
                .svars
                .iter()
                .find(|(_, v)| {
                    v.trim()
                        .strip_prefix("TriggerObjectsAttackers$Valid ")
                        .is_some_and(|rest| rest.trim() == valid_attackers.trim())
                })
                .map(|(k, _)| k.clone());
            if let Some(var_name) = matching_svar {
                let trigger_expr = self.trigger_expr_in_zone(&mut tp, &mode, true)?;
                let cond = self.delayed_optional_condition(&mut tp)?;
                let exec_name = tp.take("Execute")?;
                if !tp.exhausted() {
                    return None;
                }
                let exec_body = self.svars.get(exec_name.trim())?.clone();
                let (api, mut ep) = Params::parse(&exec_body)?;
                ep.drop_prose();
                if api == "Token" && ep.peek("TokenAmount") == Some(var_name.as_str()) {
                    let eff = self.wave16c_event_amount_token(&mut ep, optional)?;
                    return Some(vec![format!(
                        "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
                         trigger: &{trigger_expr}, watch: TriggerRemember::None, \
                         duration: {delayed_dur} }}, condition: {cond}, targets: None, \
                         remember: TriggerRemember::None, effects: &[{eff}] }}"
                    )]);
                }
            }
        }

        // 3. Garruk, Curse Breaker [-4]: `AttackersDeclaredOneTarget` with
        //    `Pump | Defined$ TriggeredAttackers`.
        if mode == "AttackersDeclaredOneTarget" && base_watch == "TriggerRemember::None" {
            let exact_one = tp.peek("ValidAttackersAmount") == Some("EQ1");
            let trigger_expr = self.trigger_expr_in_zone(&mut tp, &mode, true)?;
            let cond = self.delayed_optional_condition(&mut tp)?;
            let exec_name = tp.take("Execute")?;
            if !tp.exhausted() {
                return None;
            }
            let exec_body = self.svars.get(exec_name.trim())?.clone();
            let mut inner = Chain::default();
            self.wave25c_attackers_at_target_chain(&exec_body, exact_one, &mut inner)?;
            if inner.target.is_some() || inner.effects.is_empty() {
                return None;
            }
            let joined = inner.effects.join(", ");
            let effs = if optional {
                format!("Effect::MayDo {{ effects: &[{joined}] }}")
            } else {
                joined
            };
            return Some(vec![format!(
                "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
                 trigger: &{trigger_expr}, watch: TriggerRemember::None, \
                 duration: {delayed_dur} }}, condition: {cond}, targets: None, \
                 remember: TriggerRemember::None, effects: &[{effs}] }}"
            )]);
        }

        // 4. Electric Seaweed / Massacre Girl: `RememberObjects$ Self` with
        //    `!IsRemembered` on `ValidCard$` and optional `DamageSource$ EffectSource`.
        if base_watch == "TriggerRemember::Source" && !self.on_a_spell {
            let mut rewrote_not_rem = false;
            for (k, v) in &mut tp.entries {
                if matches!(k.as_str(), "ValidCard" | "ValidSource" | "ValidTarget")
                    && v.contains("!IsRemembered")
                {
                    *v = v.replace("!IsRemembered", "Other");
                    rewrote_not_rem = true;
                }
            }
            if rewrote_not_rem {
                let trigger_expr = self.trigger_expr_in_zone(&mut tp, &mode, true)?;
                let cond = self.delayed_optional_condition(&mut tp)?;
                let exec_name = tp.take("Execute")?;
                if !tp.exhausted() {
                    return None;
                }
                let exec_raw = self.svars.get(exec_name.trim())?.clone();
                let cleaned = exec_raw
                    .split('|')
                    .map(str::trim)
                    .filter(|part| *part != "DamageSource$ EffectSource")
                    .collect::<Vec<_>>()
                    .join(" | ");
                let (effects, targets) =
                    self.parse_delayed_body_str(&cleaned, Some(mode), optional)?;
                let targets_expr =
                    targets.map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
                return Some(vec![format!(
                    "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::OnTrigger {{ \
                     trigger: &{trigger_expr}, watch: TriggerRemember::Source, \
                     duration: {delayed_dur} }}, condition: {cond}, targets: {targets_expr}, \
                     remember: TriggerRemember::None, effects: &[{effects}] }}"
                )]);
            }
        }

        None
    }

    fn wave16c_arcbond_damage_all(&mut self, exec_body: &str, optional: bool) -> Option<String> {
        let (api, mut ep) = Params::parse(exec_body)?;
        ep.drop_prose();
        if api != "DamageAll"
            || !matches!(
                ep.take("DamageSource").as_deref(),
                Some("Remembered" | "RememberedLKI" | "TriggeredTarget" | "TriggeredTargetLKICopy")
            )
        {
            return None;
        }
        let num_raw = ep.take("NumDmg")?;
        let amount = if let Ok(n) = num_raw.trim().parse::<u32>() {
            format!("Amount::Fixed({n})")
        } else if self
            .svars
            .get(num_raw.trim())
            .is_some_and(|s| s.trim() == "TriggerCount$DamageAmount")
        {
            "Amount::EventAmount".to_string()
        } else {
            return None;
        };
        let mut exclude_event_object = false;
        let filter_expr = match ep.take("ValidCards") {
            None => "&Filter::NONE".to_string(),
            Some(vc) => {
                let mut cleaned = vc.clone();
                if cleaned.contains("!IsRemembered") {
                    cleaned = cleaned
                        .replace("+!IsRemembered", "")
                        .replace(".!IsRemembered", "");
                    exclude_event_object = true;
                }
                if cleaned.contains("IsRemembered") {
                    return None;
                }
                let expr = self.filter_expr(&cleaned)?;
                let name = self.body.filter_static("DAMAGE_EACH", &expr);
                format!("&{name}")
            }
        };
        let players_expr = match ep.take("ValidPlayers").as_deref() {
            None => "None",
            Some("Player") => "Some(PlayerRel::EachPlayer)",
            Some("Opponent" | "Player.Opponent") => "Some(PlayerRel::EachOpponent)",
            Some("You") => "Some(PlayerRel::You)",
            Some(_) => return None,
        };
        if filter_expr == "&Filter::NONE" && players_expr == "None" {
            return None;
        }
        if !ep.exhausted() {
            return None;
        }
        let core = format!(
            "Effect::EventObjectDealsDamageEach {{ amount: {amount}, filter: {filter_expr}, \
             players: {players_expr}, exclude_event_object: {exclude_event_object} }}"
        );
        Some(if optional {
            format!("Effect::MayDo {{ effects: &[{core}] }}")
        } else {
            core
        })
    }

    fn wave16c_event_amount_token(&mut self, ep: &mut Params, optional: bool) -> Option<String> {
        ep.take("TokenAmount");
        if !matches!(ep.take("TokenOwner").as_deref(), None | Some("You")) {
            return None;
        }
        let tapped = match ep.take("TokenTapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let attacking = match ep.take("TokenAttacking").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let script = ep.take("TokenScript")?;
        if !ep.exhausted() {
            return None;
        }
        let body = self.tokens?.body_fixed(script.trim(), self.cats)?;
        let token = &body.constant;
        let core = match (tapped, attacking) {
            (true, true) => format!(
                "Effect::CreateTokenAttacking {{ token: &generated_tokens::{token}, \
                 amount: Amount::EventAmount, sacrifice_at_next_end_step: false }}"
            ),
            (true, false) => format!(
                "Effect::CreateTokenModified {{ token: &generated_tokens::{token}, \
                 amount: Amount::EventAmount, tapped: true, haste: false }}"
            ),
            (false, false) => format!(
                "Effect::CreateTokenN {{ token: &generated_tokens::{token}, \
                 amount: Amount::EventAmount }}"
            ),
            (false, true) => return None,
        };
        Some(if optional {
            format!("Effect::MayDo {{ effects: &[{core}] }}")
        } else {
            core
        })
    }

    /// Player-targeted `Mode$ CantBlock` / `Mode$ CantAttack` with
    /// `ValidCard$ <filter>.RememberedPlayerCtrl` (*Ember Gale*, *Headliner Scarlett*).
    pub(super) fn wave16c_player_effect_cant_block_or_attack(
        &mut self,
        mode: &str,
        valid: &str,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        let EffectSubject::Player(who) = subject else {
            return None;
        };
        if !valid.contains("RememberedPlayerCtrl") {
            return None;
        }
        let stripped = valid
            .replace("+RememberedPlayerCtrl", "")
            .replace(".RememberedPlayerCtrl", "");
        if stripped.contains("Remembered") || stripped.contains("EffectSource") {
            return None;
        }
        let base_expr = self.filter_expr(&stripped)?;
        let combined = format!("Filter::And(&[{base_expr}, Filter::ControlledBy({who})])");
        let name = self.body.filter_static("EFFECT", &combined);
        let modifier = match mode {
            "CantBlock" => "Modifier::AddKeyword(KeywordSet::CANT_BLOCK)",
            "CantAttack" => "Modifier::CantAttack",
            _ => return None,
        };
        Some(vec![format!(
            "Effect::AddGlobalTempModifier {{ filter: &{name}, modifier: {modifier}, \
             duration: {duration} }}"
        )])
    }

    /// Wave 16c `ChoosePlayer` resolution chain extensions:
    /// - `Random$ True` -> `Effect::ChoosePlayerAtRandom { among }` (*Ruhan*,
    ///   *Knight Rampager*, *Raving Dead*; `CR 101.4`)
    /// - Multi-recipient `.Chosen,You` / `You,.Chosen`, chained second `ChoosePlayer`,
    ///   `UntapAll` across `YouCtrl` + `ChosenCtrl`, `GainControl | NewController$ Player.Chosen`,
    ///   and `ChosenCtrl` count/hand-difference expressions (*Benevolent Offering*,
    ///   *Intellectual Offering*, *Sandstone Oracle*, *Slithermuse*, *Discerning Financier*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave16c_choose_player_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let player_target_in_use = chain.target.as_deref().is_some_and(|t| {
            matches!(
                t,
                "TargetSpec::AnyPlayer"
                    | "TargetSpec::AnyOpponent"
                    | "TargetSpec::PlayerWhoAttackedThisTurn"
                    | "TargetSpec::Player(PlayerRel::Chosen)"
            )
        });
        if targets_here
            || player_target_in_use
            || p.has("ValidTgts")
            || p.has("Secretly")
            || p.has("RememberChosen")
            || !matches!(p.take("Defined").as_deref(), None | Some("You"))
        {
            return None;
        }
        let random = match p.take("Random").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let among = Self::wave16c_player_choices(p.take("Choices").as_deref())?;
        p.take("ChoiceTitle");
        if !p.exhausted() {
            return None;
        }
        let mut cursor = sub.take()?;
        let head = if random {
            format!("Effect::ChoosePlayerAtRandom {{ among: {among} }}")
        } else {
            format!("Effect::ChoosePlayerFrom {{ among: {among} }}")
        };
        let mut out = vec![head];
        let mut consumed = false;
        let mut steps = 0usize;
        loop {
            steps += 1;
            if steps > 8 {
                return None;
            }
            let body = self.svars.get(cursor.trim())?.clone();
            let (s_api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if s_api == "Cleanup" {
                let cp = sp.take("ClearChosenPlayer");
                let rem = sp.take("ClearRemembered");
                let next = sp.take("SubAbility");
                if cp.as_deref() != Some("True")
                    || rem.as_deref().is_some_and(|v| v != "True")
                    || !sp.exhausted()
                {
                    return None;
                }
                *sub = next;
                break;
            }
            let next_sub = sp.take("SubAbility");
            if s_api == "ChoosePlayer" {
                if !consumed
                    || sp.has("ValidTgts")
                    || sp.has("Secretly")
                    || sp.has("RememberChosen")
                    || !matches!(sp.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                let r2 = match sp.take("Random").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let among2 = Self::wave16c_player_choices(sp.take("Choices").as_deref())?;
                sp.take("ChoiceTitle");
                if !sp.exhausted() {
                    return None;
                }
                out.push(if r2 {
                    format!("Effect::ChoosePlayerAtRandom {{ among: {among2} }}")
                } else {
                    format!("Effect::ChoosePlayerFrom {{ among: {among2} }}")
                });
                consumed = false;
            } else if s_api == "Token"
                && matches!(
                    sp.peek("TokenOwner"),
                    Some(".Chosen,You" | "You,.Chosen" | "ChosenPlayer,You" | "You,ChosenPlayer")
                )
            {
                sp.take("TokenOwner");
                let mut sp_you = sp.clone();
                sp_you.entries.push(("TokenOwner".into(), "You".into()));
                let mut sp_chosen = sp;
                sp_chosen
                    .entries
                    .push(("TokenOwner".into(), "Targeted".into()));
                let eff_chosen = self.effect_of(
                    "Token",
                    &mut sp_chosen,
                    Some("TargetSpec::Player(PlayerRel::Chosen)"),
                    false,
                    false,
                )?;
                let eff_you = self.effect_of("Token", &mut sp_you, None, false, false)?;
                if !sp_chosen.exhausted() || !sp_you.exhausted() {
                    return None;
                }
                out.extend(eff_you);
                out.extend(eff_chosen);
                consumed = true;
            } else if s_api == "Draw"
                && matches!(
                    sp.peek("Defined"),
                    Some(".Chosen,You" | "You,.Chosen" | "ChosenPlayer,You" | "You,ChosenPlayer")
                )
            {
                sp.take("Defined");
                let mut sp_you = sp.clone();
                sp_you.entries.push(("Defined".into(), "You".into()));
                let mut sp_chosen = sp;
                sp_chosen
                    .entries
                    .push(("Defined".into(), "Targeted".into()));
                let eff_you = self.effect_of("Draw", &mut sp_you, None, false, false)?;
                let eff_chosen = self.effect_of(
                    "Draw",
                    &mut sp_chosen,
                    Some("TargetSpec::Player(PlayerRel::Chosen)"),
                    false,
                    false,
                )?;
                if !sp_you.exhausted() || !sp_chosen.exhausted() {
                    return None;
                }
                out.extend(eff_you);
                out.extend(eff_chosen);
                consumed = true;
            } else if s_api == "UntapAll"
                && sp
                    .peek("ValidCards")
                    .is_some_and(|v| v.contains("ChosenCtrl"))
            {
                let vc = sp.take("ValidCards")?;
                if !sp.exhausted() {
                    return None;
                }
                let mut alts = Vec::new();
                for part in vc.split(',').map(str::trim) {
                    if let Some(stripped) = part
                        .strip_suffix("+ChosenCtrl")
                        .or_else(|| part.strip_suffix(".ChosenCtrl"))
                    {
                        let base = self.filter_expr(stripped)?;
                        alts.push(format!(
                            "Filter::And(&[{base}, Filter::ControlledBy(PlayerRel::Chosen)])"
                        ));
                    } else {
                        alts.push(self.filter_expr(part)?);
                    }
                }
                let combined = if alts.len() == 1 {
                    alts.pop()?
                } else {
                    format!("Filter::Or(&[{}])", alts.join(", "))
                };
                let f_name = self.body.filter_static("UNTAP", &combined);
                out.push(format!("Effect::UntapAll {{ filter: &{f_name} }}"));
                consumed = true;
            } else if s_api == "GainControl"
                && chain.target.is_none()
                && sp.has("ValidTgts")
                && matches!(
                    sp.peek("NewController"),
                    Some("Player.Chosen" | "ChosenPlayer")
                )
            {
                sp.take("NewController");
                let valid_tgts = sp.take("ValidTgts")?;
                sp.take("TgtPrompt");
                sp.take("ValidTgtsDesc");
                if !sp.exhausted() {
                    return None;
                }
                let expr = self.filter_expr(&valid_tgts)?;
                let f_name = self.body.filter_static("TARGET", &expr);
                let tgt = format!("TargetSpec::Object(&{f_name})");
                chain.target = Some(tgt.clone());
                out.push(format!(
                    "Effect::GainControlEx {{ target: {tgt}, to: PlayerRel::Chosen, \
                     duration: Duration::Indefinitely, untap: false, haste_until_eot: false }}"
                ));
                consumed = true;
            } else {
                match self.wave16c_chosen_player_step(&s_api, &mut sp)? {
                    ChosenStep::Handled(effs) => {
                        out.extend(effs);
                        consumed = true;
                    }
                    ChosenStep::Pass => {
                        // Non-chosen trailing step (e.g. `DB$ Draw` for `You` after `GainControl`,
                        // or `DB$ GainLife | Defined$ You` before `Defined$ ChosenPlayer`).
                        if sp.has("ValidTgts")
                            || sp.entries.iter().any(|(_, v)| {
                                v.contains("ChosenPlayer")
                                    || v.contains("Player.Chosen")
                                    || v.contains("ChosenCtrl")
                            })
                        {
                            return None;
                        }
                        let step_effs = self.effect_of(&s_api, &mut sp, None, false, false)?;
                        if !sp.exhausted() || step_effs.is_empty() {
                            return None;
                        }
                        out.extend(step_effs);
                    }
                }
            }
            match next_sub {
                Some(next) => cursor = next,
                None => break,
            }
        }
        consumed.then_some(out)
    }

    fn wave16c_player_choices(raw: Option<&str>) -> Option<&'static str> {
        Some(match raw {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("Player.Other") => "PlayerRel::EachOtherPlayer",
            Some("Player.withMostLife") => "PlayerRel::MostLifeOrTied",
            Some("Player.withLowestLife") => "PlayerRel::LeastLifeOrTied",
            _ => return None,
        })
    }

    /// Lowers a single step inside `wave16c_choose_player_chain` that references
    /// `ChosenPlayer`, `Player.Chosen`, or `ChosenCtrl`.
    #[allow(clippy::too_many_lines)]
    fn wave16c_chosen_player_step(&mut self, s_api: &str, sp: &mut Params) -> Option<ChosenStep> {
        // Sandstone Oracle / Slithermuse: `Draw | NumCards$ X` where
        // `X = Count$ValidHand Card.ChosenCtrl/Minus.Y` and `Y = Count$ValidHand Card.YouOwn`.
        if s_api == "Draw"
            && matches!(sp.peek("Defined"), None | Some("You"))
            && let Some(num_var) = sp.peek("NumCards").map(str::to_string)
            && let Some(def) = self.svars.get(num_var.trim())
            && let Some(y_var) = def
                .trim()
                .strip_prefix("Count$ValidHand Card.ChosenCtrl/Minus.")
            && self.svars.get(y_var.trim()).is_some_and(|y| {
                matches!(
                    y.trim(),
                    "Count$ValidHand Card.YouOwn" | "Count$ValidHand Card.YouCtrl"
                )
            })
        {
            sp.take("Defined");
            sp.take("NumCards");
            // Optional redundant `ConditionCheckSVar$ X | ConditionSVarCompare$ GE1` or
            // `ConditionPresent$ Card.ChosenCtrl | ConditionCompare$ GTY | ConditionZone$ Hand`
            // (since `SaturatingSub` already draws 0 when `Chosen <= You`).
            if sp.peek("ConditionCheckSVar").map(str::trim) == Some(num_var.trim())
                && matches!(sp.peek("ConditionSVarCompare"), None | Some("GE1" | "GT0"))
            {
                sp.take("ConditionCheckSVar");
                sp.take("ConditionSVarCompare");
            } else if sp.peek("ConditionPresent") == Some("Card.ChosenCtrl")
                && sp.peek("ConditionZone") == Some("Hand")
                && sp.peek("ConditionCompare") == Some(&format!("GT{}", y_var.trim()))
            {
                sp.take("ConditionPresent");
                sp.take("ConditionZone");
                sp.take("ConditionCompare");
            }
            if !sp.exhausted() {
                return None;
            }
            let diff = "Amount::Difference { \
                        left: &Amount::HandSize(PlayerRel::Chosen), \
                        right: &Amount::HandSize(PlayerRel::You) }";
            return Some(ChosenStep::Handled(vec![format!(
                "Effect::IfCondition {{ condition: Condition::AmountAtLeast({diff}, 1), \
                 then: &[Effect::DrawCards {{ amount: {diff} }}], otherwise: &[] }}"
            )]));
        }

        // Benevolent Offering: `GainLife | Defined$ ChosenPlayer | LifeAmount$ Y`
        // where `Y = Count$Valid <filter>.ChosenCtrl[/Twice]`.
        if s_api == "GainLife"
            && matches!(sp.peek("Defined"), Some("ChosenPlayer" | "Player.Chosen"))
            && let Some(amt_var) = sp.peek("LifeAmount").map(str::to_string)
            && let Some(amt_expr) = self.wave16c_chosen_ctrl_count_amount(amt_var.trim())
        {
            sp.take("Defined");
            sp.take("LifeAmount");
            if !sp.exhausted() {
                return None;
            }
            return Some(ChosenStep::Handled(vec![format!(
                "Effect::GainLifeFor {{ who: PlayerRel::Chosen, amount: {amt_expr} }}"
            )]));
        }

        // Ruhan / Knight Rampager / Raving Dead: `Effect | StaticAbilities$ AttackChosen`
        // (`MustAttack$ ChosenPlayer` with `UntilEndOfTurn` or `UntilEndOfCombat`).
        if s_api == "Effect"
            && sp
                .peek("StaticAbilities")
                .and_then(|s| self.svars.get(s.trim()))
                .is_some_and(|b| b.contains("ChosenPlayer"))
        {
            let effs = self.effect_of(
                "Effect",
                sp,
                Some("TargetSpec::Player(PlayerRel::Chosen)"),
                false,
                false,
            )?;
            if !sp.exhausted() || effs.is_empty() {
                return None;
            }
            return Some(ChosenStep::Handled(effs));
        }

        let uses_chosen = sp.entries.iter().any(|(_, v)| {
            v == "ChosenPlayer"
                || v == "Player.Chosen"
                || v.contains("ChosenPlayer")
                || v.contains("ChosenCtrl")
        });
        if !uses_chosen {
            return Some(ChosenStep::Pass);
        }
        if sp.has("ValidTgts")
            || sp.has("UnlessCost")
            || sp.has("AtEOT")
            || sp.has("ReplaceDyingDefined")
            || (s_api == "DealDamage"
                && self
                    .oracle
                    .to_ascii_lowercase()
                    .contains("or a planeswalker that player controls"))
            || sp.entries.iter().any(|(k, v)| {
                k.starts_with("Condition") || v.contains("Targeted") || v.contains("ParentTarget")
            })
        {
            return None;
        }
        let mut rewrote = false;
        for (k, v) in &mut sp.entries {
            if matches!(k.as_str(), "Defined" | "DefinedPlayer" | "TokenOwner")
                && matches!(v.as_str(), "ChosenPlayer" | "Player.Chosen")
            {
                *v = "Targeted".to_string();
                rewrote = true;
            }
        }
        if !rewrote
            || sp.entries.iter().any(|(_, v)| {
                v.contains("ChosenPlayer")
                    || v.contains("Player.Chosen")
                    || v.contains("ChosenCtrl")
            })
        {
            return None;
        }
        let step_effs = self.effect_of(
            s_api,
            sp,
            Some("TargetSpec::Player(PlayerRel::Chosen)"),
            false,
            false,
        )?;
        if !sp.exhausted() || step_effs.is_empty() {
            return None;
        }
        Some(ChosenStep::Handled(step_effs))
    }

    fn wave16c_chosen_ctrl_count_amount(&mut self, var: &str) -> Option<String> {
        let def = self.svars.get(var)?.trim().to_string();
        let (head, suffix) = def
            .split_once('/')
            .map_or((def.as_str(), None), |(h, s)| (h.trim(), Some(s.trim())));
        let valid = head.strip_prefix("Count$Valid ")?;
        let stripped = valid
            .trim()
            .strip_suffix("+ChosenCtrl")
            .or_else(|| valid.trim().strip_suffix(".ChosenCtrl"))?;
        let base_filter = self.filter_expr(stripped)?;
        let combined =
            format!("Filter::And(&[{base_filter}, Filter::ControlledBy(PlayerRel::Chosen)])");
        let f_name = self.body.filter_static("COUNT", &combined);
        let count = format!("Amount::CountOf {{ filter: &{f_name}, zone: ZoneSel::Battlefield }}");
        match suffix {
            None => Some(count),
            Some("Twice") => Some(format!("Amount::Times {{ base: &{count}, factor: 2 }}")),
            Some(rest) => {
                let n: u32 = rest.strip_prefix("Times.")?.parse().ok()?;
                (n >= 2).then(|| format!("Amount::Times {{ base: &{count}, factor: {n} }}"))
            }
        }
    }

    /// Wave 16c `ChooseNumber` variable energy / mana / life payment chains
    /// (`Die Young`, `Harnessed Lightning`, `Aether Spike`, `Aether Refinery`,
    /// `Wrath of the Skies`, `Rampaging Aetherhood`, `Pia Nalaar`, `Localized
    /// Destruction`, `Territorial Aetherkite`, `Well of Lost Dreams`, `Plunge
    /// into Darkness`, `Karn, Living Legacy`, `Vault 112: Sadistic Simulation`).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave16c_choose_number_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        if targets_here
            || p.has("ValidTgts")
            || p.has("Secretly")
            || p.has("Random")
            || !p.has("ListTitle")
        {
            return None;
        }
        let chosen_var = self
            .svars
            .iter()
            .find(|(_, v)| v.trim() == "Count$ChosenNumber")
            .map(|(k, _)| k.clone())?;
        let first_sub = sub.as_deref()?.trim().to_string();
        // Only activate when a downstream step actually carries Forge's
        // `UnlessCost$ ... | UnlessPayer$ You | UnlessSwitched$ True` payment!
        if !self.sub_chain_has_switched_unless(&first_sub) {
            return None;
        }

        let snap_p = p.clone();
        let snap_sub = sub.clone();
        let saved_effects_len = chain.effects.len();
        let saved_target = chain.target.clone();
        let saved_second = chain.second_target.clone();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();

        let res = self.wave16c_choose_number_chain_inner(p, sub, &chosen_var, chain);
        if res.is_none() {
            *p = snap_p;
            *sub = snap_sub;
            chain.effects.truncate(saved_effects_len);
            chain.target = saved_target;
            chain.second_target = saved_second;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
        }
        res
    }

    fn sub_chain_has_switched_unless(&self, start: &str) -> bool {
        let mut cur = Some(start.to_string());
        for _ in 0..8 {
            let Some(name) = cur else {
                return false;
            };
            let Some(body) = self.svars.get(name.trim()) else {
                return false;
            };
            if body.contains("UnlessCost$") && body.contains("UnlessSwitched$ True") {
                return true;
            }
            cur = Params::parse(body).and_then(|(_, mut sp)| sp.take("SubAbility"));
        }
        false
    }

    #[allow(clippy::too_many_lines)]
    fn wave16c_choose_number_chain_inner(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chosen_var: &str,
        chain: &mut Chain,
    ) -> Option<()> {
        if !matches!(p.take("Defined").as_deref(), None | Some("You")) {
            return None;
        }
        if let Some(any_num) = p.take("ChooseAnyNumber")
            && any_num != "True"
        {
            return None;
        }
        if let Some(notify) = p.take("Notify")
            && notify != "True"
        {
            return None;
        }
        let title = p.take("ListTitle")?;
        let lower = title.to_ascii_lowercase();
        let (prompt, pay_cost, expected_unless) = if lower.contains("energy to pay") {
            (
                "NumberChoiceKind::EnergyPayment",
                "Some(VariablePayKind::Energy)",
                format!("PayEnergy<{chosen_var}>"),
            )
        } else if lower.contains("mana to pay") {
            (
                "NumberChoiceKind::ManaPayment",
                "Some(VariablePayKind::GenericMana)",
                chosen_var.to_string(),
            )
        } else if lower.contains("life to pay") {
            (
                "NumberChoiceKind::Choice",
                "Some(VariablePayKind::Life)",
                format!("PayLife<{chosen_var}>"),
            )
        } else {
            return None;
        };
        let min = match p.take("Min") {
            None => "Amount::Fixed(0)".to_string(),
            Some(raw) => self.amount_or_count_ext(&raw)?,
        };
        let max = match p.take("Max") {
            None => "Amount::Fixed(u32::MAX)".to_string(),
            Some(raw) => self.amount_or_count_ext(&raw).or_else(|| {
                (pay_cost == "Some(VariablePayKind::Life)"
                    && (raw.trim() == "Count$YourLifeTotal/LimitMin.0"
                        || self
                            .svars
                            .get(raw.trim())
                            .is_some_and(|v| v.trim() == "Count$YourLifeTotal/LimitMin.0")))
                .then(|| "Amount::LifeTotal(PlayerRel::You)".to_string())
            })?,
        };
        if !p.exhausted() {
            return None;
        }

        // Walk the follow-up chain, locating the single switched `UnlessCost$`
        // payment step and splitting any `"If you do"` (`ConditionCheckSVar$ <var>`)
        // gated prefix from ungated trailing steps (e.g. *Localized Destruction*'s
        // `DestroyAll`).
        let start = sub.take()?;
        let mut svars_override = self.svars.clone();
        let mut saw_payment = false;
        let mut gated_svar_names = Vec::new();
        let mut cur = Some(start.clone());
        let mut effective_start = Some(start);
        for _ in 0..8 {
            let Some(name) = cur.take() else {
                break;
            };
            let key = name.trim().to_string();
            let body = svars_override.get(&key)?.clone();
            let (s_api, mut sp) = Params::parse(&body)?;
            let next_sub = sp.peek("SubAbility").map(|s| s.trim().to_string());

            // Check `UnlessCost$` on non-Counter-tax lines.
            let is_spell_counter_tax = s_api == "Counter"
                && sp.peek("TargetType") == Some("Spell")
                && !sp.has("UnlessSwitched")
                && !sp.has("UnlessPayer");
            if sp.has("UnlessCost") && !is_spell_counter_tax {
                if saw_payment {
                    return None;
                }
                let raw_unless = sp.take("UnlessCost")?;
                let core_unless = raw_unless
                    .trim()
                    .strip_prefix("Mandatory ")
                    .unwrap_or(raw_unless.trim());
                if core_unless != expected_unless
                    || sp.take("UnlessPayer").as_deref() != Some("You")
                    || sp.take("UnlessSwitched").as_deref() != Some("True")
                {
                    return None;
                }
                saw_payment = true;
                if s_api == "Nothing" {
                    sp.drop_prose();
                    let pass_sub = sp.take("SubAbility");
                    if !sp.exhausted() {
                        return None;
                    }
                    if effective_start.as_deref().map(str::trim) == Some(key.as_str()) {
                        effective_start.clone_from(&pass_sub);
                    }
                    cur = pass_sub;
                    continue;
                }
            }

            let mut step_gated = false;
            if sp.peek("ConditionCheckSVar").map(str::trim) == Some(chosen_var)
                && matches!(sp.peek("ConditionSVarCompare"), None | Some("GE1" | "GT0"))
            {
                sp.take("ConditionCheckSVar");
                sp.take("ConditionSVarCompare");
                step_gated = true;
            }

            // Rebuild cleaned SVar line if modified.
            let rebuilt = std::iter::once(format!("DB$ {s_api}"))
                .chain(sp.entries.iter().map(|(k, v)| format!("{k}$ {v}")))
                .collect::<Vec<_>>()
                .join(" | ");
            svars_override.insert(key.clone(), rebuilt);
            if step_gated {
                gated_svar_names.push(key);
            }
            cur = next_sub;
        }
        if !saw_payment {
            return None;
        }

        let first_eff_svar = effective_start?;
        let followup_effects = self.wave16c_lower_choose_num_followup(
            &first_eff_svar,
            &svars_override,
            &gated_svar_names,
            chain,
        )?;

        chain.effects.push(format!(
            "Effect::ChooseNumberEx {{ chooser: PlayerRel::You, min: {min}, max: {max}, \
             prompt: {prompt}, pay_cost: {pay_cost}, then: &[{}] }}",
            followup_effects.join(", ")
        ));
        Some(())
    }

    fn wave16c_lower_choose_num_followup(
        &mut self,
        start: &str,
        svars_override: &std::collections::BTreeMap<String, String>,
        gated_svar_names: &[String],
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        // Split chain if an early step is gated by `ConditionCheckSVar$ X` (`>= 1`)
        // and a later sibling is NOT gated (e.g. *Localized Destruction*:
        // `DBPumpAll` gated -> `DBDestroyAll` unconditional!).
        if !gated_svar_names.is_empty() {
            let mut gated_chain_svars = svars_override.clone();
            // Walk from `start`: if `start` is gated, find the first ungated SVar
            // reachable via `SubAbility$` (excluding inner `Execute$`/`Play$` children).
            let mut order = Vec::new();
            let mut cur = Some(start.trim().to_string());
            while let Some(name) = cur.take() {
                let body = svars_override.get(&name)?;
                let (_, mut sp) = Params::parse(body)?;
                let next = sp.take("SubAbility").map(|s| s.trim().to_string());
                order.push(name);
                cur = next;
            }
            let first_is_gated = gated_svar_names.contains(&order[0]);
            if !first_is_gated {
                return None;
            }
            // Find where the gated prefix ends: note that steps like *Vault 112*
            // (`DBShuffle [gated] -> DBExile -> DBPlay -> DBCleanup`) gate the whole
            // `"If you paid one or more {E} this way, shuffle..., then exile..."`
            // sentence, whereas *Localized Destruction* (`DBPumpAll [gated] -> DBDestroyAll`)
            // has `DBDestroyAll` on a separate paragraph without reading `X` or `Remembered`!
            let split_at = order.iter().position(|svar_name| {
                if gated_svar_names.contains(svar_name) {
                    return false;
                }
                let Some(body) = svars_override.get(svar_name) else {
                    return false;
                };
                let (api, _) = Params::parse(body).unwrap_or_default();
                api == "DestroyAll"
            });

            if let Some(idx) = split_at {
                // Detach `SubAbility$` on `order[idx - 1]` so prefix & suffix lower cleanly.
                let prev_name = &order[idx - 1];
                let prev_body = gated_chain_svars.get(prev_name)?.clone();
                let detached = prev_body
                    .split('|')
                    .map(str::trim)
                    .filter(|part| !part.starts_with("SubAbility$"))
                    .collect::<Vec<_>>()
                    .join(" | ");
                gated_chain_svars.insert(prev_name.clone(), detached);

                let gated_effs =
                    self.wave16c_run_subchain_with_svars(start, &gated_chain_svars, chain)?;
                let tail_effs =
                    self.wave16c_run_subchain_with_svars(&order[idx], &gated_chain_svars, chain)?;
                let mut out = vec![format!(
                    "Effect::IfCondition {{ condition: Condition::AmountAtLeast(\
                     Amount::ResolutionChosenNumber, 1), then: &[{}], otherwise: &[] }}",
                    gated_effs.join(", ")
                )];
                out.extend(tail_effs);
                return Some(out);
            }

            let gated_effs = self.wave16c_run_subchain_with_svars(start, svars_override, chain)?;
            return Some(vec![format!(
                "Effect::IfCondition {{ condition: Condition::AmountAtLeast(\
                 Amount::ResolutionChosenNumber, 1), then: &[{}], otherwise: &[] }}",
                gated_effs.join(", ")
            )]);
        }

        self.wave16c_run_subchain_with_svars(start, svars_override, chain)
    }

    fn wave16c_run_subchain_with_svars(
        &mut self,
        start: &str,
        svars_override: &std::collections::BTreeMap<String, String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let saved_chosen = std::mem::replace(&mut self.has_resolution_chosen_number, true);
        let base_len = chain.effects.len();
        let mut cur = Some(start.trim().to_string());
        let mut ok = Some(());
        for _ in 0..8 {
            let Some(name) = cur.take() else {
                break;
            };
            let Some(body) = svars_override.get(&name) else {
                ok = None;
                break;
            };
            let Some((s_api, mut sp)) = Params::parse(body) else {
                ok = None;
                break;
            };
            // If `s_api` consumes its own `SubAbility$` (e.g. `Dig` with `RememberChanged$`
            // -> `DBPlay` -> `DBCleanup` on *Vault 112*), pass the untouched line straight
            // to `self.chain` when no downstream SVar was modified!
            let needs_override = {
                let mut probe = sp.peek("SubAbility").map(|s| s.trim().to_string());
                let mut differs = false;
                for _ in 0..8 {
                    let Some(pn) = probe.take() else {
                        break;
                    };
                    if svars_override.get(&pn) != self.svars.get(&pn) {
                        differs = true;
                        break;
                    }
                    probe = svars_override
                        .get(&pn)
                        .and_then(|b| Params::parse(b))
                        .and_then(|(_, mut pp)| pp.take("SubAbility"))
                        .map(|s| s.trim().to_string());
                }
                differs
            };
            if !needs_override {
                if self.chain(body, chain).is_none() {
                    ok = None;
                }
                break;
            }
            let next_sub = sp.take("SubAbility").map(|s| s.trim().to_string());
            let step_line = std::iter::once(format!("DB$ {s_api}"))
                .chain(sp.entries.iter().map(|(k, v)| format!("{k}$ {v}")))
                .collect::<Vec<_>>()
                .join(" | ");
            if self.chain(&step_line, chain).is_none() {
                ok = None;
                break;
            }
            cur = next_sub;
        }
        self.has_resolution_chosen_number = saved_chosen;
        ok?;
        let produced: Vec<String> = chain.effects.drain(base_len..).collect();
        (!produced.is_empty()).then_some(produced)
    }
}
