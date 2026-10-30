//! Reading an effect and the chain of sub-abilities it hands on to, with
//! the "unless" a player may pay.

use super::{
    Chain, Params, Price, Tx, Unless, amount, asks_for_an_object, counter_kind, generic_mana,
    is_supported_api, plain_number,
};

impl Tx<'_> {
    /// One effect and everything its `SubAbility$` chain adds.
    pub(super) fn chain(&mut self, spec: &str, chain: &mut Chain) -> Option<()> {
        let Some((api, mut p)) = Params::parse(spec) else {
            return self.deny("an ability spec with no `$` in it".to_string());
        };
        p.drop_prose();
        chain.could_add_mana |= api == "Mana";
        chain.moves_library |= matches!(api.as_str(), "Draw" | "Mill" | "Surveil")
            || (matches!(api.as_str(), "ChangeZone" | "ChangeZoneAll")
                && (p
                    .peek("Origin")
                    .is_some_and(|zones| zones.split(',').any(|zone| zone == "Library"))
                    != (p.peek("Destination") == Some("Library"))));
        let valid = p.take("ValidTgts");
        let targets_here = valid.is_some();
        if let Some(valid) = valid {
            self.chain_target(&api, &mut p, &valid, chain)?;
        }
        let mut sub = p.take("SubAbility");
        // The chosen-source shield is three lines in the reference and one
        // sentence on the card: its `SubAbility$` is part of the sentence,
        // not the next one, so this rule reads the rest of the chain itself.
        if api == "ChooseSource" {
            let effects = self.prevent_from_chosen_source(p, sub.as_deref())?;
            chain.effects.extend(effects);
            return Some(());
        }
        if api == "ChooseColor" {
            let effects = self.choose_color_animate(p, sub.as_deref(), chain)?;
            chain.effects.extend(effects);
            return Some(());
        }
        if api == "ChoosePlayer" {
            let effects = self.choose_player_effect(&mut p, sub.as_deref())?;
            chain.effects.extend(effects);
            return Some(());
        }
        if api == "GenericChoice" {
            let effects =
                self.guess_named_card_effect(p, sub.as_deref(), chain.target.as_deref())?;
            chain.effects.extend(effects);
            return Some(());
        }
        if api == "RepeatEach" {
            let effects = self.repeat_each_effect(p, sub.as_deref(), chain)?;
            chain.effects.extend(effects);
            return Some(());
        }
        // The remembered objects are the damage recipients, not this line's
        // original targets. Read their entire lifetime and consumer together;
        // accepting the flag alone would silently drop a rules clause.
        let prevent_damaged_regeneration = self.take_damage_recipient_regeneration(
            &api,
            &mut p,
            &mut sub,
            chain.effects.is_empty(),
        )?;
        let target = Self::effect_target(chain);
        // Whether an absent `Defined$` on this line means a chosen player:
        // only where this line declared the player target itself.
        let targets_a_player =
            targets_here && target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
        // "Sacrifice it unless you pay {U}", "counter target spell unless
        // its controller pays {2}": the price is the line's and not its
        // effect's, so it is read here for every API and wraps whatever the
        // line turns out to say.
        let unless = match p.take("UnlessCost") {
            Some(cost) => Some(self.unless(&cost, &mut p, target.as_deref(), sub.is_some())?),
            None => None,
        };
        // "If that creature would die this turn, exile it instead": a rider
        // on whatever the line does to its target, read for every API.
        let dying = match p.take("ReplaceDyingDefined") {
            Some(defined) => Some(self.exile_if_dies(&defined, target.as_deref())?),
            None => None,
        };
        let at_end = match p.take("AtEOT") {
            Some(what) => Some(self.at_next_end_step(&api, &what, target.as_deref())?),
            None => None,
        };

        let Some(effects) = (if chain.second_target.is_some() {
            self.pump_effect(&mut p, target.as_deref().expect("second target"), true)
        } else {
            self.effect_of(
                &api,
                &mut p,
                target.as_deref(),
                targets_here,
                targets_a_player,
            )
        }) else {
            // An API with no rule at all is a different report than a rule
            // that met a value it cannot say — the first is a missing
            // effect, the second is a missing case in one that exists.
            if is_supported_api(&api) {
                self.note(format!("unreadable value in `{api}`"));
            } else {
                self.note(format!("effect `{api}`"));
            }
            return None;
        };
        let mut effects =
            self.wrap_line_condition(&api, &mut p, &mut sub, target.as_deref(), effects)?;
        if prevent_damaged_regeneration {
            effects.push("Effect::DamagedThisWayCantBeRegenerated".to_string());
        }
        effects.extend(dying);
        effects.extend(at_end);
        let effects = match unless {
            Some(unless) => vec![self.unless_wrap(unless, effects, &mut sub, chain)?],
            None => effects,
        };
        chain.effects.extend(effects);
        self.chain_sub_ability(sub, chain)
    }

    fn chain_sub_ability(&mut self, sub: Option<String>, chain: &mut Chain) -> Option<()> {
        let Some(name) = sub else {
            return Some(());
        };
        let Some(body) = self.svars.get(&name).cloned() else {
            return self.deny(format!("`SubAbility$ {name}` names no SVar"));
        };
        self.chain(&body, chain)
    }

    // The requirement and the effect name the target differently when it
    // is a player: the wizard resolves `AnyPlayer`/`AnyOpponent` into the
    // spell's chosen player, and the effect then reads it back as
    // `PlayerRel::Chosen`. Handing the *requirement* to the effect
    // instead is how a burn spell ends up dealing damage to nothing at
    // all: `DealDamage` looks for an object target and finds none.
    fn effect_target(chain: &Chain) -> Option<String> {
        match chain
            .second_target
            .as_ref()
            .or(chain.target.as_ref())
            .map(String::as_str)
        {
            Some("TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent") => {
                Some("TargetSpec::Player(PlayerRel::Chosen)".to_string())
            }
            Some(other) => Some(other.to_string()),
            None => None,
        }
    }

    fn wrap_discarded_artifact_condition(
        p: &mut Params,
        effects: &[String],
    ) -> Option<Vec<String>> {
        p.claim_label("ConditionDescription", "ConditionPresent");
        if !p.exhausted() {
            return None;
        }
        Some(vec![format!(
            "Effect::IfCondition {{ condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::DiscardedMatching(&Filter::ARTIFACT)), 1), then: &[{}], otherwise: &[] }}",
            effects.join(", ")
        )])
    }

    fn wrap_line_condition(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        effects: Vec<String>,
    ) -> Option<Vec<String>> {
        if api == "Untap"
            && p.take("ConditionDefined").as_deref() == Some("Remembered")
            && p.take("ConditionPresent").as_deref() == Some("Card.Artifact")
            && p.take("ConditionCompare").as_deref() == Some("EQ1")
        {
            return Self::wrap_discarded_artifact_condition(p, &effects);
        }
        if p.exhausted() {
            return Some(effects);
        }
        if !p.has("ConditionPresent") {
            if p.peek("Condition") == Some("Threshold") {
                p.take("Condition");
                p.take("ConditionDescription");
                if !p.exhausted() {
                    if let Some(key) = p.first_key() {
                        self.note(format!("unclaimed parameter `{api}.{key}`"));
                    }
                    return None;
                }
                return Some(vec![format!(
                    "Effect::IfCondition {{ condition: Condition::GraveyardCountAtLeast(7), then: &[{}], otherwise: &[] }}",
                    effects.join(", ")
                )]);
            }
            if let (Some(var), Some(cmp), Some(sub_name)) = (
                p.peek("ConditionCheckSVar").map(str::to_string),
                p.peek("ConditionSVarCompare").map(str::to_string),
                sub.clone(),
            ) && matches!(cmp.trim(), "LE6" | "LT7")
                && self.svars.get(var.trim()).map(|s| s.trim())
                    == Some("Count$ValidGraveyard Card.YouOwn")
            {
                p.take("ConditionCheckSVar");
                p.take("ConditionSVarCompare");
                p.take("ConditionDescription");
                if !p.exhausted() {
                    if let Some(key) = p.first_key() {
                        self.note(format!("unclaimed parameter `{api}.{key}`"));
                    }
                    return None;
                }
                let sub_body = self.svars.get(sub_name.trim())?.clone();
                let (sub_api, mut sp) = Params::parse(&sub_body)?;
                sp.drop_prose();
                if sp.has("SubAbility") || sp.has("ValidTgts") {
                    return None;
                }
                let sub_threshold = if sp.peek("Condition") == Some("Threshold") {
                    sp.take("Condition");
                    true
                } else if sp.peek("ConditionCheckSVar").map(str::trim) == Some(var.trim())
                    && sp.peek("ConditionSVarCompare").map(str::trim) == Some("GE7")
                {
                    sp.take("ConditionCheckSVar");
                    sp.take("ConditionSVarCompare");
                    true
                } else {
                    false
                };
                if !sub_threshold {
                    return None;
                }
                sp.take("ConditionDescription");
                let then_effects = self.effect_of(&sub_api, &mut sp, target, false, false)?;
                if !sp.exhausted() || then_effects.is_empty() {
                    return None;
                }
                *sub = None;
                return Some(vec![format!(
                    "Effect::IfCondition {{ condition: Condition::GraveyardCountAtLeast(7), then: &[{}], otherwise: &[{}] }}",
                    then_effects.join(", "),
                    effects.join(", ")
                )]);
            }
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{api}.{key}`"));
            }
            return None;
        }
        p.claim_label("ConditionDescription", "ConditionPresent");
        if let Some(cmp) = p.take("ConditionCompare") {
            if p.has("PresentCompare") {
                return self.deny("both `ConditionCompare$` and `PresentCompare$`".to_string());
            }
            p.entries.push(("PresentCompare".to_string(), cmp));
        }
        let valid = p.take("ConditionPresent").expect("checked above");
        let cond = self.present_clause(p, &valid)?;
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{api}.{key}`"));
            }
            return None;
        }
        Some(vec![format!(
            "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
            effects.join(", ")
        )])
    }

    /// Register a fresh target declaration without merging independent instances.
    #[allow(clippy::too_many_lines)]
    fn chain_target(
        &mut self,
        api: &str,
        p: &mut Params,
        valid: &str,
        chain: &mut Chain,
    ) -> Option<()> {
        // A card in a graveyard is a different kind of target from a
        // permanent (CR 115.1 names both, and they are chosen from
        // different zones), so the zone the line moves *from* decides
        // the spec before the valid-string is read.
        let spec = if api == "ChangeZone" && p.peek("Origin") == Some("Graveyard") {
            let defined_ctrl = p.take("TargetsWithDefinedController");
            self.graveyard_target(valid, defined_ctrl.as_deref())
        } else {
            // "Target spell or permanent" (the Laces): the stack is a
            // zone a target may be chosen in only where the line says
            // so. The battlefield alone is what every target already
            // is.
            match p.take("TgtZone").as_deref() {
                None | Some("Battlefield") => {
                    if api == "Counter" && p.peek("TargetType") == Some("Activated") {
                        self.activated_ability_target(valid)
                    } else if chain.target.is_some()
                        && chain.target_unique
                        && p.peek("TargetUnique") == Some("True")
                        && api != "Counter"
                    {
                        p.take("TargetUnique");
                        let expr = self.filter_expr(valid)?;
                        let another =
                            format!("Filter::And(&[{expr}, Filter::Not(&Filter::Reference)])");
                        let name = self.body.filter_static("TARGET", &another);
                        Some(format!("TargetSpec::Object(&{name})"))
                    } else {
                        self.target_spec(valid, api)
                    }
                }
                Some("Stack,Battlefield" | "Battlefield,Stack") => {
                    self.spell_or_permanent_target(valid)
                }
                Some(zone) => return self.deny(format!("a target in `TgtZone$ {zone}`")),
            }
        };
        let Some(spec) = spec else {
            return self.deny(format!("target `{valid}`"));
        };
        if chain.target.is_some() {
            // A fresh ValidTgts declares another target instance even
            // when its restriction is identical. Never alias it to the
            // earlier list. Only a terminal fixed pump has explicit
            // second-slot semantics in this reader so far.
            if chain.second_target.is_some()
                || api != "Pump"
                || p.peek("SubAbility").is_some()
                || p.peek("Defined").is_some_and(|v| v != "Targeted")
                || ["UnlessCost", "ReplaceDyingDefined", "AtEOT"]
                    .iter()
                    .any(|key| p.peek(key).is_some())
                || ["NumAtt", "NumDef"].iter().any(|key| {
                    p.peek(key)
                        .is_some_and(|value| value.parse::<i16>().is_err())
                })
                || !spec.starts_with("TargetSpec::Object(")
            {
                return self.deny("unsupported independent target in one chain".to_string());
            }
            chain.second_target = Some(spec);
        } else {
            if p.peek("TargetUnique") == Some("True")
                && p.peek("SubAbility")
                    .and_then(|sub| self.svars.get(sub.trim()))
                    .and_then(|body| Params::parse(body))
                    .is_some_and(|(_, child)| {
                        child.has("ValidTgts") && child.peek("TargetUnique") == Some("True")
                    })
            {
                p.take("TargetUnique");
                chain.target_unique = true;
            }
            if self.on_a_spell
                && self.has_x
                && api == "ChangeZone"
                && p.peek("Origin") == Some("Battlefield")
                && p.peek("Destination") == Some("Hand")
                && let (Some(min), Some(max)) = (p.peek("TargetMin"), p.peek("TargetMax"))
                && min == max
                && self.svars.get(min.trim()).map(|s| s.trim()) == Some("Count$xPaid")
            {
                p.take("TargetMin");
                p.take("TargetMax");
                chain.target_is_x = true;
            } else if ((self.on_a_spell
                && ((api == "ChangeZone"
                    && p.peek("Origin") == Some("Graveyard")
                    && p.peek("Destination") == Some("Hand"))
                    || api == "Tap"))
                || api == "Pump")
                && p.peek("TargetMin") == Some("0")
                && let Some(max) = p
                    .peek("TargetMax")
                    .and_then(|s| s.trim().parse::<u8>().ok())
                && max > 0
            {
                p.take("TargetMin");
                p.take("TargetMax");
                chain.target_up_to = Some(max);
            } else if self.on_a_spell
                && api == "DealDamage"
                && let (Some(min), Some(max)) = (p.peek("TargetMin"), p.peek("TargetMax"))
                && min == max
                && let Ok(n) = min.trim().parse::<u8>()
                && n > 1
            {
                p.take("TargetMin");
                p.take("TargetMax");
                chain.target_exact = Some(n);
            }
            chain.target = Some(spec);
        }
        Some(())
    }

    /// The "unless" of a line: `UnlessCost$` with the keys that qualify it.
    ///
    /// **The payer.** `UnlessPayer$ You` is the source's controller. Absent,
    /// the reference asks the controller of the line's target (its default
    /// is `TargetedController`), which is Mana Leak's "unless its controller
    /// pays" — so an absent payer is read only on a line that targets a
    /// spell or a permanent, and refused on one that targets nothing, where
    /// that default names nobody. On a cast trigger `TriggeredActivator` is
    /// the caster and `TriggeredCardController` the spell's controller, read
    /// as `Defined$` reads them (Rhystic Study, Mystic Remora). Every other
    /// payer is refused by name: `Player` is every player at once, a price
    /// no one question can put.
    ///
    /// **The subs.** Without `UnlessResolveSubs$` the rest of the chain runs
    /// either way, which is what wrapping this line alone gives. With it the
    /// rest runs on one answer only (Power Sink's "if that player doesn't,
    /// they tap all lands…"), a shape this does not read yet.
    pub(super) fn unless(
        &mut self,
        cost: &str,
        p: &mut Params,
        target: Option<&str>,
        has_sub: bool,
    ) -> Option<Unless> {
        let switched = match p.take("UnlessSwitched").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`UnlessSwitched$ {other}`")),
        };
        let resolve_subs = match p.take("UnlessResolveSubs").as_deref() {
            None => false,
            Some("WhenNotPaid") if has_sub && !switched => true,
            Some(subs) => return self.deny(format!("`UnlessResolveSubs$ {subs}`")),
        };
        let payer = p.take("UnlessPayer");
        let targets_a_controlled_object = target.is_some_and(|t| {
            t.starts_with("TargetSpec::Spell(") || t.starts_with("TargetSpec::Object(")
        });
        let targets_a_player = target
            .is_some_and(|t| t == "TargetSpec::AnyPlayer" || t.starts_with("TargetSpec::Player("));
        let payer = match payer.as_deref() {
            Some("You") => "PlayerRel::You",
            // Paralyze's "that player may pay {4}": the enchanted
            // creature's controller (CR 303.4e names the Aura's host).
            Some("EnchantedController") => "PlayerRel::ControllerOfAttached",
            None | Some("TargetedController") if targets_a_controlled_object => {
                "PlayerRel::ControllerOfTarget"
            }
            Some("Targeted") if targets_a_player => "PlayerRel::Chosen",
            // Rhystic Study's "unless that player pays {1}": the player the
            // cast trigger's event is about, as `Defined$` reads the same
            // words (`Tx::player_rel`). Anywhere but a cast trigger they
            // name nobody this reader can see, and stay refused below.
            Some(who @ ("TriggeredActivator" | "TriggeredCardController"))
                if self.trigger_mode.as_deref() == Some("SpellCast") =>
            {
                self.player_rel(Some(who))?
            }
            Some("TriggeredCardController") if self.trigger_mode.as_deref() == Some("Drawn") => {
                self.player_rel(Some("TriggeredCardController"))?
            }
            Some("TriggeredTargetController")
                if self.trigger_mode.as_deref() == Some("DamageDone") =>
            {
                self.player_rel(Some("TriggeredTargetController"))?
            }
            Some("TriggeredDefendingPlayer")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerBlocked")
                ) =>
            {
                self.player_rel(Some("TriggeredDefendingPlayer"))?
            }
            other => {
                return self.deny(format!(
                    "an unless-cost paid by `{}`",
                    other.unwrap_or("TargetedController")
                ));
            }
        };
        let price = self.unless_price(cost.trim())?;
        Some(Unless {
            payer,
            price,
            switched,
            resolve_subs,
        })
    }

    /// An `UnlessCost$` value as a [`Price`].
    ///
    /// Read by [`Tx::cost_pieces`], the reader an activation cost goes
    /// through, and then held to what the "unless" effects can carry. Plain
    /// generic mana stays an [`Amount`] (`PlayerMayPayOr`); anything with a
    /// colour in it is printed exactly (`PlayerMayPayManaOr`), where it used
    /// to be refused — a `{U}` charged as `{1}` is a card anyone could keep
    /// with a Mountain. One part the player pays by naming an object is
    /// `PlayerMayPayCostOr`. A part that needs no answer (`PayLife<2>`) is
    /// refused even though `CostPart` can hold it: that effect asks by
    /// putting up the list of what may pay, and an empty list is how a
    /// player declines — so a price nobody names an object for would
    /// decline itself every time.
    pub(super) fn unless_price(&mut self, raw: &str) -> Option<Price> {
        // "Unless its controller pays {X}" (Power Sink): the X announced
        // for the source, and only where the card says that is what X is.
        if matches!(raw, "X" | "Y" | "Z") {
            let Some(x) = self.amount_or_count(raw) else {
                return self.deny(format!("an unless-cost of `{raw}`"));
            };
            return Some(Price::Generic(x));
        }
        let (mana, parts) = self.cost_pieces(raw)?;
        match (mana.as_str(), parts.as_slice()) {
            (m, []) if !m.is_empty() => Some(match generic_mana(m) {
                Some(n) => Price::Generic(format!("Amount::Fixed({n})")),
                None => Price::Printed(m.to_string()),
            }),
            ("", [one]) if asks_for_an_object(one) => Some(Price::Part(one.clone())),
            _ => self.deny(format!("an unless-cost of `{raw}`")),
        }
    }

    /// The line's effects behind its price. A tax runs them on a refusal and
    /// takes them as one effect (a `Sequence` when there are several); a
    /// switched price runs them on a payment and takes the list.
    pub(super) fn unless_wrap(
        &mut self,
        unless: Unless,
        mut effects: Vec<String>,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<String> {
        let Unless {
            payer,
            price,
            switched,
            resolve_subs,
        } = unless;
        if resolve_subs {
            let name = sub.take()?;
            let Some(body) = self.svars.get(&name).cloned() else {
                return self.deny(format!("`SubAbility$ {name}` names no SVar"));
            };
            let mut sub_chain = Chain {
                target: chain.target.clone(),
                second_target: chain.second_target.clone(),
                ..Chain::default()
            };
            self.chain(&body, &mut sub_chain)?;
            if sub_chain.target != chain.target || sub_chain.second_target != chain.second_target {
                return self.deny("`UnlessResolveSubs$ WhenNotPaid` with a new target".to_string());
            }
            chain.could_add_mana |= sub_chain.could_add_mana;
            chain.moves_library |= sub_chain.moves_library;
            effects.extend(sub_chain.effects);
        }
        let one = match effects.as_slice() {
            [] => return self.deny("an unless-cost on a line with no effect".to_string()),
            [one] => one.clone(),
            many => format!("Effect::Sequence(&[{}])", many.join(", ")),
        };
        let list = effects.join(", ");
        Some(match (price, switched) {
            (Price::Generic(mana), false) => format!(
                "Effect::PlayerMayPayOr {{ player: {payer}, mana: {mana}, effect: &{one} }}"
            ),
            (Price::Generic(mana), true) => format!(
                "Effect::PlayerMayPayThen {{ player: {payer}, mana: {mana}, effects: &[{list}] }}"
            ),
            (Price::Printed(cost), false) => format!(
                "Effect::PlayerMayPayManaOr {{ player: {payer}, cost: mana!(\"{cost}\"), \
                 effect: &{one} }}"
            ),
            (Price::Printed(cost), true) => format!(
                "Effect::PlayerMayPayManaThen {{ player: {payer}, cost: mana!(\"{cost}\"), \
                 effects: &[{list}] }}"
            ),
            (Price::Part(part), false) => format!(
                "Effect::PlayerMayPayCostOr {{ player: {payer}, cost: &CostPart::{part}, \
                 effect: &{one} }}"
            ),
            // "You may <sacrifice a creature>. If you do, …" is a price no
            // effect here takes on the paying answer.
            (Price::Part(part), true) => {
                return self.deny(format!("a switched unless-cost of `{part}`"));
            }
        })
    }

    /// One effect API as the `Effect` expressions it stands for.
    ///
    /// Every parameter a rule reads is *taken* from `p`; the caller then
    /// refuses the card if anything is left, which is what stops an ignored
    /// `NoRegen$ True` from generating a card that does the wrong thing.
    // One arm per reference API, and the reasons a reading is what it is
    // live beside the arm that makes it.
    #[allow(clippy::too_many_lines)]
    pub(super) fn effect_of(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        // What the effects below aim at when they take a target. `target` is
        // `None` when the chain declared none at all, which is a different
        // question — `Animate` needs to know, because `Filter::This` binds
        // to the first target if there is one and to the source if not.
        let aimed = target.unwrap_or("TargetSpec::AnyPlayer");
        Some(match api {
            "NameCard" => {
                if p.take("Defined").as_deref() != Some("You") {
                    return None;
                }
                vec![
                    "Effect::ChooseCardName { player: PlayerRel::You, types: TypeSet::EMPTY, remember: false }"
                        .to_string(),
                ]
            }
            "DealDamage" => {
                let n = self.amount_or_count(&p.take("NumDmg")?)?;
                let may = match p.take("OptionalDecider").as_deref() {
                    None => false,
                    Some("You") => true,
                    Some(_) => return None,
                };
                // "Deals 4 damage to any target and 2 damage to you"
                // (Psionic Blast): the reference gathers both into one
                // simultaneous event and deals it at `DamageResolve`. The
                // engine journals one `DamageDealt` per recipient either
                // way, as it does for `DealDamageEach`, and nothing checks
                // state-based actions between two effects of one
                // resolution, so the two in sequence are the same event.
                if p.take("DamageMap").is_some_and(|v| v != "True") {
                    return None;
                }
                let turn_start_hand = match p.peek("Defined") {
                    Some("You.CardsInHandAtBeginningOfTurn") => Some(true),
                    Some("You.!CardsInHandAtBeginningOfTurn") => Some(false),
                    _ => None,
                };
                if turn_start_hand.is_some() {
                    p.take("Defined");
                }
                let to = if turn_start_hand.is_some() {
                    "TargetSpec::Player(PlayerRel::You)".to_string()
                } else {
                    match p.take("Defined").as_deref() {
                        None => aimed.to_string(),
                        Some(
                            "TriggeredAttackerLKICopy"
                            | "TriggeredBlockerLKICopy"
                            | "TriggeredAttackerLKICopy,TriggeredBlockerLKICopy",
                        ) if matches!(self.trigger_mode.as_deref(), Some("Attacks" | "Blocks")) => {
                            "TargetSpec::EventObject".to_string()
                        }
                        // "Deals 1 damage to that player" and every other
                        // player the line names without targeting one.
                        Some(who) => format!(
                            "TargetSpec::Player({})",
                            self.player_of_line(Some(who), target, false)?
                        ),
                    }
                };
                let dmg = format!("Effect::DealDamage {{ amount: {n}, target: {to} }}");
                let dmg = if turn_start_hand == Some(true) {
                    format!(
                        "Effect::IfCondition {{ condition: Condition::Not(&Condition::HandSizeAtTurnStartAtMost(0)), then: &[{dmg}], otherwise: &[] }}"
                    )
                } else {
                    dmg
                };
                p.claim_label("ConditionDescription", "ConditionPresent");
                let condition = match (
                    p.take("ConditionDefined").as_deref(),
                    p.take("ConditionPresent").as_deref(),
                    p.take("ConditionCompare").as_deref(),
                ) {
                    (None, None, None) => None,
                    (Some("Targeted"), Some("Land.Basic"), Some("EQ0")) => {
                        Some("Filter::NONBASIC_LAND")
                    }
                    _ => return None,
                };
                let dmg = if let Some(filter) = condition {
                    format!("Effect::IfTargetMatches {{ filter: &{filter}, then: &[{dmg}] }}")
                } else {
                    dmg
                };
                if may {
                    vec![format!("Effect::MayDo {{ effects: &[{dmg}] }}")]
                } else {
                    vec![dmg]
                }
            }
            // A number, or the X the player announced (Stream of Life's
            // "target player gains X life"): [`amount`] asks both of its
            // questions of an `X`, so a count spelled with the same letter
            // is still refused.
            "GainLife" => {
                let n = self.amount_or_count(&p.take("LifeAmount")?)?;
                match (
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?,
                    n.strip_prefix("Amount::Fixed(")
                        .and_then(|r| r.strip_suffix(')')),
                ) {
                    ("PlayerRel::You", Some(fixed)) => vec![format!("Effect::gain_life({fixed})")],
                    ("PlayerRel::You", None) => vec![format!("Effect::GainLife {{ amount: {n} }}")],
                    (who, _) => vec![format!("Effect::GainLifeFor {{ amount: {n}, who: {who} }}")],
                }
            }
            "LoseLife" => {
                let n = self.amount_or_count(&p.take("LifeAmount")?)?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::LoseLife {{ amount: {n}, target: {who} }}")]
            }
            "Dig" => {
                if target.is_some()
                    || p.take("DigNum").as_deref() != Some("1")
                    || p.take("Reveal").as_deref() != Some("True")
                    || p.take("ChangeNum").as_deref() != Some("All")
                    || p.take("DestinationZone2").as_deref() != Some("Graveyard")
                {
                    return None;
                }
                let valid = p.take("ChangeValid")?;
                let filter = self.filter_expr(&valid)?;
                let name = self.body.filter_static("DIG", &filter);
                vec![format!(
                    "Effect::RevealTopAndSort {{ filter: &{name}, matched: SearchDest::Hand, otherwise: SearchDest::Graveyard }}"
                )]
            }
            "Draw"
                if matches!(
                    p.peek("Defined"),
                    Some("You.CardsInHandAtBeginningOfTurn" | "You.!CardsInHandAtBeginningOfTurn")
                ) =>
            {
                let had_cards =
                    p.take("Defined").as_deref() == Some("You.CardsInHandAtBeginningOfTurn");
                let n = self.amount_or_count(p.take("NumCards").as_deref().unwrap_or("1"))?;
                let draw = if let Some(fixed) = n
                    .strip_prefix("Amount::Fixed(")
                    .and_then(|rest| rest.strip_suffix(')'))
                {
                    format!("Effect::draw({fixed})")
                } else {
                    format!("Effect::DrawCards {{ amount: {n} }}")
                };
                let condition = if had_cards {
                    "Condition::Not(&Condition::HandSizeAtTurnStartAtMost(0))"
                } else {
                    "Condition::HandSizeAtTurnStartAtMost(0)"
                };
                vec![format!(
                    "Effect::IfCondition {{ condition: {condition}, then: &[{draw}], otherwise: &[] }}"
                )]
            }
            // The same two readings as `GainLife`: Braingeyser's "target
            // player draws X cards".
            "Draw" => {
                let n = self.amount_or_count(p.take("NumCards").as_deref().unwrap_or("1"))?;
                let may = match p.take("OptionalDecider").as_deref() {
                    None => false,
                    Some("You") => true,
                    Some(_) => return None,
                };
                let draw = match (
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?,
                    n.strip_prefix("Amount::Fixed(")
                        .and_then(|r| r.strip_suffix(')')),
                ) {
                    ("PlayerRel::You", Some(fixed)) => format!("Effect::draw({fixed})"),
                    ("PlayerRel::You", None) => format!("Effect::DrawCards {{ amount: {n} }}"),
                    (who, _) if !may => {
                        format!("Effect::DrawCardsFor {{ amount: {n}, who: {who} }}")
                    }
                    _ => return None,
                };
                if may {
                    vec![format!("Effect::MayDo {{ effects: &[{draw}] }}")]
                } else {
                    vec![draw]
                }
            }
            // "Target player discards a card": the discarding player
            // chooses, which is what discarding means unless the effect
            // says otherwise (CR 701.9b).
            // "Each player discards their hand" (Wheel of Fortune): every
            // card, nobody choosing.
            "Discard" if p.peek("Mode") == Some("Hand") => {
                p.take("Mode");
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::DiscardHand {{ who: {who} }}")]
            }
            "Discard" if p.peek("Mode") == Some("TgtChoose") => {
                if p.peek("Cost").is_some() {
                    if p.take("Cost").as_deref() != Some("Draw<2/You>")
                        || p.take("Mode").as_deref() != Some("TgtChoose")
                        || p.take("NumCards").as_deref() != Some("1")
                        || p.take("Defined").as_deref() != Some("You")
                    {
                        return None;
                    }
                    return Some(vec![
                        "Effect::MayDo { effects: &[Effect::draw(2), Effect::DiscardForPlayers { who: PlayerRel::You, count: 1 }] }"
                            .to_string(),
                    ]);
                }
                p.take("Mode");
                p.take("IsCurse");
                if let Some(remember) = p.take("RememberDiscarded")
                    && remember != "True"
                {
                    return None;
                }
                let raw_n = p.take("NumCards").unwrap_or_else(|| "1".to_string());
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                if let Ok(n) = raw_n.parse::<u8>() {
                    vec![format!(
                        "Effect::DiscardForPlayers {{ who: {who}, count: {n} }}"
                    )]
                } else {
                    let amount = self.amount_or_count(&raw_n)?;
                    vec![format!(
                        "Effect::DiscardAmountForPlayers {{ who: {who}, amount: {amount} }}"
                    )]
                }
            }
            "Discard" if p.peek("Mode") == Some("RevealYouChoose") => {
                p.take("Mode");
                if !targets_a_player
                    || p.take("Defined").is_some()
                    || !matches!(p.take("NumCards").as_deref(), None | Some("1"))
                {
                    return None;
                }
                let filter = match p.take("DiscardValid").as_deref() {
                    None | Some("Card") => "Filter::Any".to_string(),
                    Some(valid) => self.filter_expr(valid)?,
                };
                vec![format!("Effect::RevealHandDiscard {{ filter: &{filter} }}")]
            }
            "Discard" => {
                // Refuse other modes instead of turning a chosen discard
                // into a random one. Unconsumed qualifiers also refuse it.
                if p.take("Mode").as_deref() != Some("Random") {
                    return None;
                }
                let n = amount(
                    p.take("NumCards").as_deref().unwrap_or("1"),
                    self.svars,
                    self.has_x,
                )?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::DiscardRandom {{ who: {who}, count: {n} }}"
                )]
            }
            "Mill" => {
                let n = amount(&p.take("NumCards")?, self.svars, self.has_x)?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::Mill {{ amount: {n}, target: {who} }}")]
            }
            "PutCounter" => {
                let code = p.take("CounterType")?;
                let Some(kind) = counter_kind(&code) else {
                    return self.deny(format!("counter `{code}`"));
                };
                let n = amount(
                    p.take("CounterNum").as_deref().unwrap_or("1"),
                    self.svars,
                    self.has_x,
                )?;
                // `AddCounter` puts them on the first target, or on the
                // source when the ability has none. That is the API's own
                // default and so is the right reading of a line with no
                // `Defined$` at all — but `Defined$ Self` says the *source*,
                // and on a line that also targets those are two different
                // permanents.
                //
                // Consumptive Goo is the card that proved it: "{2}{B}{B}:
                // Target creature gets -1/-1 until end of turn. Put a +1/+1
                // counter on this creature." Read as `AddCounter` the counter
                // landed on the *target*, where it cancelled the -1/-1 it was
                // paired with exactly — so the ability resolved, charged four
                // mana and changed nothing at all that a test could see.
                // `AddCounterFilter` over `Filter::This` is the spelling that
                // names the source whatever the ability targets.
                match p.take("Defined").as_deref() {
                    Some("Self") if target.is_some() => {
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::This, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
                    None | Some("Self") => {}
                    Some(_) => return None,
                }
                vec![format!(
                    "Effect::AddCounter {{ kind: {kind}, amount: {n} }}"
                )]
            }
            "RemoveCounterAll" => {
                if target.is_some() || p.take("AllCounters").as_deref() != Some("True") {
                    return None;
                }
                let kind = counter_kind(&p.take("CounterType")?)?;
                let valid = p.take("ValidCards")?;
                let expr = self.filter_expr(&valid)?;
                let filter = self.body.filter_static("COUNTERS", &expr);
                vec![format!(
                    "Effect::RemoveAllCountersFilter {{ filter: &{filter}, kind: {kind} }}"
                )]
            }
            "Cleanup" => {
                if p.take("ClearRemembered").as_deref() != Some("True") || !p.exhausted() {
                    return None;
                }
                vec![]
            }
            "RemoveCounter" => {
                let code = p.take("CounterType")?;
                let Some(kind) = counter_kind(&code) else {
                    return self.deny(format!("counter `{code}`"));
                };
                let all = p.peek("CounterNum") == Some("All");
                let n = if all {
                    p.take("CounterNum");
                    if p.take("RememberRemoved").as_deref() != Some("True") {
                        return None;
                    }
                    0
                } else {
                    plain_number(p.take("CounterNum").as_deref().unwrap_or("1"), self.svars)?
                        .try_into()
                        .ok()?
                };
                if !all && n == 0 {
                    return None;
                }
                match p.take("Defined").as_deref() {
                    None | Some("Self") if target.is_none() => {}
                    _ => return None,
                }
                if all {
                    vec![format!("Effect::RemoveAllCountersSelf {{ kind: {kind} }}")]
                } else {
                    vec![format!(
                        "Effect::RemoveCounterSelf {{ kind: {kind}, n: {n} }}"
                    )]
                }
            }
            "Scry" => {
                let n = plain_number(p.take("ScryNum").as_deref().unwrap_or("1"), self.svars)?;
                vec![format!("Effect::scry({n})")]
            }
            "Surveil" => {
                // `Amount$` and never `Defined$`: not one of the reference
                // corpus's 228 surveil lines names a player, because a
                // surveil is the controller's own library by construction
                // (CR 701.25a). 224 of them write `Amount$` and 219 of those
                // write a plain number; the rest announce an `X`, and the
                // constructor only fits the first kind.
                let raw = p.take("Amount")?;
                if let Some(n) = plain_number(&raw, self.svars) {
                    vec![format!("Effect::surveil({n})")]
                } else {
                    let n = amount(&raw, self.svars, self.has_x)?;
                    vec![format!("Effect::Surveil {{ amount: {n} }}")]
                }
            }
            "Mana" => self.mana_effect(p)?,
            "Branch" => {
                let var = p.take("BranchConditionSVar")?;
                let cmp = p
                    .take("BranchConditionSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                let cond = match (self.svars.get(var.trim()).map(|s| s.trim()), cmp.trim()) {
                    (Some("Count$ValidGraveyard Card.YouOwn"), "GE7") => {
                        "Condition::GraveyardCountAtLeast(7)"
                    }
                    _ => return None,
                };
                let true_sub = p.take("TrueSubAbility")?;
                let false_sub = p.take("FalseSubAbility")?;
                let true_body = self.svars.get(true_sub.trim())?.clone();
                let false_body = self.svars.get(false_sub.trim())?.clone();
                let mut then_chain = Chain {
                    target: target.map(str::to_string),
                    ..Chain::default()
                };
                let mut else_chain = Chain {
                    target: target.map(str::to_string),
                    ..Chain::default()
                };
                self.chain(&true_body, &mut then_chain)?;
                self.chain(&false_body, &mut else_chain)?;
                if then_chain.effects.is_empty() || else_chain.effects.is_empty() {
                    return None;
                }
                vec![format!(
                    "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[{}] }}",
                    then_chain.effects.join(", "),
                    else_chain.effects.join(", ")
                )]
            }
            "DelayedTrigger" => vec![self.delayed_trigger(p, target, targets_here)?],
            "Destroy" => {
                // `NoRegen$ True` is **read** and no longer merely consumed.
                // It was vacuous while this engine had no regeneration at
                // all — one door, and every card printing "it can't be
                // regenerated" was right for free — and the day a shield
                // existed the two doors stopped being the same function
                // (CR 701.19c). Any other value is a third thing the DSL
                // cannot say, so it refuses.
                let no_regen = match p.take("NoRegen").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let verb = if no_regen {
                    "destroy_no_regen"
                } else {
                    "destroy"
                };
                // "Destroy that creature" in a delayed trigger's body: the
                // object it remembers ([`Tx::delayed_trigger`]). Anywhere
                // else the key is left for the unclaimed check to refuse.
                let remembered = (self.in_delayed
                    && target.is_none()
                    && matches!(
                        p.peek("Defined"),
                        Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
                    ))
                    || (matches!(
                        self.trigger_mode.as_deref(),
                        Some("DamageDone" | "DamageDoneOnce")
                    ) && target.is_none()
                        && matches!(
                            p.peek("Defined"),
                            Some("TriggeredTargetLKICopy" | "TriggeredTarget")
                        ))
                    || (self.trigger_mode.as_deref() == Some("ChangesZone")
                        && target.is_none()
                        && matches!(
                            p.peek("Defined"),
                            Some("TriggeredCardLKICopy" | "TriggeredCard")
                        ))
                    || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                        && target.is_none()
                        && p.peek("Defined") == Some("TriggeredBlockerLKICopy"));
                if remembered {
                    p.take("Defined");
                    vec![format!("Effect::{verb}(TargetSpec::EventObject)")]
                } else if !self.on_a_spell && target.is_none() && p.peek("Defined") == Some("Self")
                {
                    p.take("Defined");
                    vec![format!("Effect::{verb}(TargetSpec::ThisObject)")]
                } else {
                    if !targets_here && target.is_some() && p.peek("Defined") == Some("Targeted") {
                        p.take("Defined");
                    }
                    vec![format!("Effect::{verb}({aimed})")]
                }
            }
            "SacrificeAll" => {
                if targets_here {
                    return None;
                }
                let valid = p.take("ValidCards")?;
                let filter = self.filter_expr(&valid)?;
                if !self.on_a_spell
                    && (filter.contains("Filter::CmcAtMostX")
                        || filter.contains("Filter::CmcExactlyX"))
                {
                    return None;
                }
                let name = self.body.filter_static("SWEEP", &filter);
                vec![format!("Effect::SacrificeAll {{ filter: &{name} }}")]
            }
            // "Destroy all lands", "destroy all creatures. They can't be
            // regenerated": every permanent the valid-string names, none of
            // them a target (so an ability that also targets is refused
            // rather than read as a sweep of its target). `NoRegen$` is the
            // same two doors as the single destroy above (CR 701.19c).
            "DestroyAll" => {
                if target.is_some() {
                    return None;
                }
                let valid = p.take("ValidCards")?;
                let no_regen = p.take("NoRegen");
                if let Some(filter) = self.filter_expr(&valid) {
                    // `Filter::CmcAtMostX` and `Filter::CmcExactlyX` read
                    // `this.x_value`, which is the announced X on a spell on
                    // the stack (`SP$`) and 0 on a permanent activating `AB$`.
                    if !self.on_a_spell
                        && (filter.contains("Filter::CmcAtMostX")
                            || filter.contains("Filter::CmcExactlyX"))
                    {
                        return None;
                    }
                    let verb = match no_regen.as_deref() {
                        None => "destroy_all",
                        Some("True") => "destroy_all_no_regen",
                        Some(_) => return None,
                    };
                    vec![format!("Effect::{verb}(&{filter})")]
                } else if no_regen.is_none() {
                    let stripped: Option<Vec<&str>> = valid
                        .split(',')
                        .map(|alt| alt.trim().strip_suffix(".cmcEQX"))
                        .collect();
                    let filter = self.filter_expr(&stripped?.join(","))?;
                    let value = amount("X", self.svars, self.has_x)?;
                    vec![format!(
                        "Effect::DestroyAllWithManaValue {{ filter: &{filter}, value: {value} }}"
                    )]
                } else {
                    return None;
                }
            }
            // "X damage to each creature without flying and each player"
            // (Earthquake): `DealDamageEach` for the permanents and a
            // `DealDamage` for the players, which is the one spelling
            // `DealDamageEach` names for the second half. Nothing is
            // targeted, so an ability that also targets is refused.
            // Where the reference deals the damage `DamageMap$` gathered:
            // the `DealDamage` lines before it already did.
            "DamageResolve" => Vec::new(),
            // "Prevent the next N damage that would be dealt to any target
            // this turn" (Samite Healer; CR 615.7): a shield on what the
            // line targets, or on the player `Defined$` names. With
            // neither, nothing is shielded, and that is not a card.
            "PreventDamage" => {
                let raw_amount = p.take("Amount")?;
                let n = if self.svars.get(raw_amount.trim()).map(|d| d.trim())
                    == Some("Count$Domain")
                {
                    "Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)".to_string()
                } else {
                    amount(&raw_amount, self.svars, self.has_x)?
                };
                let to = match (p.take("Defined").as_deref(), target) {
                    (Some("Self"), None) if !self.on_a_spell => {
                        "TargetSpec::ThisObject".to_string()
                    }
                    (Some(who), None) => {
                        format!("TargetSpec::Player({})", self.player_rel(Some(who))?)
                    }
                    (None, Some(aimed)) => aimed.to_string(),
                    _ => return None,
                };
                vec![format!(
                    "Effect::PreventNextDamage {{ target: {to}, amount: {n} }}"
                )]
            }
            // "Prevent all combat damage that would be dealt this turn."
            // The bare line only: the reference narrows it with keys this
            // rule does not claim, and those refuse.
            "Fog" => vec!["Effect::PreventAllCombatDamageThisTurn".to_string()],
            "DamageAll" => {
                if targets_here {
                    return None;
                }
                let raw_dmg = p.take("NumDmg")?;
                let threshold_dmg = self
                    .svars
                    .get(raw_dmg.trim())
                    .and_then(|s| s.trim().strip_prefix("Count$Threshold."))
                    .and_then(|s| s.strip_suffix(".0"))
                    .and_then(|s| s.parse::<u32>().ok());
                let n = match threshold_dmg {
                    Some(v) => format!("Amount::Fixed({v})"),
                    None => self.amount_or_count(&raw_dmg)?,
                };
                // The printed words for the two keys below ("each creature
                // and each player"), which say nothing those keys do not.
                p.take("ValidDescription");
                let mut out = Vec::new();
                if let Some(valid) = p.take("ValidCards") {
                    let filter = self.filter_expr(&valid)?;
                    let filter = self.body.filter_static("EACH", &filter);
                    out.push(format!(
                        "Effect::DealDamageEach {{ amount: {n}, filter: &{filter} }}"
                    ));
                }
                if let Some(players) = p.take("ValidPlayers") {
                    let who = self.player_rel(Some(&players))?;
                    out.push(format!(
                        "Effect::DealDamage {{ amount: {n}, target: TargetSpec::Player({who}) }}"
                    ));
                }
                if out.is_empty() {
                    return None;
                }
                if threshold_dmg.is_some() {
                    vec![format!(
                        "Effect::IfCondition {{ condition: Condition::GraveyardCountAtLeast(7), then: &[{}], otherwise: &[] }}",
                        out.join(", ")
                    )]
                } else {
                    out
                }
            }
            "Regenerate" => {
                // Bare `AB$ Regenerate` is "regenerate CARDNAME" — 181 of
                // the reference's 275 lines writing this API name no target
                // at all — so the absent target is the source and not a
                // missing one, the same reading `Untap` makes two arms
                // below. `Defined$` names something else entirely (the
                // enchanted creature, a remembered object; 33 lines) and is
                // refused rather than guessed at, which leaves the 62 that
                // carry a `ValidTgts$` as the targeted half.
                match p.take("Defined").as_deref() {
                    None => {}
                    Some("Self") if target.is_none() && !self.on_a_spell => {}
                    // "Regenerate enchanted creature" (Regeneration): the
                    // Aura's host, which the filter binds to the source.
                    Some("Enchanted" | "Equipped") if target.is_none() => {
                        return Some(vec![
                            "Effect::RegenerateAll { filter: &Filter::AttachedToBySource }"
                                .to_string(),
                        ]);
                    }
                    Some(_) => return None,
                }
                match target {
                    Some(t) => vec![format!("Effect::regenerate({t})")],
                    None => vec!["Effect::regenerate(TargetSpec::ThisObject)".to_string()],
                }
            }
            "ControlPlayer" => {
                if target != Some("TargetSpec::Player(PlayerRel::Chosen)") {
                    return None;
                }
                vec!["Effect::ControlPlayerNextTurn { player: PlayerRel::Chosen }".to_string()]
            }
            "Play" => {
                if target.is_some()
                    || p.take("Valid").as_deref() != Some("Card.IsImprinted+ExiledWithSource")
                    || p.take("ValidZone").as_deref() != Some("Exile")
                    || p.take("Amount").as_deref() != Some("All")
                    || p.take("WithoutManaCost").as_deref() != Some("True")
                    || p.take("ValidSA").as_deref() != Some("Spell")
                    || p.take("Optional").as_deref() != Some("True")
                    || p.take("CopyCard").as_deref() != Some("True")
                {
                    return None;
                }
                vec!["Effect::MayCopyCastLinkedExile".to_string()]
            }
            "Token" => self.token_effect(p, target, targets_a_player)?,
            "Investigate" => self.investigate_effect(p, targets_a_player)?,
            "Animate" => self.animate_effect(p, target)?,
            "Debuff" => self.debuff_effect(p, target)?,
            "AnimateAll"
                if target.is_none()
                    && p.peek("ValidCards") == Some("Creature.OppCtrl")
                    && p.peek("RemoveKeywords") == Some("Hexproof & Shroud")
                    && p.peek("CantHaveKeyword") == Some("Hexproof & Shroud") =>
            {
                p.take("ValidCards");
                p.take("RemoveKeywords");
                p.take("CantHaveKeyword");
                vec!["Effect::continuous(&Filter::OPPONENT_CREATURE, Modifier::CantHaveKeywords(KeywordSet::HEXPROOF.union(KeywordSet::SHROUD)), Duration::UntilEndOfTurn)".to_string()]
            }
            "AnimateAll" => self.animate_all_effect(p, target)?,
            "Pump" if p.peek("KW") == Some("Protection from everything") => {
                self.everything_protection(p)?
            }
            "Phases" => self.phase_your_permanents(p)?,
            "Pump" => self.pump_effect(p, aimed, false)?,
            "PumpAll" => self.pump_all_effect(p, target)?,
            "Effect" => self.static_effect(p, target)?,
            "GainControl" => self.gain_control_effect(p, target, targets_here, targets_a_player)?,
            "Shuffle" => {
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::ShuffleLibrary {{ who: {who} }}")]
            }
            "ChangeZone" => self.change_zone(p, target, targets_here)?,
            "ChangeZoneAll" => self.shuffle_into_library(p, target, targets_a_player)?,
            // "Look at the top three cards of target player's library and
            // put them back in any order. You may have that player shuffle"
            // (Natural Selection). The ability's controller looks and
            // decides; a count the player announced is refused.
            "RearrangeTopOfLibrary" => {
                let count: u8 = p.take("NumCards")?.parse().ok()?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                let mut effects = vec![if who == "PlayerRel::You" {
                    format!("Effect::ReorderTopLibrary {{ count: {count} }}")
                } else {
                    format!("Effect::ReorderTopLibraryOf {{ who: {who}, count: {count} }}")
                }];
                match p.take("MayShuffle").as_deref() {
                    None => {}
                    Some("True") => effects.push(format!(
                        "Effect::MayDo {{ effects: &[Effect::ShuffleLibrary {{ who: {who} }}] }}"
                    )),
                    Some(_) => return None,
                }
                effects
            }
            "Sacrifice" => self.sacrifice_effect(p, target, targets_a_player)?,
            // "Tap enchanted creature" (Paralyze): the host, and not a
            // target.
            // "Tap all lands target player controls" (Mana Short): the
            // permanents of the players `Defined$` names, or of the line's
            // own player target. Without either it is every matching
            // permanent, `TapAll`, and the line must target nothing, for
            // a target it never uses is not a card.
            "TapAll" => {
                let filter = self.filter_expr(&p.take("ValidCards")?)?;
                let defined = p.take("Defined");
                if defined.is_none() && !targets_a_player {
                    if target.is_some() {
                        return None;
                    }
                    return Some(vec![format!("Effect::TapAll {{ filter: &{filter} }}")]);
                }
                let who = self.player_of_line(defined.as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::TapAllOf {{ who: {who}, filter: &{filter} }}"
                )]
            }
            // "That player loses all unspent mana" (CR 106.4).
            "DrainMana" => {
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::LoseUnspentMana {{ who: {who} }}")]
            }
            // "You may tap or untap target artifact, creature, or land"
            // (Twiddle): one of the two choices always does nothing, so
            // the choice is a yes or a no (`Effect::ToggleTapTarget`).
            "TapOrUntap" if target.is_some() && p.take("Defined").is_none() => {
                vec!["Effect::MayDo { effects: &[Effect::ToggleTapTarget] }".to_string()]
            }
            "Tap" => match p.take("Defined").as_deref() {
                None => vec!["Effect::TapTarget".to_string()],
                Some("Enchanted" | "Equipped") if target.is_none() => {
                    vec!["Effect::TapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                Some(_) => return None,
            },
            // `AB$ Untap` with no `ValidTgts$` is the source, not a target
            // — Basalt Monolith's "{3}: Untap this artifact". Read as
            // `UntapTarget` it would walk an empty `res.targets` and untap
            // nothing at all, which is a card that compiles, claims
            // `Implemented` and does nothing. Nothing in the pool was
            // written that way (asserted in `untap_tests`); it was one
            // reference script away from being.
            "Untap" => match (p.take("Defined").as_deref(), target) {
                (None, Some(_)) => vec!["Effect::UntapTarget".to_string()],
                (None, None) | (Some("Self"), _) => vec!["Effect::UntapSelf".to_string()],
                // "Untap enchanted creature" (Instill Energy).
                (Some("Enchanted" | "Equipped"), None) => {
                    vec!["Effect::UntapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                _ => return None,
            },
            "RemoveFromCombat" => match (p.take("Defined").as_deref(), target) {
                (None | Some("Self"), None) => {
                    vec!["Effect::RemoveTargetFromCombat { unblock: false }".to_string()]
                }
                _ => return None,
            },
            // A fixed single skip owed by the resolving ability's controller.
            // Other players and counts are not represented by SkipNextTurn.
            "SkipTurn" => {
                if target.is_some()
                    || targets_a_player
                    || p.take("NumTurns").as_deref() != Some("1")
                    || p.take("Defined").as_deref() != Some("You")
                {
                    return None;
                }
                vec!["Effect::SkipNextTurn".to_string()]
            }
            // "Take an extra turn after this one" (Time Walk; CR 500.7).
            // One turn, and yours: another count or another player is a
            // different sentence.
            "AddTurn" => {
                if p.take("NumTurns").as_deref() != Some("1")
                    || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                vec!["Effect::TakeExtraTurn".to_string()]
            }
            "Protection" => {
                match (p.take("Defined").as_deref(), target) {
                    (None | Some("Targeted"), Some(_)) => {}
                    (None | Some("Self"), None) if !self.on_a_spell => {}
                    _ => return None,
                }
                let duration = match p.take("Duration").as_deref() {
                    None => "Duration::UntilEndOfTurn",
                    Some("Permanent") => "Duration::Indefinitely",
                    Some(_) => return None,
                };
                match p.take("Gains").as_deref() {
                    Some("Choice") => {
                        if p.take("Choices").as_deref() != Some("AnyColor") {
                            return None;
                        }
                        vec![format!(
                            "Effect::ProtectionFromChosenColor {{ duration: {duration} }}"
                        )]
                    }
                    Some(gains) => {
                        let mut out = Vec::new();
                        for part in gains.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                            let from =
                                self.protection_filter(&format!("Protection from {part}"))?;
                            out.push(format!(
                                "Effect::continuous(&Filter::This, Modifier::ProtectionFrom(&{from}), {duration})"
                            ));
                        }
                        if out.is_empty() {
                            return None;
                        }
                        out
                    }
                    None => return None,
                }
            }
            "Counter" => {
                let dest = p.take("Destination");
                match (dest.as_deref(), p.take("TargetType").as_deref()) {
                    (None | Some("Graveyard"), Some("Spell")) => {
                        vec!["Effect::CounterTargetSpell".to_string()]
                    }
                    (Some("Exile"), Some("Spell")) => {
                        vec!["Effect::CounterTargetSpellToExile".to_string()]
                    }
                    (None | Some("Graveyard"), Some("Activated")) => {
                        vec!["Effect::CounterTargetAbility".to_string()]
                    }
                    _ => return None,
                }
            }
            "WinsGame" => {
                if target.is_some() || p.take("Defined").as_deref() != Some("You") {
                    return None;
                }
                vec!["Effect::WinGame".to_string()]
            }
            _ => return None,
        })
    }
}
