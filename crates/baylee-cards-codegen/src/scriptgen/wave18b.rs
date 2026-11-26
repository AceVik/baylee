//! Wave 18 Lane B transcoders (`Assist`, `Waterbend`, `GenericChoice`, `Bookkeeping`).

use super::saga_ward_convoke::ExtraOutcome;
use super::{Chain, Params, Price, Tx};

impl Tx<'_> {
    /// Parses `Waterbend<N>` or `Waterbend<X>` into a [`CostPart`] variant string
    /// (`CostPart::` prefix added by the caller when needed).
    pub(super) fn wave18b_waterbend_cost_token(&mut self, token: &str) -> Option<String> {
        let Some(inner) = token
            .strip_prefix("Waterbend<")
            .and_then(|t| t.strip_suffix('>'))
        else {
            return self.deny(format!("cost `{token}`"));
        };
        let inner = inner.trim();
        if inner == "X" {
            if self.has_x
                && self.svars.get("X").map(String::as_str) == Some("Count$xPaid")
                && !self.oracle.to_ascii_lowercase().contains("x is ")
            {
                return Some("WaterbendX".to_string());
            }
            return self.deny(format!("cost `{token}`"));
        }
        let Some(n) = inner.parse::<u16>().ok().filter(|&v| v > 0) else {
            return self.deny(format!("cost `{token}`"));
        };
        Some(format!("Waterbend({n})"))
    }

    /// Parses `K:Ward:Waterbend<N>` into `AbilityDef::WardWaterbend { mana: N }`.
    pub(super) fn wave18b_try_ward_waterbend(&mut self, rest: &str) -> Option<()> {
        let trimmed = rest.trim();
        let inner = trimmed
            .strip_prefix("Waterbend<")
            .and_then(|s| s.strip_suffix('>'))?;
        let Ok(mana) = inner.trim().parse::<u16>() else {
            return self.deny(format!("ward `{rest}`"));
        };
        if !(1..=10).contains(&mana) {
            return self.deny(format!("ward `{mana}`"));
        }
        self.body
            .abilities
            .push(format!("AbilityDef::WardWaterbend {{ mana: {mana} }}"));
        Some(())
    }

    /// Parses `UnlessCost$ Waterbend<N>` / `Waterbend<X>`.
    pub(super) fn wave18b_try_waterbend_unless_price(&mut self, raw: &str) -> Option<Price> {
        let inner = raw
            .trim()
            .strip_prefix("Waterbend<")
            .and_then(|s| s.strip_suffix('>'))?;
        let inner = inner.trim();
        if matches!(inner, "X" | "Y" | "Z")
            && let Some(x) = self.amount_or_count_ext(inner)
        {
            return Some(Price::Part(format!("__WATERBEND__:{x}")));
        }
        let n = inner.parse::<u32>().ok()?;
        Some(Price::Part(format!("__WATERBEND__:Amount::Fixed({n})")))
    }

    /// Escapes a Rust string literal safely for emitted DSL arrays.
    fn rust_str_lit(raw: &str) -> String {
        format!("{raw:?}")
    }

    /// Wave 18b `K:ETBReplacement:Other:<svar>` handler for:
    /// - Siege-style `DB$ GenericChoice | Defined$ You | SetChosenMode$ True | Choices$ ...`
    ///   -> `EnterModifier::ChooseMode(&[...])`
    /// - *Archangel of Strife* `DB$ GenericChoice | Defined$ Player | TempRemember$ Chooser | Choices$ ...`
    ///   -> `EnterModifier::EachPlayerChooseMode(&[...])`
    #[allow(
        clippy::too_many_lines,
        reason = "covers Siege ChooseMode and Archangel EachPlayerChooseMode"
    )]
    pub(super) fn wave18b_etb_other(&mut self, svar: &str) -> ExtraOutcome {
        let Some(body) = self.svars.get(svar).cloned() else {
            return ExtraOutcome::Pass;
        };
        let Some((api, mut p)) = Params::parse(&body) else {
            return ExtraOutcome::Pass;
        };
        if api != "GenericChoice" {
            return ExtraOutcome::Pass;
        }
        p.drop_prose();
        p.take("ShowChoice");
        p.take("AILogic");
        p.take("ChoiceTitle");
        let Some(raw_choices) = p.take("Choices") else {
            return ExtraOutcome::Handled(
                self.deny("as-enters `GenericChoice` missing `Choices$`".into()),
            );
        };
        let choice_names: Vec<String> = raw_choices
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect();
        if choice_names.len() < 2 {
            return ExtraOutcome::Handled(
                self.deny("as-enters `GenericChoice` with fewer than two choices".into()),
            );
        }

        // Case 1: Siege-style single controller mode choice (`SetChosenMode$ True`).
        if p.take("SetChosenMode").as_deref() == Some("True") {
            if let Some(lock) = p.take("LockInText")
                && lock != "True"
            {
                return ExtraOutcome::Handled(
                    self.deny("unsupported `SetChosenMode` ETB `GenericChoice`".into()),
                );
            }
            if !matches!(p.take("Defined").as_deref(), None | Some("You")) || !p.exhausted() {
                return ExtraOutcome::Handled(
                    self.deny("unsupported `SetChosenMode` ETB `GenericChoice`".into()),
                );
            }
            let mut labels = Vec::with_capacity(choice_names.len());
            for name in &choice_names {
                let Some(sub_body) = self.svars.get(name.as_str()) else {
                    return ExtraOutcome::Handled(
                        self.deny(format!("`GenericChoice` missing SVar `{name}`")),
                    );
                };
                let Some((sub_api, mut sp)) = Params::parse(sub_body) else {
                    return ExtraOutcome::Handled(
                        self.deny(format!("unreadable `GenericChoice` SVar `{name}`")),
                    );
                };
                let label = sp
                    .take("SpellDescription")
                    .map_or_else(|| name.clone(), |s| s.trim().to_string());
                sp.drop_prose();
                if sub_api != "Nothing" || !sp.exhausted() {
                    return ExtraOutcome::Handled(
                        self.deny(format!("non-empty `SetChosenMode` choice SVar `{name}`")),
                    );
                }
                labels.push(Self::rust_str_lit(&label));
            }
            self.etb_chosen_modes = choice_names;
            self.body.enter_modifiers.push(format!(
                "EnterModifier::ChooseMode(&[{}])",
                labels.join(", ")
            ));
            return ExtraOutcome::Handled(Some(()));
        }

        // Case 2: Each-player ETB choice (*Archangel of Strife*).
        if p.take("Defined").as_deref() == Some("Player")
            && p.take("TempRemember").as_deref() == Some("Chooser")
        {
            if !p.exhausted() {
                return ExtraOutcome::Handled(
                    self.deny("unsupported per-player ETB `GenericChoice`".into()),
                );
            }
            let mut noted_names = Vec::with_capacity(choice_names.len());
            let mut labels = Vec::with_capacity(choice_names.len());
            for name in &choice_names {
                let Some(sub_body) = self.svars.get(name.as_str()) else {
                    return ExtraOutcome::Handled(
                        self.deny(format!("`GenericChoice` missing SVar `{name}`")),
                    );
                };
                let Some((sub_api, mut sp)) = Params::parse(sub_body) else {
                    return ExtraOutcome::Handled(
                        self.deny(format!("unreadable `GenericChoice` SVar `{name}`")),
                    );
                };
                let label = sp
                    .take("SpellDescription")
                    .map_or_else(|| name.clone(), |s| s.trim().to_string());
                sp.drop_prose();
                if sub_api != "Bookkeeping"
                    || sp.take("Defined").as_deref() != Some("Remembered")
                    || sp.take("NoteCards").as_deref() != Some("Self")
                {
                    return ExtraOutcome::Handled(self.deny(format!(
                        "per-player ETB choice SVar `{name}` outside NoteCardsFor pattern"
                    )));
                }
                let Some(noted) = sp.take("NoteCardsFor") else {
                    return ExtraOutcome::Handled(self.deny(format!(
                        "per-player ETB choice SVar `{name}` missing `NoteCardsFor$`"
                    )));
                };
                if !sp.exhausted() {
                    return ExtraOutcome::Handled(
                        self.deny(format!("extra keys on per-player ETB choice SVar `{name}`")),
                    );
                }
                noted_names.push(noted.trim().to_string());
                labels.push(Self::rust_str_lit(&label));
            }
            self.etb_player_chosen_modes = noted_names;
            self.body.enter_modifiers.push(format!(
                "EnterModifier::EachPlayerChooseMode(&[{}])",
                labels.join(", ")
            ));
            return ExtraOutcome::Handled(Some(()));
        }

        ExtraOutcome::Pass
    }

    /// Maps `Card.ChosenMode<Name>` / `Creature.ChosenMode<Name>` to `(Filter::This, Condition::ChosenMode(idx))`.
    pub(super) fn wave18b_chosen_mode_affected(
        &self,
        affected: &str,
    ) -> Option<(&'static str, String)> {
        if self.etb_chosen_modes.is_empty() {
            return None;
        }
        let trimmed = affected.trim();
        let rest = trimmed
            .strip_prefix("Card.Self+ChosenMode")
            .or_else(|| trimmed.strip_prefix("Creature.Self+ChosenMode"))
            .or_else(|| trimmed.strip_prefix("Permanent.Self+ChosenMode"))
            .or_else(|| trimmed.strip_prefix("Card.ChosenMode"))
            .or_else(|| trimmed.strip_prefix("Creature.ChosenMode"))
            .or_else(|| trimmed.strip_prefix("Permanent.ChosenMode"))?;
        let idx = self
            .etb_chosen_modes
            .iter()
            .position(|mode| mode == rest.trim())?;
        Some(("Filter::This", format!("Condition::ChosenMode({idx})")))
    }

    /// Maps `ControlledBy Player.NotedFor<Name>` to `Filter::ControlledByPlayerWhoChoseMode(idx)`.
    pub(super) fn wave18b_controlled_by_noted(&self, atom: &str) -> Option<String> {
        let rest = atom
            .trim()
            .strip_prefix("ControlledBy Player.NotedFor")
            .or_else(|| atom.trim().strip_prefix("ControlledBy NotedFor"))?;
        let idx = self
            .etb_player_chosen_modes
            .iter()
            .position(|mode| mode == rest.trim())?;
        Some(format!("Filter::ControlledByPlayerWhoChoseMode({idx})"))
    }

    /// Lowers Siege self-granted `AddTrigger$` / `AddStaticAbility$` guarded by
    /// `Condition::ChosenMode(idx)` directly into conditional self abilities on
    /// `self.body.abilities`, supporting graveyard play statics, `RaiseCost`,
    /// `Panharmonicon`, and triggers with any `TriggerZone`.
    pub(super) fn wave18b_try_self_conditional_grants(
        &mut self,
        p: &mut Params,
        triggers: Option<&str>,
        statics: Option<&str>,
        condition: &str,
        self_granted: &mut bool,
    ) -> Option<bool> {
        if p.has("AddAbility") {
            return Some(false);
        }
        if let Some(raw) = p.take("LockInText")
            && raw != "True"
        {
            return self.deny(format!("continuous static with `LockInText$ {raw}`"));
        }
        let Some(cond_inner) = condition
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            return Some(false);
        };

        if let Some(raw) = triggers {
            for name in raw.split(" & ").map(str::trim).filter(|s| !s.is_empty()) {
                let Some(line) = self.svars.get(name).cloned() else {
                    return self.deny(format!("`AddTrigger$ {name}` naming no `SVar`"));
                };
                let prev_abilities_len = self.body.abilities.len();
                let prev_statics_len = self.body.statics.len();
                let prev_trigger_mode = self.trigger_mode.take();
                let prev_block_line = self.block_line.take();
                let prev_on_a_spell = std::mem::replace(&mut self.on_a_spell, false);
                let ok = self.triggered(&line);
                self.on_a_spell = prev_on_a_spell;
                self.trigger_mode = prev_trigger_mode;
                let block_seen = self.block_line.is_some();
                self.block_line = prev_block_line;
                if ok.is_none() || block_seen || self.body.abilities.len() <= prev_abilities_len {
                    self.body.abilities.truncate(prev_abilities_len);
                    self.body.statics.truncate(prev_statics_len);
                    if ok.is_some() {
                        return self.deny(format!(
                            "granted trigger `{name}` produced no trigger definition"
                        ));
                    }
                    return None;
                }
                let emitted = self.body.abilities.split_off(prev_abilities_len);
                for expr in emitted {
                    let Some(injected) = Self::wave18b_inject_ability_condition(&expr, cond_inner)
                    else {
                        self.body.statics.truncate(prev_statics_len);
                        return self.deny(format!(
                            "unable to guard self-granted trigger `{name}` with `{cond_inner}`"
                        ));
                    };
                    self.body.abilities.push(injected);
                }
                *self_granted = true;
            }
        }

        if let Some(raw) = statics {
            for name in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
                let Some(line) = self.svars.get(name).cloned() else {
                    return self.deny(format!("`AddStaticAbility$ {name}` naming no `SVar`"));
                };
                if !line.trim_start().starts_with("Mode$") {
                    return self.deny("a granted static without `Mode$`".to_string());
                }
                let prev_abilities_len = self.body.abilities.len();
                let prev_statics_len = self.body.statics.len();
                let prev_block_line = self.block_line.take();
                let ok = match self.try_extra_static(&line) {
                    ExtraOutcome::Handled(res) => res,
                    ExtraOutcome::Pass => self.static_ability(&line),
                };
                let block_seen = self.block_line.is_some();
                self.block_line = prev_block_line;
                if ok.is_none() || block_seen || self.body.abilities.len() <= prev_abilities_len {
                    self.body.abilities.truncate(prev_abilities_len);
                    self.body.statics.truncate(prev_statics_len);
                    if ok.is_some() {
                        return self.deny(format!(
                            "granted static `{name}` produced no static ability"
                        ));
                    }
                    return None;
                }
                let emitted = self.body.abilities.split_off(prev_abilities_len);
                for expr in emitted {
                    let Some(injected) = Self::wave18b_inject_ability_condition(&expr, cond_inner)
                    else {
                        self.body.statics.truncate(prev_statics_len);
                        return self.deny(format!(
                            "unable to guard self-granted static `{name}` with `{cond_inner}`"
                        ));
                    };
                    self.body.abilities.push(injected);
                }
                *self_granted = true;
            }
        }

        Some(true)
    }

    fn wave18b_inject_ability_condition(expr: &str, cond_inner: &str) -> Option<String> {
        let trimmed = expr.trim();
        if (trimmed.starts_with("triggered!(")
            || trimmed.starts_with("modal_triggered!(")
            || trimmed.starts_with("static_ability!("))
            && trimmed.ends_with(')')
        {
            if let Some(pos) = trimmed.rfind(", condition = Some(") {
                let prefix = &trimmed[..pos];
                let existing = &trimmed[pos + ", condition = Some(".len()..];
                let existing = existing.strip_suffix("))")?;
                return Some(format!(
                    "{prefix}, condition = Some(Condition::All(&[{cond_inner}, {existing}])))"
                ));
            }
            let open = trimmed.strip_suffix(')')?;
            return Some(format!("{open}, condition = Some({cond_inner}))"));
        }
        if trimmed.starts_with("AbilityDef::Replacement(ReplacementRule::TriggerMultiplier") {
            if let Some(prefix) = trimmed.strip_suffix(", condition: None })") {
                return Some(format!("{prefix}, condition: Some(&{cond_inner}) }})"));
            }
            if let Some(pos) = trimmed.rfind(", condition: Some(&") {
                let prefix = &trimmed[..pos];
                let existing = &trimmed[pos + ", condition: Some(&".len()..];
                let existing = existing.strip_suffix(") })")?;
                return Some(format!(
                    "{prefix}, condition: Some(&Condition::All(&[{cond_inner}, {existing}])) }})"
                ));
            }
        }
        None
    }

    /// Consumes a trailing `DB$ Cleanup | ClearRemembered$ True` (and optional
    /// trailing `ClearNotedCardsFor$`), returning the next unconsumed `SubAbility$`.
    #[allow(
        clippy::option_option,
        reason = "Outer None means invalid cleanup chain; inner Option is remaining SubAbility"
    )]
    pub(super) fn wave18b_consume_cleanup(&self, svar: Option<&str>) -> Option<Option<String>> {
        let Some(mut cur) = svar.map(|s| s.trim().to_string()) else {
            return Some(None);
        };
        for _ in 0..2 {
            let body = self.svars.get(&cur)?;
            let (api, mut p) = Params::parse(body)?;
            p.drop_prose();
            if api == "Cleanup" && p.take("ClearRemembered").as_deref() == Some("True") {
                let next = p.take("SubAbility").map(|s| s.trim().to_string());
                if !p.exhausted() {
                    return None;
                }
                match next {
                    Some(n) => cur = n,
                    None => return Some(None),
                }
                continue;
            }
            if api == "Bookkeeping"
                && p.take("Defined").as_deref() == Some("Player")
                && p.take("ClearNotedCardsFor").is_some()
            {
                let next = p.take("SubAbility").map(|s| s.trim().to_string());
                if !p.exhausted() {
                    return None;
                }
                return Some(next);
            }
            return Some(Some(cur));
        }
        Some(Some(cur))
    }

    /// Resolution `DB$ GenericChoice` handler (`PayUnlessCost` + per-player modal choices).
    pub(super) fn wave18b_try_generic_choice_chain(
        &mut self,
        mut p: Params,
        sub: &mut Option<String>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let mut probe = p.clone();
        probe.drop_prose();
        probe.take("ShowChoice");
        probe.take("ChoiceTitle");
        probe.take("DamageMap");
        let ai_logic = probe.take("AILogic");
        let fallback = probe.take("FallbackAbility");
        let choices_raw = probe.take("Choices")?;
        let choice_names: Vec<String> = choices_raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect();
        if choice_names.len() < 2 {
            return None;
        }

        // Pattern A: Pay-unless-one-of-two-costs (`Indulgent Tormentor`,
        // `Nicol Bolas, the Deceiver`, `Polygraph Orb`, `Thornplate Intimidator`,
        // `Torment of Hailfire`, `Torment of Scarabs`, `Torment of Venom`, `Zoyowa`).
        let all_pay_unless = choice_names.iter().all(|name| {
            self.svars
                .get(name.as_str())
                .is_some_and(|b| b.contains("UnlessCost$") && b.contains("UnlessPayer$"))
        });
        if ai_logic.as_deref() == Some("PayUnlessCost") || fallback.is_some() || all_pay_unless {
            return self.wave18b_pay_unless_choices(
                &mut probe,
                &choice_names,
                fallback.as_deref(),
                sub,
                targets_here,
                chain,
            );
        }

        // Pattern B: Per-player choice (`Defined$ Player | Opponent`, `TempRemember$ Chooser`).
        let defined = probe.take("Defined");
        let temp_rem = probe.take("TempRemember");
        let secret = match probe.take("Secretly").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if temp_rem.as_deref() == Some("Chooser")
            && matches!(defined.as_deref(), Some("Player" | "Opponent"))
            && probe.exhausted()
            && !targets_here
        {
            p = Params::default();
            let _ = p;
            let players_rel = if defined.as_deref() == Some("Opponent") {
                "PlayerRel::EachOpponent"
            } else {
                "PlayerRel::EachPlayer"
            };
            if secret && choice_names == ["Silence", "Snitch"] {
                return self.wave18b_prisoners_dilemma(sub);
            }
            return self.wave18b_per_player_modal(players_rel, &choice_names, sub, chain);
        }

        None
    }

    fn wave18b_pay_unless_choices(
        &mut self,
        p: &mut Params,
        choice_names: &[String],
        fallback: Option<&str>,
        sub: &mut Option<String>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let defined = p.take("Defined");
        let temp_rem = p.take("TempRemember");
        if !p.exhausted() {
            return None;
        }
        let next_after = self.wave18b_consume_cleanup(sub.as_deref())?;
        if next_after.is_some() {
            return None;
        }
        *sub = None;

        let (wrap_each_opponent, player_rel) = match (
            chain.target.as_deref(),
            defined.as_deref(),
            temp_rem.as_deref(),
        ) {
            (Some("TargetSpec::AnyOpponent"), None, None) if targets_here => {
                (false, "PlayerRel::Chosen")
            }
            (None, Some("Opponent"), Some("Chooser")) if !targets_here => {
                (true, "PlayerRel::EventPlayer")
            }
            (None, Some("TriggeredPlayer"), None)
                if !targets_here && self.trigger_mode.as_deref() == Some("Phase") =>
            {
                (false, "PlayerRel::EventPlayer")
            }
            (Some(_), Some("TargetedController"), None) if !targets_here => {
                (false, "PlayerRel::ControllerOfTarget")
            }
            _ => return None,
        };

        let mut costs = Vec::with_capacity(choice_names.len());
        let mut branch_fallbacks: Vec<Vec<String>> = Vec::with_capacity(choice_names.len());
        for name in choice_names {
            let body = self.svars.get(name.as_str())?.clone();
            let (c_api, mut cp) = Params::parse(&body)?;
            cp.drop_prose();
            cp.take("UnlessAI");
            let raw_unless = cp.take("UnlessCost")?;
            let unless_payer = cp.take("UnlessPayer")?;
            let payer_ok = match (unless_payer.as_str(), player_rel) {
                ("Remembered", "PlayerRel::EventPlayer") => wrap_each_opponent,
                ("TriggeredPlayer", "PlayerRel::EventPlayer") => !wrap_each_opponent,
                ("Targeted", "PlayerRel::Chosen")
                | ("TargetedController", "PlayerRel::ControllerOfTarget") => true,
                _ => false,
            };
            if !payer_ok || cp.has("SubAbility") {
                return None;
            }
            let cost_expr = match self.unless_price(raw_unless.trim())? {
                Price::Part(part) => format!("CostPart::{part}"),
                _ => return None,
            };
            costs.push(cost_expr);

            let effs = self.wave18b_lower_pay_unless_base(&c_api, cp, wrap_each_opponent)?;
            branch_fallbacks.push(effs);
        }

        let otherwise_effects = if let Some(fb_name) = fallback {
            let fb_body = self.svars.get(fb_name.trim())?.clone();
            let (fb_api, mut fp) = Params::parse(&fb_body)?;
            fp.drop_prose();
            if fp.has("SubAbility") {
                return None;
            }
            self.wave18b_lower_pay_unless_base(&fb_api, fp, wrap_each_opponent)?
        } else {
            let first = branch_fallbacks.first()?.clone();
            if !branch_fallbacks.iter().all(|b| b == &first) {
                return None;
            }
            first
        };

        let pay_eff = format!(
            "Effect::PlayerMayPayOneOfCostsOr {{ player: {player_rel}, costs: &[{}], otherwise: &[{}] }}",
            costs.join(", "),
            otherwise_effects.join(", "),
        );
        Some(if wrap_each_opponent {
            vec![format!(
                "Effect::ForEachPlayer {{ players: PlayerRel::EachOpponent, effects: &[{pay_eff}] }}"
            )]
        } else {
            vec![pay_eff]
        })
    }

    fn wave18b_lower_pay_unless_base(
        &mut self,
        api: &str,
        mut p: Params,
        wrap_each_opponent: bool,
    ) -> Option<Vec<String>> {
        let defined = p.take("Defined");
        match api {
            "Draw" => {
                if !matches!(defined.as_deref(), None | Some("You")) {
                    return None;
                }
                let n = p
                    .take("NumCards")
                    .map_or(Some(1_u32), |s| s.trim().parse().ok())?;
                if !p.exhausted() {
                    return None;
                }
                Some(vec![format!("Effect::draw({n})")])
            }
            "LoseLife" => {
                let who =
                    self.wave18b_player_rel_of_defined(defined.as_deref(), wrap_each_opponent)?;
                let amt_raw = p.take("LifeAmount")?;
                let amt = self.amount_num(&amt_raw)?;
                if !p.exhausted() {
                    return None;
                }
                Some(vec![format!(
                    "Effect::LoseLife {{ amount: {amt}, target: {who} }}"
                )])
            }
            "DealDamage" => {
                let who =
                    self.wave18b_player_rel_of_defined(defined.as_deref(), wrap_each_opponent)?;
                let amt_raw = p.take("NumDmg")?;
                let amt = self.amount_num(&amt_raw)?;
                if !p.exhausted() {
                    return None;
                }
                Some(vec![format!(
                    "Effect::DealDamage {{ target: TargetSpec::Player({who}), amount: {amt} }}"
                )])
            }
            _ => None,
        }
    }

    fn wave18b_player_rel_of_defined(
        &self,
        defined: Option<&str>,
        wrap_each_opponent: bool,
    ) -> Option<&'static str> {
        Some(match defined? {
            "You" => "PlayerRel::You",
            "Targeted" => "PlayerRel::Chosen",
            "TriggeredPlayer" if self.trigger_mode.as_deref() == Some("Phase") => {
                "PlayerRel::EventPlayer"
            }
            "Remembered" | "Player.IsRemembered" if wrap_each_opponent => "PlayerRel::EventPlayer",
            "TargetedController" | "Player.IsRemembered" => "PlayerRel::ControllerOfTarget",
            _ => return None,
        })
    }

    fn wave18b_prisoners_dilemma(&self, sub: &mut Option<String>) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?;
        let (a1, mut p1) = Params::parse(s1)?;
        if a1 != "Branch"
            || p1.take("BranchConditionSVar").as_deref()
                != Some("PlayerCountOpponents$HasPropertyNotedForSilence")
        {
            return None;
        }
        let s2 = self.svars.get(p1.take("FalseSubAbility")?.trim())?;
        let (a2, mut p2) = Params::parse(s2)?;
        if a2 != "Branch"
            || p2.take("BranchConditionSVar").as_deref()
                != Some("PlayerCountOpponents$HasPropertyNotedForSnitch")
        {
            return None;
        }
        for (svar_name, expected_dmg) in [
            (p1.take("TrueSubAbility")?, "4"),
            (p2.take("TrueSubAbility")?, "8"),
            (p2.take("FalseSubAbility")?, "12"),
        ] {
            let body = self.svars.get(svar_name.trim())?;
            let (da, mut dp) = Params::parse(body)?;
            if da != "DealDamage" || dp.take("NumDmg").as_deref() != Some(expected_dmg) {
                return None;
            }
            let next = dp.take("SubAbility")?;
            let clean = self.wave18b_consume_cleanup(Some(&next))?;
            if clean.is_some() {
                return None;
            }
        }
        *sub = None;
        Some(vec![
            "Effect::PlayersChooseMode(MultiPlayerChoice { \
                 players: PlayerRel::EachOpponent, \
                 labels: &[\"Silence\", \"Snitch\"], \
                 branches: &[\
                     &[Effect::DealDamage { target: TargetSpec::Player(PlayerRel::EventPlayer), amount: Amount::Fixed(4) }] as &[Effect], \
                     &[Effect::DealDamage { target: TargetSpec::Player(PlayerRel::EventPlayer), amount: Amount::Fixed(8) }] as &[Effect], \
                     &[Effect::DealDamage { target: TargetSpec::Player(PlayerRel::EventPlayer), amount: Amount::Fixed(12) }] as &[Effect], \
                 ], \
                 secret: true, \
                 aggregate: MultiPlayerChoiceAggregate::PrisonersDilemma, \
             })"
            .to_string(),
        ])
    }

    fn wave18b_per_player_modal(
        &mut self,
        players_rel: &str,
        choice_names: &[String],
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        // Parse every choice SVar: either `DB$ Bookkeeping | NoteCardsFor$ <Tag>`
        // or `DB$ Nothing`.
        let mut labels = Vec::with_capacity(choice_names.len());
        let mut noted_tags: Vec<Option<String>> = Vec::with_capacity(choice_names.len());
        for name in choice_names {
            let body = self.svars.get(name.as_str())?;
            let (api, mut sp) = Params::parse(body)?;
            let label = sp
                .take("SpellDescription")
                .map_or_else(|| name.clone(), |s| s.trim().to_string());
            sp.drop_prose();
            if api == "Bookkeeping"
                && sp.take("Defined").as_deref() == Some("Remembered")
                && sp.take("NoteCards").as_deref() == Some("Self")
            {
                let tag = sp.take("NoteCardsFor")?.trim().to_string();
                if !sp.exhausted() {
                    return None;
                }
                noted_tags.push(Some(tag));
                labels.push(Self::rust_str_lit(&label));
            } else if api == "Nothing" {
                sp.take("Defined");
                if !sp.exhausted() {
                    return None;
                }
                noted_tags.push(None);
                labels.push(Self::rust_str_lit(&label));
            } else {
                return None;
            }
        }

        // Case B1: Discard-if-you-do family (`Borderland Explorer`, `Mog, Moogle Warrior`,
        // `The Death of Gwen Stacy` Chapter II) and `Imposing Grandeur`.
        if noted_tags == [Some("Discard".to_string()), None] {
            return self
                .wave18b_per_player_discard_choice(players_rel, sub)
                .or_else(|| self.wave18b_imposing_grandeur(players_rel, &labels, sub, chain));
        }

        // Case B2: Timetwister choice (`Step Between Worlds`, `Turtles in Time`).
        if noted_tags == [Some("Stargate".to_string()), None] {
            return self.wave18b_per_player_timetwister(players_rel, &labels, sub);
        }

        // Case B2b: `Wheel of Potential` (ChooseNumber energy payment -> players may exile hand & draw X).
        if noted_tags == [Some("ExileDraw".to_string()), None] && self.has_resolution_chosen_number
        {
            return self.wave18b_wheel_of_potential(players_rel, &labels, sub, chain);
        }

        // Case B3: Ordered `DB$ RepeatEach` loops over `Player.NotedFor<Tag>`
        // (`Master of Ceremonies`, `Seize the Spotlight`).
        if noted_tags.iter().all(Option::is_some) {
            let mut branches = Vec::with_capacity(noted_tags.len());
            let mut cur = sub.clone();
            for tag_opt in &noted_tags {
                let tag = tag_opt.as_deref()?;
                let s_name = cur.take()?;
                let body = self.svars.get(s_name.trim())?.clone();
                let (api, mut rp) = Params::parse(&body)?;
                rp.drop_prose();
                if api != "RepeatEach"
                    || rp.take("RepeatPlayers").as_deref() != Some(&format!("Player.NotedFor{tag}"))
                {
                    return None;
                }
                rp.take("ClearRememberedBeforeLoop");
                let repeat_sub = rp.take("RepeatSubAbility")?;
                let next_loop = rp.take("SubAbility").map(|s| s.trim().to_string());
                if !rp.exhausted() {
                    return None;
                }
                let effs = self.wave18b_lower_repeat_chooser_body(repeat_sub.trim())?;
                branches.push(format!("&[{}] as &[Effect]", effs.join(", ")));
                cur = next_loop;
            }
            let remaining = self.wave18b_consume_cleanup(cur.as_deref())?;
            *sub = remaining;
            return Some(vec![format!(
                "Effect::PlayersChooseMode(MultiPlayerChoice {{ \
                     players: {players_rel}, \
                     labels: &[{}], \
                     branches: &[{}], \
                     secret: false, \
                     aggregate: MultiPlayerChoiceAggregate::PerModeChooser, \
                 }})",
                labels.join(", "),
                branches.join(", "),
            )]);
        }

        None
    }

    fn wave18b_lower_repeat_chooser_body(&mut self, svar: &str) -> Option<Vec<String>> {
        let body = self.svars.get(svar)?.clone();
        let (api, mut p) = Params::parse(&body)?;
        p.drop_prose();
        // Pattern 1 (`Master of Ceremonies`): `RememberObjects$ You` + `SubAbility$` using `Remembered`
        // for You & EventPlayer -> `[for_you, for_event_player]`.
        if api == "Bookkeeping" && p.take("RememberObjects").as_deref() == Some("You") {
            let eff_svar = p.take("SubAbility")?;
            if !p.exhausted() {
                return None;
            }
            let eff_body = self.svars.get(eff_svar.trim())?.clone();
            let (ea, mut ep) = Params::parse(&eff_body)?;
            ep.drop_prose();
            if ep.has("SubAbility") {
                return None;
            }
            return match ea.as_str() {
                "Token" => {
                    if ep.take("TokenOwner").as_deref() != Some("Remembered") {
                        return None;
                    }
                    let rebuilt_you = format!(
                        "DB$ Token | TokenOwner$ You | {}",
                        ep.entries
                            .iter()
                            .map(|(k, v)| format!("{k}$ {v}"))
                            .collect::<Vec<_>>()
                            .join(" | ")
                    );
                    let mut c = Chain::default();
                    self.chain(&rebuilt_you, &mut c)?;
                    if c.effects.len() != 1 || c.target.is_some() {
                        return None;
                    }
                    let you_eff = c.effects.pop()?;
                    let event_eff = if you_eff.contains("owner: PlayerRel::You") {
                        you_eff.replace("owner: PlayerRel::You", "owner: PlayerRel::EventPlayer")
                    } else {
                        let tok = you_eff
                            .strip_prefix("Effect::CreateToken { token: ")
                            .and_then(|s| s.strip_suffix(" }"))?;
                        format!(
                            "Effect::CreateTokensFor {{ player: PlayerRel::EventPlayer, token: {tok}, amount: Amount::Fixed(1) }}"
                        )
                    };
                    Some(vec![you_eff, event_eff])
                }
                "Draw" => {
                    if ep.take("Defined").as_deref() != Some("Remembered") {
                        return None;
                    }
                    let n = ep
                        .take("NumCards")
                        .map_or(Some(1_u32), |s| s.trim().parse().ok())?;
                    if !ep.exhausted() {
                        return None;
                    }
                    Some(vec![
                        format!("Effect::draw({n})"),
                        format!(
                            "Effect::DrawCardsFor {{ amount: Amount::Fixed({n}), who: PlayerRel::EventPlayer }}"
                        ),
                    ])
                }
                _ => None,
            };
        }

        // Pattern 2 (`Seize the Spotlight` Fame mode):
        // `GainControl | Choices$ Creature.RememberedPlayerCtrl | LoseControl$ EOT | Untap$ True | AddKWs$ Haste`.
        if api == "GainControl"
            && p.take("Choices").as_deref() == Some("Creature.RememberedPlayerCtrl")
            && p.take("LoseControl").as_deref() == Some("EOT")
            && p.take("Untap").as_deref() == Some("True")
            && p.take("AddKWs").as_deref() == Some("Haste")
            && p.exhausted()
        {
            return Some(vec![
                "Effect::ChooseCard { \
                     chooser: PlayerRel::You, \
                     zone: ChooseZone::Battlefield, \
                     filter: &Filter::And(&[Filter::CREATURE, Filter::ControlledBy(PlayerRel::EventPlayer)]), \
                     min: 1, \
                     max: Amount::Fixed(1), \
                     movement: ChooseMove::None, \
                     then: &[Effect::GainControlEx { \
                         target: TargetSpec::EventObject, \
                         to: PlayerRel::You, \
                         duration: Duration::UntilEndOfTurn, \
                         untap: true, \
                         haste_until_eot: true, \
                     }], \
                 }"
                .to_string(),
            ]);
        }

        // Pattern 3: Standard effect chain (e.g. `Seize the Spotlight` Fortune mode: `Draw -> Treasure`).
        let mut c = Chain::default();
        self.chain(&body, &mut c)?;
        if c.effects.is_empty() || c.target.is_some() || c.second_target.is_some() {
            return None;
        }
        Some(c.effects)
    }

    fn wave18b_per_player_timetwister(
        &self,
        players_rel: &str,
        labels: &[String],
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?;
        let (a1, mut p1) = Params::parse(s1)?;
        p1.drop_prose();
        if a1 != "ChangeZoneAll"
            || p1.take("ChangeType").as_deref() != Some("Card.OwnedBy Player.NotedForStargate")
            || p1.take("Origin").as_deref() != Some("Hand,Graveyard")
            || p1.take("Destination").as_deref() != Some("Library")
            || p1.take("Shuffle").as_deref() != Some("True")
            || p1.take("UseAllOriginZones").as_deref() != Some("True")
        {
            return None;
        }
        let draw_svar = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }
        let s2 = self.svars.get(draw_svar.trim())?;
        let (a2, mut p2) = Params::parse(s2)?;
        p2.drop_prose();
        if a2 != "Draw"
            || p2.take("Defined").as_deref() != Some("Player.NotedForStargate")
            || p2.take("NumCards").as_deref() != Some("7")
        {
            return None;
        }
        let next_after = p2.take("SubAbility");
        if !p2.exhausted() {
            return None;
        }
        *sub = self.wave18b_consume_cleanup(next_after.as_deref())?;
        Some(vec![format!(
            "Effect::PlayersChooseMode(MultiPlayerChoice {{ \
                 players: {players_rel}, \
                 labels: &[{}], \
                 branches: &[\
                     &[\
                         Effect::ShuffleIntoLibrary {{ who: PlayerRel::EventPlayer, hand: true, graveyard: true }}, \
                         Effect::DrawCardsFor {{ amount: Amount::Fixed(7), who: PlayerRel::EventPlayer }}\
                     ] as &[Effect], \
                     &[] as &[Effect]\
                 ], \
                 secret: false, \
                 aggregate: MultiPlayerChoiceAggregate::PerModeChooser, \
             }})",
            labels.join(", "),
        )])
    }

    #[allow(
        clippy::too_many_lines,
        reason = "handles 3 per-player discard continuation patterns"
    )]
    fn wave18b_per_player_discard_choice(
        &mut self,
        players_rel: &str,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let d_body = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (da, mut dp) = Params::parse(&d_body)?;
        dp.drop_prose();
        if da != "Discard"
            || dp.take("Defined").as_deref() != Some("Player.NotedForDiscard")
            || dp.take("Mode").as_deref() != Some("TgtChoose")
            || dp.take("RememberDiscarded").as_deref() != Some("True")
        {
            return None;
        }
        let next_svar = dp.take("SubAbility")?;
        if !dp.exhausted() {
            return None;
        }
        let n_body = self.svars.get(next_svar.trim())?.clone();
        let (na, mut np) = Params::parse(&n_body)?;
        np.drop_prose();

        // 1. `The Death of Gwen Stacy` Chapter II: discard or lose 3 life.
        if na == "LoseLife"
            && np.take("Defined").as_deref() == Some("NonRememberedOwner")
            && np.take("LifeAmount").as_deref() == Some("3")
        {
            let tail = np.take("SubAbility");
            if !np.exhausted() {
                return None;
            }
            if self.wave18b_consume_cleanup(tail.as_deref())?.is_some() {
                return None;
            }
            *sub = None;
            return Some(vec![format!(
                "Effect::ForEachPlayer {{ \
                     players: {players_rel}, \
                     effects: &[Effect::PlayerMayPayCostOr {{ \
                         player: PlayerRel::EventPlayer, \
                         cost: &CostPart::Discard(&Filter::Any), \
                         effect: &Effect::LoseLife {{ amount: Amount::Fixed(3), target: PlayerRel::EventPlayer }}, \
                     }}] \
                 }}"
            )]);
        }

        // 2. `Borderland Explorer`: who discards may search their library for a basic land -> hand.
        if na == "ChangeZone"
            && np.take("Origin").as_deref() == Some("Library")
            && np.take("Destination").as_deref() == Some("Hand")
            && np.take("ChangeType").as_deref() == Some("Land.Basic")
            && np.take("DefinedPlayer").as_deref() == Some("RememberedOwner")
        {
            np.take("ChangeTypeDesc");
            let tail = np.take("SubAbility");
            if !np.exhausted() {
                return None;
            }
            if self.wave18b_consume_cleanup(tail.as_deref())?.is_some() {
                return None;
            }
            *sub = None;
            return Some(vec![format!(
                "Effect::ForEachPlayer {{ \
                     players: {players_rel}, \
                     effects: &[Effect::PlayerMayPayCostThen {{ \
                         player: PlayerRel::EventPlayer, \
                         cost: &CostPart::Discard(&Filter::Any), \
                         effect: &Effect::SearchLibraryOf {{ \
                             library: PlayerRel::EventPlayer, \
                             owner_searches: true, \
                             filter: &Filter::BASIC_LAND, \
                             mana_value: None, \
                             finds: &[Find::HAND], \
                             optional: true, \
                         }}, \
                     }}] \
                 }}"
            )]);
        }

        // 3. `Mog, Moogle Warrior`: who discards draws 1; if creature discarded -> Moogle token;
        //    if noncreature discarded -> +1/+1 counter on each Moogle you control.
        if na == "Draw" && np.take("Defined").as_deref() == Some("RememberedOwner") {
            let tok_svar = np.take("SubAbility")?;
            if !np.exhausted() {
                return None;
            }
            let tok_body = self.svars.get(tok_svar.trim())?.clone();
            let (ta, mut tp) = Params::parse(&tok_body)?;
            tp.drop_prose();
            if ta != "Token"
                || tp.take("ConditionDefined").as_deref() != Some("Remembered")
                || tp.take("ConditionPresent").as_deref() != Some("Creature")
                || tp.take("TokenOwner").as_deref() != Some("You")
            {
                return None;
            }
            let ctr_svar = tp.take("SubAbility")?;
            let script = tp.take("TokenScript")?;
            if !tp.exhausted() {
                return None;
            }
            let mut tc = Chain::default();
            self.chain(
                &format!("DB$ Token | TokenScript$ {script} | TokenOwner$ You"),
                &mut tc,
            )?;
            let [tok_eff] = tc.effects.as_slice() else {
                return None;
            };

            let ctr_body = self.svars.get(ctr_svar.trim())?.clone();
            let (ca, mut cp) = Params::parse(&ctr_body)?;
            cp.drop_prose();
            if ca != "PutCounterAll"
                || cp.take("ConditionDefined").as_deref() != Some("Remembered")
                || cp.take("ConditionPresent").as_deref() != Some("Card.nonCreature")
            {
                return None;
            }
            let tail = cp.take("SubAbility");
            let valid_cards = cp.take("ValidCards")?;
            let ctr_type = cp.take("CounterType")?;
            let ctr_num = cp.take("CounterNum")?;
            if !cp.exhausted() {
                return None;
            }
            let mut cc = Chain::default();
            self.chain(
                &format!(
                    "DB$ PutCounterAll | ValidCards$ {valid_cards} | CounterType$ {ctr_type} | CounterNum$ {ctr_num}"
                ),
                &mut cc,
            )?;
            let [ctr_eff] = cc.effects.as_slice() else {
                return None;
            };
            if self.wave18b_consume_cleanup(tail.as_deref())?.is_some() {
                return None;
            }
            *sub = None;
            return Some(vec![
                format!(
                    "Effect::ForEachPlayer {{ \
                         players: {players_rel}, \
                         effects: &[Effect::PlayerMayPayCostThen {{ \
                             player: PlayerRel::EventPlayer, \
                             cost: &CostPart::Discard(&Filter::Any), \
                             effect: &Effect::DrawCardsFor {{ amount: Amount::Fixed(1), who: PlayerRel::EventPlayer }}, \
                         }}] \
                     }}"
                ),
                format!(
                    "Effect::IfCondition {{ \
                         condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::DiscardedMatching(&Filter::CREATURE)), 1), \
                         then: &[{tok_eff}], \
                         otherwise: &[] \
                     }}"
                ),
                format!(
                    "Effect::IfCondition {{ \
                         condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::DiscardedMatching(&Filter::NONCREATURE)), 1), \
                         then: &[{ctr_eff}], \
                         otherwise: &[] \
                     }}"
                ),
            ]);
        }

        None
    }

    fn wave18b_imposing_grandeur(
        &mut self,
        players_rel: &str,
        labels: &[String],
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        if a1 != "RepeatEach"
            || p1.take("RepeatPlayers").as_deref() != Some("Player.NotedForDiscard")
        {
            return None;
        }
        let disc_name = p1.take("RepeatSubAbility")?;
        let next_after = p1.take("SubAbility");
        if !p1.exhausted() {
            return None;
        }
        let d_body = self.svars.get(disc_name.trim())?.clone();
        let (da, mut dp) = Params::parse(&d_body)?;
        dp.drop_prose();
        if da != "Discard"
            || dp.take("Mode").as_deref() != Some("Hand")
            || dp.take("Defined").as_deref() != Some("Player.IsRemembered")
        {
            return None;
        }
        let draw_name = dp.take("SubAbility")?;
        if !dp.exhausted() {
            return None;
        }
        let dr_body = self.svars.get(draw_name.trim())?.clone();
        let (dra, mut drp) = Params::parse(&dr_body)?;
        drp.drop_prose();
        if dra != "Draw" || drp.take("Defined").as_deref() != Some("Player.IsRemembered") {
            return None;
        }
        let num_cards = drp.take("NumCards")?;
        if !drp.exhausted() {
            return None;
        }
        let x_def = self.svars.get(num_cards.trim())?.trim();
        if !matches!(
            x_def,
            "Count$ValidBattlefield,Command Card.IsCommander+RememberedPlayerOwn$GreatestCardManaCost"
                | "Count$ValidBattlefield,Command Card.RememberedPlayerOwn+IsCommander$GreatestCardManaCost"
        ) {
            return None;
        }
        let remaining = self.wave18b_consume_cleanup(next_after.as_deref())?;
        let cmd_filter = self.body.filter_static(
            "IMPOSING",
            "Filter::And(&[Filter::IsCommander, Filter::OwnedBy(PlayerRel::EventPlayer), Filter::InZone(ZoneRef::BattlefieldOrCommand)])",
        );
        chain.moves_library = true;
        *sub = remaining;
        Some(vec![format!(
            "Effect::PlayersChooseMode(MultiPlayerChoice {{ \
                 players: {players_rel}, \
                 labels: &[{}], \
                 branches: &[\
                     &[\
                         Effect::DiscardHand {{ who: PlayerRel::EventPlayer }}, \
                         Effect::DrawCardsFor {{ \
                             amount: Amount::GreatestManaValueAmong(&{cmd_filter}), \
                             who: PlayerRel::EventPlayer \
                         }}\
                     ] as &[Effect], \
                     &[] as &[Effect]\
                 ], \
                 secret: false, \
                 aggregate: MultiPlayerChoiceAggregate::PerModeChooser, \
             }})",
            labels.join(", "),
        )])
    }

    fn wave18b_wheel_of_potential(
        &mut self,
        players_rel: &str,
        labels: &[String],
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let s1 = self.svars.get(sub.as_deref()?.trim())?.clone();
        let (a1, mut p1) = Params::parse(&s1)?;
        p1.drop_prose();
        if a1 != "ChangeZoneAll"
            || p1.take("Origin").as_deref() != Some("Hand")
            || p1.take("Destination").as_deref() != Some("Exile")
            || p1.take("Defined").as_deref() != Some("Player.NotedForExileDraw")
            || p1.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        let draw_name = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }
        let s2 = self.svars.get(draw_name.trim())?.clone();
        let (a2, mut p2) = Params::parse(&s2)?;
        p2.drop_prose();
        if a2 != "Draw" || p2.take("Defined").as_deref() != Some("Player.NotedForExileDraw") {
            return None;
        }
        let num_var = p2.take("NumCards")?;
        if self.svars.get(num_var.trim()).map(String::as_str) != Some("Count$ChosenNumber")
            || self.amount_or_count_ext(&num_var).as_deref()
                != Some("Amount::ResolutionChosenNumber")
        {
            return None;
        }
        let eff_name = p2.take("SubAbility")?;
        if !p2.exhausted() {
            return None;
        }
        let s3 = self.svars.get(eff_name.trim())?.clone();
        let (a3, mut p3) = Params::parse(&s3)?;
        p3.drop_prose();
        if a3 != "Effect"
            || p3.take("ConditionCheckSVar").as_deref() != Some(num_var.trim())
            || p3.take("ConditionSVarCompare").as_deref() != Some("GE7")
            || p3.take("RememberObjects").as_deref() != Some("Remembered.YouOwn")
            || p3.take("Duration").as_deref() != Some("UntilTheEndOfYourNextTurn")
            || p3.take("ForgetOnMoved").as_deref() != Some("Exile")
        {
            return None;
        }
        let play_name = p3.take("StaticAbilities")?;
        let next_after = p3.take("SubAbility");
        if !p3.exhausted() {
            return None;
        }
        let play_body = self.svars.get(play_name.trim())?.clone();
        let (pa, mut pp) = Params::parse(&play_body)?;
        pp.drop_prose();
        if pa != "Continuous"
            || pp.take("MayPlay").as_deref() != Some("True")
            || pp.take("Affected").as_deref() != Some("Card.YouOwn+IsRemembered")
            || pp.take("AffectedZone").as_deref() != Some("Exile")
            || !pp.exhausted()
        {
            return None;
        }
        let remaining = self.wave18b_consume_cleanup(next_after.as_deref())?;
        chain.moves_library = true;
        *sub = remaining;
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
                 first: &[Effect::PlayersChooseMode(MultiPlayerChoice {{ \
                     players: {players_rel}, \
                     labels: &[{}], \
                     branches: &[\
                         &[\
                             Effect::ExileHand {{ who: PlayerRel::EventPlayer }}, \
                             Effect::DrawCardsFor {{ amount: Amount::ResolutionChosenNumber, who: PlayerRel::EventPlayer }}\
                         ] as &[Effect], \
                         &[] as &[Effect]\
                     ], \
                     secret: false, \
                     aggregate: MultiPlayerChoiceAggregate::PerModeChooser, \
                 }})], \
                 then: &[Effect::IfCondition {{ \
                     condition: Condition::AmountAtLeast(Amount::ResolutionChosenNumber, 7), \
                     then: &[Effect::GrantRememberedExilePermission {{ \
                         filter: &Filter::OwnedByYou, \
                         permission: ExilePermission {{ \
                             window: PlayWindow::ThroughYourNextTurn, \
                             cast_only: false, \
                             spell_types: None, \
                             free: false, \
                             mana: ExileMana::Exact, \
                             caster: PlayerRel::You, \
                         }}, \
                     }}], \
                     otherwise: &[], \
                 }}] \
             }}",
            labels.join(", "),
        )])
    }
}
