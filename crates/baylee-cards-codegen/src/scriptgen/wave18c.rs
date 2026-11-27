//! Wave 18c script readers: basic land type choices (`ChooseType | Type$ Basic Land`),
//! APNAP any-player unless-costs (`UnlessPayer$ Player` / `TriggeredSourceSAController`),
//! event/destination/mass `Counter` rules (+ `RememberCountered*`), `GainControl` riders &
//! `NextPlayerToYourRight` / `Annihilator` / attached-LTB stealing, `DestroyAll` target/X/attached
//! variants, `BecomesBlocked`, and `Spell Crumple` bottom-library tucking.

use super::{Chain, Params, Tx, card_type_const, generic_mana, keyword_const, mana_color_const};

impl Tx<'_> {
    /// `ChooseType | Type$ Basic Land` -> `Effect::ChooseBasicLandType` +
    /// `Effect::BecomeChosenBasicLandType`.
    pub(super) fn wave18c_choose_basic_land_type_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        match p.take("Defined").as_deref() {
            None | Some("You") => {}
            Some(other) => {
                return self.deny(format!(
                    "`ChooseType` for somebody other than `You` (`{other}`)"
                ));
            }
        }
        p.take("Type");
        if p.has("InvalidTypes") {
            return self.deny("`ChooseType` of `Basic Land` with `InvalidTypes$`".to_string());
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChooseType.{key}`"));
        }
        let Some(sub_name) = sub else {
            return self.deny("`ChooseType` with no `SubAbility$`".to_string());
        };
        let Some(sub_body) = self.svars.get(sub_name).cloned() else {
            return self.deny(format!("`SubAbility$ {sub_name}` names no SVar"));
        };
        chain
            .effects
            .push("Effect::ChooseBasicLandType".to_string());

        let Some((api, mut sp)) = Params::parse(&sub_body) else {
            return self.deny("a `ChooseType` `SubAbility$` with no `$` in it".to_string());
        };
        if api != "Animate"
            || sp.peek("Types") != Some("ChosenType")
            || sp.peek("RemoveLandTypes") != Some("True")
        {
            return self.deny("`ChooseType` of `Basic Land`".to_string());
        }
        sp.drop_prose();
        sp.take("Types");
        sp.take("RemoveLandTypes");
        let next_sub = sp.take("SubAbility");
        let valid_tgts = sp.take("ValidTgts");
        let defined = sp.take("Defined");
        let target_spec = if let Some(valid) = valid_tgts {
            if !matches!(defined.as_deref(), None | Some("Targeted")) {
                return self
                    .deny("`ChooseType` `Animate` with both `ValidTgts$` and `Defined$`".into());
            }
            if chain.target.is_some() {
                return self.deny("`ChooseType` `Animate` with a second target".into());
            }
            let ts = self.target_spec(&valid, "Animate")?;
            chain.target = Some(ts.clone());
            ts
        } else {
            match defined.as_deref() {
                None | Some("Self") => "TargetSpec::ThisObject".to_string(),
                Some("Enchanted" | "Equipped") => {
                    "TargetSpec::Object(&Filter::AttachedToBySource)".to_string()
                }
                Some(other) => {
                    return self.deny(format!("`ChooseType` `Animate` on `Defined$ {other}`"));
                }
            }
        };
        let duration = match sp.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(other) => {
                return self.deny(format!("`ChooseType` `Animate` lasting `{other}`"));
            }
        };
        if let Some(key) = sp.first_key() {
            return self.deny(format!("unclaimed parameter `Animate.{key}`"));
        }
        chain.effects.push(format!(
            "Effect::BecomeChosenBasicLandType {{ target: {target_spec}, duration: {duration} }}"
        ));
        self.chain_sub_ability(next_sub, chain)
    }

    /// Lower `UnlessPayer$ Player` into `AnyPlayerMayPayManaOr` / `AnyPlayerMayPayLifeOr` /
    /// `AnyPlayerMayPayManaAmount` / `AnyPlayerMayPayCost`.
    pub(super) fn wave18c_any_player_unless(
        &mut self,
        raw_cost: &str,
        list: &str,
        switched: bool,
    ) -> Option<String> {
        if let Ok(n) = raw_cost.trim().parse::<u32>() {
            if !switched {
                let cost_expr = if n == 0 {
                    "ManaCost::ZERO".to_string()
                } else {
                    format!("mana!(\"{{{n}}}\")")
                };
                return Some(format!(
                    "Effect::AnyPlayerMayPayManaOr {{ cost: {cost_expr}, effects: &[{list}] }}"
                ));
            }
            return Some(format!(
                "Effect::AnyPlayerMayPayManaAmount {{ mana: Amount::Fixed({n}), on_payment: true, effects: &[{list}] }}"
            ));
        }
        if !raw_cost.contains('<')
            && matches!(raw_cost.trim(), "X" | "Y" | "Z")
            && let Some(mana_expr) = self.amount_or_count_ext(raw_cost)
        {
            return Some(format!(
                "Effect::AnyPlayerMayPayManaAmount {{ mana: {mana_expr}, on_payment: {switched}, effects: &[{list}] }}"
            ));
        }
        let (mana, parts) = self.cost_pieces(raw_cost)?;
        if !mana.is_empty() && parts.is_empty() {
            if !switched {
                return match generic_mana(&mana) {
                    Some(0) => Some(format!(
                        "Effect::AnyPlayerMayPayManaOr {{ cost: ManaCost::ZERO, effects: &[{list}] }}"
                    )),
                    Some(n) => Some(format!(
                        "Effect::AnyPlayerMayPayManaOr {{ cost: mana!(\"{{{n}}}\"), effects: &[{list}] }}"
                    )),
                    None => Some(format!(
                        "Effect::AnyPlayerMayPayManaOr {{ cost: mana!(\"{mana}\"), effects: &[{list}] }}"
                    )),
                };
            }
            let n = generic_mana(&mana)?;
            return Some(format!(
                "Effect::AnyPlayerMayPayManaAmount {{ mana: Amount::Fixed({n}), on_payment: true, effects: &[{list}] }}"
            ));
        }
        if !mana.is_empty() || parts.is_empty() {
            return None;
        }
        if !switched
            && let [one] = parts.as_slice()
            && let Some(life) = one
                .strip_prefix("PayLife(")
                .and_then(|s| s.strip_suffix(')'))
                .and_then(|s| s.parse::<u32>().ok())
        {
            return Some(format!(
                "Effect::AnyPlayerMayPayLifeOr {{ life: {life}, effects: &[{list}] }}"
            ));
        }
        if let [one] = parts.as_slice()
            && let Some(dmg) = one
                .strip_prefix("DealDamageToYou(")
                .and_then(|s| s.strip_suffix(')'))
                .and_then(|s| s.parse::<u16>().ok())
        {
            return Some(format!(
                "Effect::AnyPlayerMayPayCost {{ cost: AnyPlayerCost::DamageToYou({dmg}), on_payment: {switched}, effects: &[{list}] }}"
            ));
        }
        let first = &parts[0];
        let count = u8::try_from(parts.len()).ok()?;
        if count >= 1 && parts.iter().all(|p| p == first) {
            if let Some(filter) = first
                .strip_prefix("Sacrifice(")
                .and_then(|s| s.strip_suffix(')'))
            {
                return Some(format!(
                    "Effect::AnyPlayerMayPayCost {{ cost: AnyPlayerCost::Sacrifice {{ filter: {filter}, count: {count} }}, on_payment: {switched}, effects: &[{list}] }}"
                ));
            }
            if let Some(filter) = first
                .strip_prefix("ExileFromGraveyard(")
                .and_then(|s| s.strip_suffix(')'))
            {
                return Some(format!(
                    "Effect::AnyPlayerMayPayCost {{ cost: AnyPlayerCost::ExileFromGraveyard {{ filter: {filter}, count: {count} }}, on_payment: {switched}, effects: &[{list}] }}"
                ));
            }
        }
        None
    }

    /// Lower `SP$/DB$ BecomesBlocked`.
    pub(super) fn wave18c_becomes_blocked(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        match (p.take("Defined").as_deref(), target) {
            (None | Some("Targeted" | "ParentTarget"), Some(t))
                if t.starts_with("TargetSpec::Object(") =>
            {
                Some(vec![format!("Effect::BecomesBlocked({t})")])
            }
            (Some(def), None) if def.starts_with("Valid ") => {
                let raw = def.strip_prefix("Valid ")?.trim();
                let expr = self.filter_expr(raw)?;
                let name = self.body.filter_static("BECOMES_BLOCKED", &expr);
                Some(vec![format!(
                    "Effect::BecomesBlocked(TargetSpec::Object(&{name}))"
                )])
            }
            _ => None,
        }
    }

    /// Lower *Spell Crumple* self-tuck (`DB$ ChangeZone | Origin$ Stack | Destination$ Library |
    /// LibraryPosition$ -1 | Defined$ Parent`) inside a spell chain.
    pub(super) fn wave18c_put_self_on_bottom(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if !self.on_a_spell
            || p.peek("Origin") != Some("Stack")
            || p.peek("Destination") != Some("Library")
            || p.peek("LibraryPosition") != Some("-1")
            || p.peek("Defined") != Some("Parent")
            || p.has("Shuffle")
            || p.has("ValidTgts")
        {
            return None;
        }
        p.take("Origin");
        p.take("Destination");
        p.take("LibraryPosition");
        p.take("Defined");
        Some(vec!["Effect::PutSourceOnBottomOfLibrary".to_string()])
    }

    /// Lower Wave 18c `Counter` forms:
    /// - `Defined$ TriggeredSpellAbility | TriggeredSourceSA`
    /// - `Destination$ Hand | TopOfLibrary | BottomOfLibrary`
    /// - `Defined$ ValidStack <spec>` (`CounterAll`)
    /// - `RememberCountered$` / `RememberCounteredSA$` / `RememberCounteredCMC$`
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave18c_counter_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        has_sub: bool,
    ) -> Option<Vec<String>> {
        let defined = p.take("Defined");
        let dest = p.take("Destination");
        let target_type = p.take("TargetType");
        let destroy_source = match p.take("DestroyPermanent").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let rem_count = match (
            p.take("RememberCountered").as_deref(),
            p.take("RememberCounteredSA").as_deref(),
        ) {
            (None | Some("False"), None | Some("False")) => false,
            (Some("True"), None) | (None, Some("True")) => true,
            _ => return None,
        };
        let rem_cmc = match p.take("RememberCounteredCMC").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if (rem_count || rem_cmc) && !has_sub {
            return None;
        }
        if rem_count {
            self.countered_this_way = true;
        }
        if rem_cmc {
            self.remembered_cmc = true;
        }

        if let Some(def) = defined.as_deref() {
            if matches!(def, "TriggeredSpellAbility" | "TriggeredSourceSA") {
                if target.is_some()
                    || targets_here
                    || destroy_source
                    || !matches!(dest.as_deref(), None | Some("Graveyard"))
                    || rem_count
                    || rem_cmc
                {
                    return None;
                }
                let ok_trigger = match def {
                    "TriggeredSpellAbility" => matches!(
                        self.trigger_mode.as_deref(),
                        Some("SpellCast" | "SpellCastOrCopy")
                    ),
                    "TriggeredSourceSA" => self.trigger_mode.as_deref() == Some("BecomesTarget"),
                    _ => false,
                };
                if !ok_trigger {
                    return None;
                }
                return Some(vec!["Effect::CounterEventSpellOrAbility".to_string()]);
            }
            if let Some(stack_spec) = def.strip_prefix("ValidStack ") {
                if target.is_some()
                    || targets_here
                    || destroy_source
                    || !matches!(dest.as_deref(), None | Some("Graveyard"))
                    || rem_cmc
                {
                    return None;
                }
                let (scope, expr) = self.wave18c_parse_valid_stack(stack_spec.trim())?;
                let filter_ref = if expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else {
                    let name = self.body.filter_static("COUNTER_ALL", &expr);
                    format!("&{name}")
                };
                return Some(vec![format!(
                    "Effect::CounterAll {{ scope: {scope}, filter: {filter_ref} }}"
                )]);
            }
        }

        let inherited_target = match defined.as_deref() {
            None => false,
            Some("Targeted" | "ParentTarget") if !targets_here && target.is_some() => true,
            Some(_) => return None,
        };
        if target_type.is_none() && !(inherited_target || targets_here) {
            return None;
        }
        let mut effects = match target {
            Some(t) if t.starts_with("TargetSpec::Spell(") => match dest.as_deref() {
                None | Some("Graveyard") => vec!["Effect::CounterTargetSpell".to_string()],
                Some("Exile") => vec!["Effect::CounterTargetSpellToExile".to_string()],
                Some("Hand") => {
                    vec!["Effect::CounterTargetSpellTo(CounterDestination::Hand)".to_string()]
                }
                Some("TopOfLibrary") => {
                    vec![
                        "Effect::CounterTargetSpellTo(CounterDestination::TopOfLibrary)"
                            .to_string(),
                    ]
                }
                Some("BottomOfLibrary") => vec![
                    "Effect::CounterTargetSpellTo(CounterDestination::BottomOfLibrary)".to_string(),
                ],
                Some(_) => return None,
            },
            Some(t)
                if matches!(dest.as_deref(), None | Some("Graveyard"))
                    && !rem_cmc
                    && (t.starts_with("TargetSpec::AbilityOnStack(")
                        || t.starts_with("TargetSpec::ActivatedAbility(")) =>
            {
                vec!["Effect::CounterTargetAbility".to_string()]
            }
            Some(t)
                if matches!(dest.as_deref(), None | Some("Graveyard"))
                    && !rem_cmc
                    && t.starts_with("TargetSpec::SpellOrAbility(") =>
            {
                vec!["Effect::CounterTargetSpellOrAbility".to_string()]
            }
            _ => return None,
        };
        if destroy_source {
            effects.push("Effect::DestroyCounteredAbilitySource".to_string());
        }
        Some(effects)
    }

    fn wave18c_parse_valid_stack(&self, spec: &str) -> Option<(&'static str, String)> {
        let mut has_spell = false;
        let mut has_activated = false;
        let mut has_triggered = false;
        let mut qual: Option<String> = None;
        for alt in spec.split(',').map(str::trim) {
            let (base, raw_qual) = alt.split_once('.').unwrap_or((alt, ""));
            match base {
                "Spell" => has_spell = true,
                "Activated" => has_activated = true,
                "Triggered" => has_triggered = true,
                _ => return None,
            }
            match qual.as_deref() {
                None => qual = Some(raw_qual.to_string()),
                Some(prev) if prev == raw_qual => {}
                Some(_) => return None,
            }
        }
        if has_activated != has_triggered {
            return None;
        }
        let scope = match (has_spell, has_activated) {
            (true, false) => "CounterScope::Spells",
            (false, true) => "CounterScope::Abilities",
            (true, true) => "CounterScope::SpellsAndAbilities",
            (false, false) => return None,
        };
        let q = qual.unwrap_or_default();
        let expr = if q.is_empty() {
            "Filter::Any".to_string()
        } else {
            self.filter_expr(&format!("Card.{q}"))?
        };
        Some((scope, expr))
    }

    /// Lower `Phase$ Main1` / `Phase$ Main1,Main2` `DelayedTrigger` reading `RememberNumber$ True`
    /// (`Count$TriggerRememberAmount`) right after `Counter | RememberCounteredCMC$ True`
    /// (*Mana Drain* & *Plasm Capture*).
    pub(super) fn wave18c_delayed_countered_mana(
        &mut self,
        p: &mut Params,
        phase: &str,
        targets_here: bool,
    ) -> Option<String> {
        if !self.remembered_cmc
            || targets_here
            || p.peek("RememberNumber") != Some("True")
            || p.peek("ValidPlayer") != Some("You")
            || p.has("NextTurn")
            || p.has("ThisTurn")
            || p.has("OptionalDecider")
            || p.has("IsPresent")
            || p.has("CheckSVar")
            || p.has("ConditionDefined")
            || p.has("ConditionPresent")
            || p.has("RememberObjects")
        {
            return None;
        }
        let exec_name = p.peek("Execute")?;
        let exec_body = self.svars.get(exec_name.trim())?.clone();
        let (api, mut mp) = Params::parse(&exec_body)?;
        if api != "Mana" || mp.has("SubAbility") || mp.has("Defined") {
            return None;
        }
        mp.drop_prose();
        let raw_amt = mp.take("Amount")?;
        let amt_def = self
            .svars
            .get(raw_amt.trim())
            .map_or(raw_amt.as_str(), String::as_str)
            .trim();
        if amt_def != "Count$TriggerRememberAmount" {
            return None;
        }
        let produced = mp.take("Produced")?;
        if !mp.exhausted() {
            return None;
        }
        let effect = match (phase, produced.as_str()) {
            ("Main1", "Combo Any") => "Effect::DelayedManaAnyColorAtNextMain".to_string(),
            ("Main1,Main2" | "Main2,Main1", c) => {
                let color = if c == "C" {
                    "ManaColor::Colorless"
                } else {
                    mana_color_const(c)?
                };
                format!("Effect::DelayedManaAtNextMain {{ color: {color} }}")
            }
            _ => return None,
        };
        p.take("RememberNumber");
        p.take("ValidPlayer");
        p.take("Execute");
        Some(effect)
    }

    /// Lower Wave 18c `TurnFaceUp` triggers for non-self battlefield filters (`Permanent`, etc.).
    pub(super) fn wave18c_turn_face_up_matching(
        &mut self,
        p: &mut Params,
        zoned: bool,
        valid: &str,
    ) -> Option<String> {
        if !zoned
            || valid.contains("AttachedBy")
            || valid.split(['.', '+', ',']).any(|atom| atom == "Self")
        {
            return None;
        }
        let expr = self.filter_expr(valid)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        }
        Some(format!("Trigger::TurnedFaceUpMatching(&{filter})"))
    }

    /// Lower Wave 18c `GainControl` patterns:
    /// - `Defined$ AttachedTo Targeted.<filter>` (*Murderous Spoils*)
    /// - `AddKWs$` with `&` including `Annihilator:N` (*Flayer of Loyalties*, *Hideous Taskmaster*)
    pub(super) fn wave18c_gain_control_special(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if let Some(def) = p.peek("Defined")
            && let Some(rest) = def.strip_prefix("AttachedTo Targeted.")
            && !targets_a_player
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && !p.has("AllValid")
            && !p.has("Untap")
            && !p.has("AddKWs")
            && matches!(p.peek("NewController"), None | Some("You"))
        {
            let duration = match p.peek("LoseControl") {
                None => "Duration::Indefinitely",
                Some("EOT") => "Duration::UntilEndOfTurn",
                _ => return None,
            };
            let expr = self.filter_expr(&format!("Card.{rest}"))?;
            let filter = self.body.filter_static("GAIN_ATTACHED", &expr);
            p.take("Defined");
            p.take("NewController");
            p.take("LoseControl");
            return Some(vec![format!(
                "Effect::GainControlLtbAttached {{ filter: &{filter}, duration: {duration} }}"
            )]);
        }
        if let Some(t) = target.filter(|t| t.starts_with("TargetSpec::Object("))
            && let Some(kw_raw) = p.peek("AddKWs")
            && kw_raw.contains("Annihilator:")
            && !p.has("AllValid")
            && matches!(p.peek("NewController"), None | Some("You"))
            && matches!(p.peek("Defined"), None | Some("Targeted"))
            && matches!(p.peek("LoseControl"), Some("EOT"))
        {
            p.take("Defined");
            p.take("NewController");
            p.take("LoseControl");
            let kw_raw = p.take("AddKWs")?;
            let untap = match p.take("Untap").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let mut out = Vec::new();
            if untap {
                out.push("Effect::UntapTarget".to_string());
            }
            out.push(format!(
                "Effect::GainControlEx {{ target: {t}, to: PlayerRel::You, duration: Duration::UntilEndOfTurn, \
                 untap: false, haste_until_eot: false }}"
            ));
            out.extend(Self::wave18c_gain_control_add_kws(
                &kw_raw,
                "Duration::UntilEndOfTurn",
            )?);
            return Some(out);
        }
        None
    }

    /// Lower `GainControl | AddKWs$` when it grants keywords alongside `Annihilator:N` at EOT
    /// (*Flayer of Loyalties*, *Hideous Taskmaster*).
    pub(super) fn wave18c_gain_control_add_kws(
        kw_raw: &str,
        duration: &str,
    ) -> Option<Vec<String>> {
        if duration != "Duration::UntilEndOfTurn" {
            return None;
        }
        let mut kws = Vec::new();
        let mut annihilator: Option<u32> = None;
        for part in kw_raw.split('&').map(str::trim) {
            if let Some(n_str) = part.strip_prefix("Annihilator:") {
                if annihilator.is_some() {
                    return None;
                }
                let n: u32 = n_str.trim().parse().ok().filter(|&n: &u32| n >= 1)?;
                annihilator = Some(n);
            } else {
                kws.push(keyword_const(part)?);
            }
        }
        let mut out = Vec::new();
        if !kws.is_empty() {
            let kw = kws.join(".union(") + &")".repeat(kws.len().saturating_sub(1));
            out.push(format!(
                "Effect::PumpTarget {{ power: Amount::Fixed(0), toughness: Amount::Fixed(0), \
                 keywords: {kw}, duration: Duration::UntilEndOfTurn }}"
            ));
        }
        if let Some(n) = annihilator {
            out.push(format!(
                "Effect::AddTempModifier {{ target: TargetSpec::Object(&Filter::Any), \
                 modifier: Modifier::GrantTriggered {{ \
                     trigger: Trigger::Attacks(&Filter::This), \
                     effects: &[Effect::SacrificeAmountForPlayers {{ \
                         who: PlayerRel::DefendingPlayer, filter: &Filter::Any, \
                         amount: Amount::Fixed({n}) \
                     }}], target: None \
                 }}, duration: Duration::UntilEndOfTurn }}"
            ));
        }
        (!out.is_empty()).then_some(out)
    }

    /// Lower Wave 18c `DestroyAll` forms:
    /// - `TargetedCard.Self,<T>.NotDefinedTargeted+sharesNameWith Targeted` (*Echoing Calm*,
    ///   *Echoing Ruin*, *Wake of Destruction*)
    /// - `Targeted.Self,<T>+Attached` (*Eaten by Spiders*) & `<T>.AttachedTo Targeted`
    ///   (*Street Sweeper*, *Strip Bare*)
    /// - `ValidTgts$ Player|Opponent` with `ValidCards$` (*Primeval Light*, *Overwhelming Forces*,
    ///   *Rain of Daggers*, *Mogg Infestation*)
    /// - `ValidTgts$ Creature` + `ValidCards$ Creature.powerGTX` (*Fell the Mighty*)
    pub(super) fn wave18c_destroy_all_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        let valid = p.peek("ValidCards")?.to_string();
        let no_regen_raw = p.peek("NoRegen").map(str::to_string);

        if !targets_here
            && !prior_target_used
            && !targets_a_player
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && no_regen_raw.is_none()
            && let Some(rest) = valid.strip_prefix("TargetedCard.Self,")
            && let Some(base) = rest.strip_suffix(".NotDefinedTargeted+sharesNameWith Targeted")
            && !base.contains(',')
        {
            let expr = if let Some(ct) = card_type_const(base) {
                format!("Filter::HasType({ct})")
            } else {
                self.filter_expr(base)?
            };
            let filter = self.body.filter_static("DESTROY_SAME_NAME", &expr);
            p.take("ValidCards");
            return Some(vec![format!(
                "Effect::DestroyTargetAndSameName {{ filter: &{filter} }}"
            )]);
        }

        if !targets_a_player
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && no_regen_raw.is_none()
        {
            if let Some(rest) = valid.strip_prefix("Targeted.Self,")
                && let Some(att_base) = rest.strip_suffix("+Attached")
                && !targets_here
                && !prior_target_used
                && !att_base.contains(',')
            {
                let expr = self.filter_expr(att_base)?;
                let filter = self.body.filter_static("DESTROY_ATTACHED", &expr);
                p.take("ValidCards");
                return Some(vec![format!(
                    "Effect::DestroyAllAttachedToTarget {{ filter: &{filter}, destroy_target: true }}"
                )]);
            }
            let alts: Option<Vec<&str>> = valid
                .split(',')
                .map(|a| {
                    let t = a.trim();
                    t.strip_suffix(".AttachedTo Targeted")
                        .or_else(|| t.strip_suffix("+AttachedTo Targeted"))
                })
                .collect();
            if let Some(stripped) = alts {
                let expr = self.filter_expr(&stripped.join(","))?;
                let filter = self.body.filter_static("DESTROY_ATTACHED", &expr);
                p.take("ValidCards");
                return Some(vec![format!(
                    "Effect::DestroyAllAttachedToTarget {{ filter: &{filter}, destroy_target: false }}"
                )]);
            }
        }

        if targets_a_player
            && !valid.contains("TargetedPlayerCtrl")
            && target.is_some_and(|t| {
                matches!(
                    t,
                    "TargetSpec::Player(PlayerRel::Chosen)"
                        | "TargetSpec::AnyOpponent"
                        | "TargetSpec::AnyPlayer"
                )
            })
        {
            let no_regen = match no_regen_raw.as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let expr = self.filter_expr(&valid)?;
            let filter = self.body.filter_static("DESTROY", &expr);
            p.claim_label("ValidDescription", "ValidCards");
            p.take("ValidCards");
            p.take("NoRegen");
            return Some(vec![format!(
                "Effect::DestroyAllOf {{ who: PlayerRel::Chosen, filter: &{filter}, no_regen: {no_regen} }}"
            )]);
        }

        if targets_here
            && !targets_a_player
            && no_regen_raw.is_none()
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && valid == "Creature.powerGTX"
            && let Some(expr) = self.filter_expr(&valid)
        {
            p.take("ValidCards");
            return Some(vec![format!("Effect::destroy_all(&{expr})")]);
        }

        None
    }
}
