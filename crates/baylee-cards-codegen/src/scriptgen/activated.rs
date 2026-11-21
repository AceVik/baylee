//! Reading activated abilities, spells, charms and their conditions.

use super::{
    Bound, Chain, Params, Price, Tx, asks_for_an_object, cost_parts, count_bound, counter_kind,
    hybrid_pair, on_the_battlefield, printed_mana,
};

/// Optional payment parsed off a trigger-executed `AB$ ... | Cost$ ...` line.
pub(super) enum OptionalTriggerPrice {
    Generic(String),
    Printed(String),
    Life(String),
    CostPart(String),
    SacrificeN { filter: String, count: u8 },
}

/// `(mode_expr, first_target_spec, has_second_target, could_add_mana)`.
pub(super) type CharmMode = (String, Option<String>, bool, bool);

/// Components of a parsed `AB$` activated ability line or granted `AddAbility$` `SVar`.
pub(super) struct ParsedActivated {
    pub cost: String,
    pub effects: String,
    pub mana_ability: bool,
    pub extras: String,
    pub ability_expr: String,
    pub alternate_ability_expr: Option<String>,
    pub non_battlefield_zone: bool,
    pub simple_target: Option<String>,
}

struct ExtractedAbilitySpec {
    cost: Option<String>,
    alternate_cost: Option<String>,
    limit: Option<String>,
    activation_zone: Option<String>,
    non_battlefield_zone: bool,
    zone_suffix: &'static str,
    any_player: bool,
    sorcery_speed: bool,
    condition: String,
    cost_reduction: Option<String>,
    chain: Chain,
}

impl Tx<'_> {
    fn activation_phase_condition(
        &self,
        probe: &mut Params,
        phases: &str,
        claimed: &mut Vec<&'static str>,
    ) -> Option<&'static str> {
        let clause = match phases {
            "BeginCombat->EndCombat" => "Condition::DuringCombat",
            "Upkeep" => "Condition::DuringStep(StepKind::Upkeep)",
            "Draw" => "Condition::DuringStep(StepKind::Draw)",
            "BeginCombat" => "Condition::DuringStep(StepKind::CombatBegin)",
            "Declare Attackers" => "Condition::DuringStep(StepKind::DeclareAttackers)",
            "Declare Blockers" => "Condition::DuringStep(StepKind::DeclareBlockers)",
            "EndCombat" => "Condition::DuringStep(StepKind::CombatEnd)",
            "End of Turn" => "Condition::DuringStep(StepKind::End)",
            "Declare Blockers->EndCombat"
                if matches!(probe.peek("ActivationAfterBlockers"), None | Some("True")) =>
            {
                if probe.take("ActivationAfterBlockers").is_some() {
                    claimed.push("ActivationAfterBlockers$");
                }
                "Condition::InCombatAfterBlockers"
            }
            "Upkeep->BeginCombat" if probe.peek("ActivationFirstCombat") == Some("True") => {
                probe.take("ActivationFirstCombat");
                claimed.push("ActivationFirstCombat$");
                "Condition::BeforeStep(StepKind::DeclareAttackers)"
            }
            "Upkeep->Declare Attackers" if probe.peek("ActivationFirstCombat") == Some("True") => {
                probe.take("ActivationFirstCombat");
                claimed.push("ActivationFirstCombat$");
                "Condition::BeforeStep(StepKind::DeclareBlockers)"
            }
            "Upkeep->Declare Blockers" if probe.peek("ActivationFirstCombat") == Some("True") => {
                probe.take("ActivationFirstCombat");
                claimed.push("ActivationFirstCombat$");
                "Condition::BeforeStep(StepKind::CombatDamage)"
            }
            "Upkeep->Main1" if probe.peek("ActivationFirstCombat") == Some("True") => {
                probe.take("ActivationFirstCombat");
                claimed.push("ActivationFirstCombat$");
                "Condition::BeforeStep(StepKind::CombatBegin)"
            }
            _ => return self.deny(format!("`ActivationPhases$ {phases}`")),
        };
        Some(clause)
    }

    /// When an ability may be activated, beyond its `IsPresent$` clause: a
    /// restriction on activating (CR 602.5) that the `condition` field says,
    /// and so only where no other clause already fills it. Hands back the
    /// keys it claimed.
    ///
    /// - `PlayerTurn$ True` / `OpponentTurn$ True`: turn-owner gating.
    /// - `ActivationPhases$`: combat phase, individual turn steps, or bounded
    ///   pre-combat/pre-attacker/post-blocker windows.
    pub(super) fn activation_restriction(
        &mut self,
        probe: &mut Params,
        is_activated: bool,
        condition: &mut String,
    ) -> Option<Vec<&'static str>> {
        let mut claimed = Vec::new();
        let mut clauses: Vec<String> = condition
            .strip_prefix(", condition = Some(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::to_string)
            .into_iter()
            .collect();
        if let Some(your_turn) = probe.take("PlayerTurn") {
            if your_turn != "True" || !is_activated {
                return self.deny(format!("`PlayerTurn$ {your_turn}` beside another clause"));
            }
            clauses.push("Condition::YourTurn".to_string());
            claimed.push("PlayerTurn$");
        }
        if let Some(opp_turn) = probe.take("OpponentTurn") {
            if opp_turn != "True"
                || !is_activated
                || clauses.iter().any(|c| c == "Condition::YourTurn")
            {
                return self.deny(format!("`OpponentTurn$ {opp_turn}` beside another clause"));
            }
            clauses.push("Condition::OpponentsTurn".to_string());
            claimed.push("OpponentTurn$");
        }
        if let Some(phases) = probe.take("ActivationPhases") {
            if !is_activated {
                return self.deny(format!("`ActivationPhases$ {phases}`"));
            }
            let clause = self.activation_phase_condition(probe, &phases, &mut claimed)?;
            clauses.push(clause.to_string());
            claimed.push("ActivationPhases$");
        }
        if let Some(act) = probe.take("Activation") {
            if !is_activated {
                return self.deny(format!("`Activation$ {act}`"));
            }
            let cond = match act.as_str() {
                "Threshold" => "Condition::GraveyardCountAtLeast(7)",
                "Delirium" => "Condition::GraveyardTypesAtLeast(4)",
                "Metalcraft" => "Condition::ControlCount(&Filter::ARTIFACT, 3)",
                "Hellbent" => "Condition::HandSizeAtMost(0)",
                "Blessing" => "Condition::CitysBlessing",
                _ => return self.deny(format!("`Activation$ {act}`")),
            };
            clauses.push(cond.to_string());
            claimed.push("Activation$");
        }
        if let Some(name) = probe.take("CheckSVar") {
            if !is_activated {
                return self
                    .deny("an entry-history activation clause beside another clause".to_string());
            }
            self.activation_check_svar(probe, &name, &mut clauses, &mut claimed)?;
        }
        *condition = match clauses.as_slice() {
            [] => String::new(),
            [one] => format!(", condition = Some({one})"),
            many => format!(", condition = Some(Condition::All(&[{}]))", many.join(", ")),
        };
        Some(claimed)
    }

    fn activation_check_svar(
        &mut self,
        probe: &mut Params,
        name: &str,
        clauses: &mut Vec<String>,
        claimed: &mut Vec<&'static str>,
    ) -> Option<()> {
        let Some(raw) = self.svars.get(name).cloned() else {
            return self.deny(format!("activation check `{name}` without an SVar"));
        };
        let cmp = probe.peek("SVarCompare").map(str::to_string);
        if let Some(alternatives) = raw.strip_prefix("Count$ThisTurnEntered_Battlefield_")
            && cmp.is_none()
        {
            if !clauses.is_empty() {
                return self
                    .deny("an entry-history activation clause beside another clause".to_string());
            }
            let mut subtypes = Vec::new();
            for alternative in alternatives.split(',') {
                let Some(subtype) = alternative.strip_suffix(".YouCtrl") else {
                    return self.deny(format!("entry-history alternative `{alternative}`"));
                };
                let Some(path) = self.cats.const_path(subtype) else {
                    return self.deny(format!("entry-history subtype `{subtype}`"));
                };
                subtypes.push(path);
            }
            clauses.push(format!(
                "Condition::EnteredSubtypeUnderYourControlThisTurn(&[{}])",
                subtypes.join(", ")
            ));
            claimed.push("CheckSVar$");
            return Some(());
        }
        let Some(first) = self.svar_condition_clause(name, cmp.as_deref().unwrap_or("GE1")) else {
            return self.deny(format!("activation check `{name}`"));
        };
        let clause = if let Some(name2) = probe.take("CheckSecondSVar") {
            let cmp2 = probe
                .take("SecondSVarCompare")
                .unwrap_or_else(|| "GE1".to_string());
            let Some(second) = self.svar_condition_clause(&name2, &cmp2) else {
                return self.deny(format!("activation check `{name2}`"));
            };
            claimed.push("CheckSecondSVar$");
            claimed.push("SecondSVarCompare$");
            format!("Condition::All(&[{first}, {second}])")
        } else {
            first
        };
        clauses.push(clause);
        claimed.push("CheckSVar$");
        if probe.take("SVarCompare").is_some() {
            claimed.push("SVarCompare$");
        }
        Some(())
    }

    /// Read the activation count separately from cost and effect emission.
    fn activation_limit(&mut self, limit: Option<&str>) -> Option<String> {
        Some(match limit {
            None => String::new(),
            Some(n) => {
                let Ok(n) = n.parse::<u8>() else {
                    // `GE4` and `X` are the corpus's other two spellings
                    // (5 scripts between them) and neither is a count
                    // this side can read: one is a *threshold* on the
                    // counters already spent, the other a number the
                    // board works out.
                    return self.deny(format!("activation limit `{n}`"));
                };
                format!(", limit = ActivationLimit::PerTurn({n})")
            }
        })
    }

    pub(super) fn parse_activated_spec(
        &mut self,
        spec: &str,
        allow_oracle_tap: bool,
        allow_alternate_cost: bool,
    ) -> Option<ParsedActivated> {
        let extracted = self.extract_ability_spec(spec, true, allow_oracle_tap)?;
        if !allow_alternate_cost && extracted.alternate_cost.is_some() {
            return self.deny("an `AlternateCost` on a granted ability".to_string());
        }
        self.build_activated_parts(&extracted)
    }

    pub(super) fn activated_or_spell(&mut self, spec: &str) -> Option<()> {
        let is_activated = spec.starts_with("AB$");
        if !is_activated
            && let Some((api, p)) = Params::parse(spec)
            && api == "DealDamage"
            && p.has("DividedAsYouChoose")
        {
            return self.divided_damage_spell(p);
        }
        if let Some((api, p)) = Params::parse(spec)
            && api == "ManaReflected"
        {
            if !is_activated {
                return self.deny("reflected Gate mana as a spell".into());
            }
            return self.gond_mana_reflected(p);
        }
        if Params::parse(spec).is_some_and(|(api, _)| api == "Charm") {
            return if is_activated {
                self.modal_activated(spec)
            } else {
                self.charm(spec)
            };
        }
        let extracted = self.extract_ability_spec(spec, is_activated, true)?;
        if is_activated {
            let built = self.build_activated_parts(&extracted)?;
            self.body.abilities.push(built.ability_expr);
            if let Some(alt) = built.alternate_ability_expr {
                self.body.abilities.push(alt);
            }
            Some(())
        } else {
            let effects = format!("&[{}]", extracted.chain.effects.join(", "));
            let second_targets = Self::second_target_req_expr(&extracted.chain)
                .map_or_default(|req| format!(", second_targets = Some({req})"));
            self.push_spell_ability(
                extracted.cost.as_deref(),
                extracted.limit.as_deref(),
                &extracted.condition,
                &effects,
                extracted.chain.target.as_deref(),
                extracted.chain.target_is_x,
                extracted.chain.target_up_to,
                extracted.chain.target_exact,
                &second_targets,
            )
        }
    }

    fn infer_single_ability_tap(&self, cost: Option<String>) -> Option<String> {
        let printed_cost = self.oracle.split(':').next().unwrap_or_default();
        let tap_before_other_costs = printed_cost
            .split(", ")
            .take_while(|part| *part != "{T}")
            .all(|part| part.starts_with('{') && part.ends_with('}') && part != "{Q}");
        if self.oracle.matches(':').count() == 1
            && printed_cost.split(", ").any(|part| part == "{T}")
            && cost
                .as_deref()
                .is_some_and(|parts| !parts.split_whitespace().any(|part| part == "T"))
        {
            cost.map(|parts| {
                if tap_before_other_costs {
                    format!("T {parts}")
                } else {
                    format!("{parts} T")
                }
            })
        } else {
            cost
        }
    }

    fn extract_ability_spec(
        &mut self,
        spec: &str,
        is_activated: bool,
        allow_oracle_tap: bool,
    ) -> Option<ExtractedAbilitySpec> {
        let Some((api, mut probe)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        if is_activated && !allow_oracle_tap && (api == "ManaReflected" || api == "Charm") {
            return self.deny(format!("a granted `{api}` ability"));
        }
        probe.drop_prose();
        let mut cost = probe.take("Cost");
        if is_activated && allow_oracle_tap {
            cost = self.infer_single_ability_tap(cost);
        }
        let alternate_cost = probe.take("AlternateCost");
        if !is_activated && alternate_cost.is_some() {
            return self.deny("an `AlternateCost` on a spell".to_string());
        }
        let limit = probe.take("ActivationLimit");
        let activator = probe.take("Activator");
        let activation_zone = probe.take("ActivationZone");
        if !is_activated && let Some(zone) = activation_zone.as_deref() {
            return self.deny(format!("a spell in `ActivationZone$ {zone}`"));
        }
        let reduce_cost = probe.take("ReduceCost");
        if !is_activated && reduce_cost.is_some() {
            return self.deny("`ReduceCost$` on a spell".to_string());
        }
        if is_activated && reduce_cost.is_some() && probe.has("ReduceAmount") {
            return self.deny("a `ReduceCost$` with `ReduceAmount$`".to_string());
        }
        let cost_reduction = match (is_activated, reduce_cost.as_deref()) {
            (true, Some(raw)) => {
                Some(self.activation_cost_reduction(raw, activation_zone.as_deref())?)
            }
            _ => None,
        };
        let any_player = is_activated && activator.as_deref() == Some("Player");
        let (non_battlefield_zone, zone_suffix) = if is_activated {
            self.activation_zone_expr(activation_zone.as_deref(), any_player, false)?
        } else {
            (false, "")
        };
        let zone_handled = is_activated && activation_zone.is_some();
        let sorcery_speed = is_activated && probe.take("SorcerySpeed").as_deref() == Some("True");
        let mut condition = self.condition(&mut probe)?;
        let present_read = !condition.is_empty();
        let restrictions = self.activation_restriction(&mut probe, is_activated, &mut condition)?;
        let mut chain = Chain::default();
        let claimed: &[&str] = if present_read {
            &[
                "Condition$",
                "IsPresent$",
                "PresentZone$",
                "PresentCompare$",
                "PresentDefined$",
            ]
        } else {
            &[]
        };
        let stripped: Vec<&str> = spec
            .split(" | ")
            .filter(|part| !part.starts_with("Cost$") && !part.starts_with("ActivationLimit$"))
            .filter(|part| alternate_cost.is_none() || !part.starts_with("AlternateCost$"))
            .filter(|part| {
                !is_activated || reduce_cost.is_none() || !part.starts_with("ReduceCost$")
            })
            .filter(|part| !(any_player && part.starts_with("Activator$")))
            .filter(|part| !(zone_handled && part.starts_with("ActivationZone$")))
            .filter(|part| !(sorcery_speed && part.starts_with("SorcerySpeed$")))
            .filter(|part| !restrictions.iter().any(|key| part.starts_with(key)))
            .filter(|part| !claimed.iter().any(|key| part.starts_with(key)))
            .collect();
        self.cost_sacrifices_self = is_activated
            && cost
                .as_deref()
                .is_some_and(|c| c.split_whitespace().any(|t| t == "Sac<1/CARDNAME>"));
        self.cost_sacrifices_other = cost.as_deref().is_some_and(|c| {
            cost_parts(c)
                .iter()
                .any(|t| t.starts_with("Sac<1/") && !t.starts_with("Sac<1/CARDNAME"))
        });
        let chain_ok = self.chain(&stripped.join(" | "), &mut chain);
        self.cost_sacrifices_self = false;
        self.cost_sacrifices_other = false;
        chain_ok?;
        if chain.effects.is_empty() {
            return self.deny("an ability that reads as no effect at all".to_string());
        }
        Some(ExtractedAbilitySpec {
            cost,
            alternate_cost,
            limit,
            activation_zone,
            non_battlefield_zone,
            zone_suffix,
            any_player,
            sorcery_speed,
            condition,
            cost_reduction,
            chain,
        })
    }

    fn activation_zone_expr(
        &self,
        activation_zone: Option<&str>,
        any_player: bool,
        modal: bool,
    ) -> Option<(bool, &'static str)> {
        match (activation_zone, any_player) {
            (None | Some("Battlefield"), false) => Some((false, "")),
            (None | Some("Battlefield"), true) => {
                Some((false, ", zone = ActivationZone::BattlefieldAnyPlayer"))
            }
            (Some("Graveyard"), false) => Some((true, ", zone = ActivationZone::Graveyard")),
            (Some("Hand"), false) => Some((true, ", zone = ActivationZone::Hand")),
            (Some("Command"), false) => Some((true, ", zone = ActivationZone::Command")),
            (Some("Battlefield,Command" | "Command,Battlefield"), false) => {
                Some((true, ", zone = ActivationZone::BattlefieldOrCommand"))
            }
            (Some(zone), true) => {
                self.deny(format!("`Activator$ Player` in `ActivationZone$ {zone}`"))
            }
            (Some(zone), false) => {
                let prefix = if modal { "a modal " } else { "an " };
                self.deny(format!(
                    "{prefix}activated ability in `ActivationZone$ {zone}`"
                ))
            }
        }
    }

    fn build_activated_parts(&mut self, spec: &ExtractedAbilitySpec) -> Option<ParsedActivated> {
        let Some(cost) = spec.cost.as_deref() else {
            return self.deny("an activated ability with no `Cost$`".to_string());
        };
        let cost = self.parse_activated_cost(cost, spec.activation_zone.as_deref())?;
        let effects = format!("&[{}]", spec.chain.effects.join(", "));
        let second_targets = Self::second_target_req_expr(&spec.chain)
            .map_or_default(|req| format!(", second_targets = Some({req})"));
        let zone = spec.zone_suffix;
        let mana_ability =
            spec.chain.could_add_mana && !spec.chain.moves_library && spec.chain.target.is_none();
        if !zone.is_empty() && mana_ability {
            return self.deny("a non-default activation zone on a mana ability".to_string());
        }
        if spec.sorcery_speed && mana_ability {
            return self.deny("`SorcerySpeed$ True` on a mana ability".to_string());
        }
        let timing = if spec.sorcery_speed {
            ", timing = ActivationTiming::SorcerySpeed"
        } else {
            ""
        };
        if spec.chain.target_is_x {
            return self.deny("`TargetMin$/TargetMax$ X` on an activated ability".to_string());
        }
        let target = spec.chain.target.as_ref().map_or_default(|t| {
            if let Some(max) = spec.chain.target_up_to {
                if max == 1 {
                    format!(", targets = Some(TargetReq::up_to_one({t}))")
                } else {
                    format!(", targets = Some(TargetReq::up_to({t}, {max}))")
                }
            } else if let Some(n) = spec.chain.target_exact {
                format!(", targets = Some(TargetReq::exactly({t}, {n}))")
            } else {
                format!(", target = Some({t})")
            }
        });
        let limit_str = self.activation_limit(spec.limit.as_deref())?;
        let condition = &spec.condition;
        let cost_reduction = spec.cost_reduction.as_deref().unwrap_or("");
        let extras =
            format!("{timing}{zone}{target}{second_targets}{limit_str}{condition}{cost_reduction}");
        let ability_expr = Self::format_activated_macro(mana_ability, &cost, &effects, &extras);
        let alternate_ability_expr = match spec.alternate_cost.as_deref() {
            Some(alt) => {
                let alt_cost = self.parse_activated_cost(alt, spec.activation_zone.as_deref())?;
                Some(Self::format_activated_macro(
                    mana_ability,
                    &alt_cost,
                    &effects,
                    &extras,
                ))
            }
            None => None,
        };
        let simple_target = if !mana_ability
            && !spec.non_battlefield_zone
            && !spec.any_player
            && !spec.sorcery_speed
            && limit_str.is_empty()
            && condition.is_empty()
            && spec.cost_reduction.is_none()
            && spec.chain.second_target.is_none()
            && spec.chain.target_up_to.is_none()
            && spec.chain.target_exact.is_none()
        {
            spec.chain.target.clone()
        } else {
            None
        };
        Some(ParsedActivated {
            cost,
            effects,
            mana_ability,
            extras,
            ability_expr,
            alternate_ability_expr,
            non_battlefield_zone: spec.non_battlefield_zone,
            simple_target,
        })
    }

    fn format_activated_macro(
        mana_ability: bool,
        cost: &str,
        effects: &str,
        extras: &str,
    ) -> String {
        match (mana_ability, cost) {
            (true, "Cost::TAP") if extras.is_empty() => format!("mana_ability!({effects})"),
            (true, _) => format!("mana_ability!({cost}, {effects}{extras})"),
            (false, _) => format!("activated!({cost}, {effects}{extras})"),
        }
    }
    // And the third of them, which this branch dropped in silence
    // until a batch of four hundred cards walked into it. A spell's
    // `Cost$` is its mana cost *plus* whatever else the card charges
    // (CR 601.2b), and only the mana half is on the face — so
    // Kaervek's Spite came out as three mana for "target player
    // loses 5 life", with "sacrifice all permanents you control and
    // discard your hand" nowhere in the card, and Crop Rotation as a
    // one-mana tutor that sacrifices no land. 215 of the 236
    // `A:SP$` lines naming a `Cost$` charge something beside mana.
    //
    // `FaceDef::mandatory_additional_costs` is where they belong and
    // the engine collects them at cast; emitting them is the next
    // step and not this one, because it would walk a dozen cards
    // into a path exactly one card in the pool has ever played
    // (Toxic Deluge's `PayLifeX`). Until then the honest answer is
    // the stub. See #52.
    //
    // The token is read here rather than through `cost_pieces`,
    // which prices an *activation* and denies what it cannot price:
    // routing a spell's cost through it would refuse `Cost$ X G` for
    // its bare `X` — mana the face already carries — and take a card
    // off the list for the one thing that is not wrong with it.
    //
    // So the question asked of each token is the narrow one: is this
    // the card's own mana cost? Everything else refuses, which is an
    // allow-list on purpose. The alternative — refusing the `<…>`
    // spelling every one of those 215 additional costs happens to
    // use — goes silent on a restriction that needs no brackets, and
    // one of those is already here: `XMin1` is "X can't be 0" and
    // not mana at all (Ertai's Meddling and four others, none of
    // them in this pool yet, all five reachable by a batch).
    #[allow(clippy::too_many_arguments)]
    fn push_spell_ability(
        &mut self,
        cost: Option<&str>,
        limit: Option<&str>,
        condition: &str,
        effects: &str,
        target: Option<&str>,
        target_is_x: bool,
        target_up_to: Option<u8>,
        target_exact: Option<u8>,
        second_targets: &str,
    ) -> Option<()> {
        if limit.is_some() {
            // A spell is cast, not activated, so there is nothing for the
            // key to restrict and dropping it quietly would be the
            // unclaimed-parameter fault one level down.
            return self.deny("`ActivationLimit$` on a spell line".to_string());
        }
        if !condition.is_empty() {
            // The same, for the same reason: `spell!` has no
            // precondition, and a restriction on casting is a rule this
            // DSL does not have (CR 601.2 has no place for one).
            return self.deny("`IsPresent$` on a spell line".to_string());
        }
        if let Some(raw) = cost {
            self.spell_mana_cost(raw)?;
        }
        let targets = target.map_or_default(|t| {
            if let Some(max) = target_up_to {
                if max == 1 {
                    format!(", targets = Some(TargetReq::up_to_one({t}))")
                } else {
                    format!(", targets = Some(TargetReq::up_to({t}, {max}))")
                }
            } else if let Some(n) = target_exact {
                format!(", targets = Some(TargetReq::exactly({t}, {n}))")
            } else {
                let req_fn = if target_is_x { "x_targets" } else { "one" };
                format!(", targets = Some(TargetReq::{req_fn}({t}))")
            }
        });
        self.body
            .abilities
            .push(format!("spell!({effects}{targets}{second_targets})"));
        Some(())
    }

    /// Validate the spell's cost tokens and record any non-mana mandatory
    /// additional costs (`FaceDef::mandatory_additional_costs`) that
    /// `cast_wizard::paid_as_a_mandatory_additional_cost` pays.
    fn spell_mana_cost(&mut self, raw: &str) -> Option<()> {
        let mut extra = Vec::new();
        for token in cost_parts(raw) {
            let is_mana = token.chars().all(|c| c.is_ascii_digit())
                || matches!(token.as_str(), "W" | "U" | "B" | "R" | "G" | "C" | "X")
                || hybrid_pair(&token).is_some();
            if !is_mana {
                extra.push(token);
            }
        }
        if extra.is_empty() {
            return Some(());
        }
        if !self.body.mandatory_additional_costs.is_empty() {
            return self
                .deny("multiple spell lines declaring mandatory additional costs".to_string());
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        if let Some(parts) = self.parse_spell_cost_parts(raw) {
            self.body.mandatory_additional_costs.extend(parts);
            return Some(());
        }
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        self.body.statics.truncate(saved_statics_len);
        let prev_alt = self.in_alternative_cost;
        self.in_alternative_cost = true;
        let parsed = self.cost_pieces(&extra.join(" "));
        self.in_alternative_cost = prev_alt;
        let Some((mana, parts)) = parsed else {
            let head = extra[0].split('<').next().unwrap_or(&extra[0]);
            return self.deny(format!(
                "`{head}` on a spell line, beside the mana the face carries"
            ));
        };
        if !mana.is_empty() || parts.is_empty() {
            let head = extra[0].split('<').next().unwrap_or(&extra[0]);
            return self.deny(format!(
                "`{head}` on a spell line, beside the mana the face carries"
            ));
        }
        let all_supported = parts.iter().all(|part| {
            part == "PayLifeX"
                || part == "PayEnergyX"
                || part == "DiscardRandom"
                || part == "DiscardHand"
                || part == "DiscardLastDrawnThisTurn"
                || part.starts_with("PayLife(")
                || part.starts_with("PayEnergy(")
                || part.starts_with("Sacrifice(")
                || part.starts_with("SacrificeX(")
                || part.starts_with("SacrificeAll(")
                || part.starts_with("ReturnToHand(")
                || part.starts_with("ExilePermanent(")
                || part.starts_with("ExileFromHand(")
                || part.starts_with("ExileFromGraveyard(")
                || part.starts_with("ExileTopOfLibrary(")
                || part.starts_with("Discard(")
                || part.starts_with("TapOther(")
                || part.starts_with("TapX(")
                || part.starts_with("TapN {")
        });
        if !all_supported {
            let head = extra[0].split('<').next().unwrap_or(&extra[0]);
            return self.deny(format!(
                "`{head}` on a spell line, beside the mana the face carries"
            ));
        }
        for part in parts {
            self.body
                .mandatory_additional_costs
                .push(format!("CostPart::{part}"));
        }
        Some(())
    }

    /// Formats a `TargetReq` from a chain's target fields.
    pub(super) fn target_req_expr(chain: &Chain) -> Option<String> {
        let t = chain.target.as_deref()?;
        Some(if let Some(max) = chain.target_up_to {
            if max == 1 {
                format!("TargetReq::up_to_one({t})")
            } else {
                format!("TargetReq::up_to({t}, {max})")
            }
        } else if let Some(n) = chain.target_exact {
            format!("TargetReq::exactly({t}, {n})")
        } else if chain.target_is_x {
            format!("TargetReq::x_targets({t})")
        } else {
            format!("TargetReq::one({t})")
        })
    }

    /// Formats a `TargetReq` from a chain's second target fields.
    pub(super) fn second_target_req_expr(chain: &Chain) -> Option<String> {
        let t = chain.second_target.as_deref()?;
        Some(if let Some(max) = chain.second_target_up_to {
            if max == 1 {
                format!("TargetReq::up_to_one({t})")
            } else {
                format!("TargetReq::up_to({t}, {max})")
            }
        } else {
            format!("TargetReq::one({t})")
        })
    }

    /// Reads the `Choices$` list of a `Charm` ability into [`CharmMode`] tuples.
    pub(super) fn read_charm_modes(&mut self, choices: &str) -> Option<Vec<CharmMode>> {
        let mut modes = Vec::new();
        for name in choices.split(',').map(str::trim) {
            let Some(line) = self.svars.get(name).cloned() else {
                return self.deny(format!("charm choice `{name}` with no `SVar`"));
            };
            let mut chain = Chain::default();
            self.chain(&line, &mut chain)?;
            if chain.effects.is_empty() {
                return self.deny("a charm mode that reads as no effect at all".to_string());
            }
            let targets = Self::target_req_expr(&chain)
                .map_or_default(|req| format!(", targets = Some({req})"));
            let has_second = chain.second_target.is_some();
            let second = Self::second_target_req_expr(&chain)
                .map_or_default(|req| format!(", second_targets = Some({req})"));
            modes.push((
                format!("mode!(&[{}]{targets}{second})", chain.effects.join(", ")),
                chain.target,
                has_second,
                chain.could_add_mana,
            ));
        }
        Some(modes)
    }

    fn is_objects_only_target(spec: &str) -> bool {
        spec.starts_with("TargetSpec::Object(")
            || spec.starts_with("TargetSpec::Spell(")
            || spec.starts_with("TargetSpec::StackOrBattlefield(")
            || spec.starts_with("TargetSpec::SpellOrAbility(")
            || spec.starts_with("TargetSpec::AbilityOnStack(")
            || spec.starts_with("TargetSpec::CardInGraveyard(")
    }

    /// "Choose one —" (CR 700.2): `SP$ Charm | Choices$ A,B` is a modal
    /// spell whose modes are the named `SVar`s, each read as the chain it
    /// is and each targeting for itself, since a target a mode names is
    /// chosen only when that mode is (CR 700.2c).
    pub(super) fn charm(&mut self, spec: &str) -> Option<()> {
        let Some((_, mut p)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        p.drop_prose();
        let Some(choices) = p.take("Choices") else {
            return self.deny("a charm with no `Choices$`".to_string());
        };
        let max: u8 = match p.take("CharmNum").as_deref() {
            None => 1,
            Some(raw) => match raw.trim().parse::<u8>() {
                Ok(n) if n > 0 => n,
                _ => return self.deny(format!("a charm choosing `{raw}`")),
            },
        };
        let min: u8 = match p.take("MinCharmNum").as_deref() {
            None => max,
            Some(raw) => match raw.trim().parse::<u8>() {
                Ok(n) if n > 0 => n,
                _ => return self.deny(format!("a charm with `MinCharmNum$ {raw}`")),
            },
        };
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `Charm.{key}`"));
            return None;
        }
        if min == 0 || min > max {
            return self.deny(format!("a charm with min {min} > max {max}"));
        }
        let parsed = self.read_charm_modes(&choices)?;
        if parsed.len() < 2 {
            return self.deny("a charm with fewer than two modes".to_string());
        }
        if usize::from(min) > parsed.len() {
            return self.deny("a charm choosing more modes than it has".to_string());
        }
        // Mirror `crates/cards/src/lints/shapes.rs::modes_fault` for multi-mode
        // spells so no generated card violates the pool invariant.
        if max > 1 {
            if parsed.len() > 8 {
                return self.deny("a multi-mode charm with more than eight modes".to_string());
            }
            if parsed.iter().any(|(_, _, second, _)| *second) {
                return self
                    .deny("a multi-mode charm with a mode declaring a second target".to_string());
            }
            let targeting: Vec<&str> = parsed
                .iter()
                .filter_map(|(_, tgt, _, _)| tgt.as_deref())
                .collect();
            if targeting.len() > 2 {
                return self
                    .deny("a multi-mode charm targeting in more than two modes".to_string());
            }
            if targeting
                .get(1)
                .is_some_and(|spec| !Self::is_objects_only_target(spec))
            {
                return self.deny(
                    "a multi-mode charm with a non-object second targeting mode".to_string(),
                );
            }
        }
        let choose = match (min, max) {
            (1, 1) => "ModeCount::ONE".to_string(),
            (2, 2) => "ModeCount::TWO".to_string(),
            _ => format!("ModeCount {{ min: {min}, max: {max} }}"),
        };
        let modes: Vec<String> = parsed.into_iter().map(|(m, _, _, _)| m).collect();
        self.body.abilities.push(format!(
            "AbilityDef::ModalSpell {{ choose: {choose}, modes: &[{}] }}",
            modes.join(", ")
        ));
        Some(())
    }

    /// `AB$ Charm | Cost$ ... | Choices$ A,B`: a modal activated ability
    /// (`modal_activated!`). Choose-one only; non-mana only.
    fn modal_activated(&mut self, spec: &str) -> Option<()> {
        let Some((_, mut p)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        p.drop_prose();
        let Some(cost_raw) = p.take("Cost") else {
            return self.deny("a modal activated ability with no `Cost$`".to_string());
        };
        let Some(choices) = p.take("Choices") else {
            return self.deny("a charm with no `Choices$`".to_string());
        };
        match p.take("CharmNum").as_deref() {
            None | Some("1") => {}
            Some(n) => return self.deny(format!("a modal activated ability choosing `{n}`")),
        }
        let limit = p.take("ActivationLimit");
        let activator = p.take("Activator");
        let activation_zone = p.take("ActivationZone");
        let reduce_cost = p.take("ReduceCost");
        if reduce_cost.is_some() && p.has("ReduceAmount") {
            return self.deny("a `ReduceCost$` with `ReduceAmount$`".to_string());
        }
        let cost_reduction = match reduce_cost.as_deref() {
            Some(raw) => Some(self.activation_cost_reduction(raw, activation_zone.as_deref())?),
            None => None,
        };
        let sorcery_speed = match p.take("SorcerySpeed").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`SorcerySpeed$ {other}`")),
        };
        let any_player = match activator.as_deref() {
            None => false,
            Some("Player") => true,
            Some(other) => return self.deny(format!("`Activator$ {other}`")),
        };
        let (_non_battlefield, zone) =
            self.activation_zone_expr(activation_zone.as_deref(), any_player, true)?;
        let mut condition = self.condition(&mut p)?;
        let _restrictions = self.activation_restriction(&mut p, true, &mut condition)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Charm.{key}`"));
        }
        let cost = self.parse_activated_cost(&cost_raw, activation_zone.as_deref())?;
        self.cost_sacrifices_self = cost_raw.split_whitespace().any(|t| t == "Sac<1/CARDNAME>");
        self.cost_sacrifices_other = cost_raw
            .split_whitespace()
            .any(|t| t.starts_with("Sac<1/") && !t.starts_with("Sac<1/CARDNAME"));
        let parsed = self.read_charm_modes(&choices);
        self.cost_sacrifices_self = false;
        self.cost_sacrifices_other = false;
        let parsed = parsed?;
        if parsed.len() < 2 {
            return self.deny("a charm with fewer than two modes".to_string());
        }
        if parsed.iter().any(|(_, _, _, mana)| *mana) {
            return self.deny("a modal activated ability with a mana mode".to_string());
        }
        let timing = if sorcery_speed {
            ", timing = ActivationTiming::SorcerySpeed"
        } else {
            ""
        };
        let limit = self.activation_limit(limit.as_deref())?;
        let reduction_suffix = cost_reduction.as_deref().unwrap_or("");
        let modes: Vec<String> = parsed.into_iter().map(|(m, _, _, _)| m).collect();
        self.body.abilities.push(format!(
            "modal_activated!({cost}, &[{}]{timing}{zone}{limit}{condition}{reduction_suffix})",
            modes.join(", ")
        ));
        Some(())
    }

    fn self_zone_cost_part(token: &str, in_graveyard: bool, in_hand: bool) -> Option<&'static str> {
        let (kind, body) = super::object_cost(token)?;
        let mut fields = body.splitn(3, '/');
        let (Some("1"), Some(spec)) = (fields.next(), fields.next()) else {
            return None;
        };
        match kind {
            "ExileFromGrave" if in_graveyard && spec == "CARDNAME" => Some("ExileSelf"),
            "Discard" if in_hand && matches!(spec, "CARDNAME" | "NICKNAME") => Some("DiscardSelf"),
            "ExileFromHand" if in_hand && matches!(spec, "CARDNAME" | "NICKNAME") => {
                Some("ExileSelf")
            }
            _ => None,
        }
    }

    fn exert_self_cost_part(token: &str) -> bool {
        matches!(token, "Exert<1/CARDNAME>" | "Exert<1/NICKNAME>")
            || token.starts_with("Exert<1/CARDNAME/")
            || token.starts_with("Exert<1/NICKNAME/")
    }

    fn parse_activated_cost(&mut self, raw: &str, activation_zone: Option<&str>) -> Option<String> {
        let in_graveyard = activation_zone == Some("Graveyard");
        let in_hand = activation_zone == Some("Hand");
        let prev_grave = std::mem::replace(&mut self.in_graveyard_activation, in_graveyard);
        let prev_hand = std::mem::replace(&mut self.in_hand_activation, in_hand);
        let res = self.parse_activated_cost_inner(raw, in_graveyard, in_hand);
        self.in_graveyard_activation = prev_grave;
        self.in_hand_activation = prev_hand;
        res
    }

    fn parse_activated_cost_inner(
        &mut self,
        raw: &str,
        in_graveyard: bool,
        in_hand: bool,
    ) -> Option<String> {
        let tokens = cost_parts(raw);
        if (in_graveyard || in_hand) && tokens.iter().any(|t| t.starts_with("Exert<")) {
            return self.deny("`Exert` outside the battlefield".to_string());
        }
        let has_nickname_exert = !in_graveyard
            && !in_hand
            && tokens
                .iter()
                .any(|t| t == "Exert<1/NICKNAME>" || t.starts_with("Exert<1/NICKNAME/"));
        if !in_graveyard && !in_hand && !has_nickname_exert {
            return self.cost_expr(raw);
        }
        let mut mana = String::new();
        let mut parts = Vec::new();
        let mut chunk = Vec::new();
        for token in tokens {
            let intercepted =
                Self::self_zone_cost_part(&token, in_graveyard, in_hand).or_else(|| {
                    (!in_graveyard && !in_hand && Self::exert_self_cost_part(&token))
                        .then_some("ExertSelf")
                });
            if let Some(self_part) = intercepted {
                if !chunk.is_empty() {
                    let (m, p) = self.cost_pieces(&chunk.join(" "))?;
                    mana.push_str(&m);
                    parts.extend(p);
                    chunk.clear();
                }
                parts.push(self_part.to_string());
            } else {
                chunk.push(token);
            }
        }
        if !chunk.is_empty() {
            let (m, p) = self.cost_pieces(&chunk.join(" "))?;
            mana.push_str(&m);
            parts.extend(p);
        }
        Some(crate::body::cost_literal(&mana, &parts))
    }

    fn activation_cost_reduction(
        &mut self,
        raw: &str,
        activation_zone: Option<&str>,
    ) -> Option<String> {
        let raw = raw.trim();
        let def = if raw.starts_with("Count$") || raw.starts_with("PlayerCount") {
            raw.to_string()
        } else if let Some(resolved) = self.svars.get(raw).cloned() {
            resolved.trim().to_string()
        } else {
            return self.deny(format!("`ReduceCost$ {raw}` naming no `SVar`"));
        };
        if def == "PlayerCountPropertyYou$HasPropertyActive" {
            return Some(
                ", cost_reduction = Some(CostReduction::If { condition: Condition::YourTurn, amount: 1 })"
                    .to_string(),
            );
        }
        if let Some(compare) = def.strip_prefix("Count$Compare ") {
            if let Some((lhs_part, else_str)) = compare.rsplit_once('.')
                && else_str.trim() == "0"
                && let Some((cond_spec, then_str)) = lhs_part.rsplit_once('.')
                && let Ok(amount) = then_str.trim().parse::<u32>()
                && amount > 0
                && let Some((var, cmp)) = cond_spec.trim().rsplit_once(' ')
                && let Some(cond) = self.svar_condition_clause(var.trim(), cmp.trim())
                && !cond.contains("Condition::XAtLeast")
            {
                return Some(format!(
                    ", cost_reduction = Some(CostReduction::If {{ condition: {cond}, amount: {amount} }})"
                ));
            }
            return self.deny(format!("unsupported `ReduceCost$ {raw}`"));
        }
        let Some(amt) = self.static_count_expr(&def) else {
            return self.deny(format!("unsupported `ReduceCost$ {raw}`"));
        };
        if amt.starts_with("Amount::Fixed(")
            || (amt.contains("SourcePower")
                && !matches!(activation_zone, None | Some("Battlefield")))
        {
            return self.deny(format!("unsupported `ReduceCost$ {raw}`"));
        }
        Some(format!(
            ", cost_reduction = Some(CostReduction::PerCount {{ amount: {amt}, each: 1 }})"
        ))
    }

    /// Named ability-word / game-state conditions shared by `S:`, `T:`, `A:`,
    /// and line-level `Condition$`.
    pub(super) fn named_condition(cond: &str) -> Option<&'static str> {
        Some(match cond.trim() {
            "Threshold" => "Condition::GraveyardCountAtLeast(7)",
            "PlayerTurn" => "Condition::YourTurn",
            "NotPlayerTurn" => "Condition::Not(&Condition::YourTurn)",
            "MaxSpeed" => "Condition::MaxSpeed",
            "Delirium" => "Condition::GraveyardTypesAtLeast(4)",
            "Metalcraft" => "Condition::ControlCount(&Filter::ARTIFACT, 3)",
            "Hellbent" => "Condition::HandSizeAtMost(0)",
            "Blessing" => "Condition::CitysBlessing",
            "EnduringStory" => "Condition::EnduringStory",
            "FatefulHour" => "Condition::LifeAtMost(5)",
            "Revolt" => "Condition::Revolt",
            _ => return None,
        })
    }

    /// Unified `CheckSVar$` + `SVarCompare$` entry point across activated,
    /// static, and cost-reduction readers.
    pub(super) fn svar_condition_clause(&mut self, var: &str, cmp: &str) -> Option<String> {
        self.svar_condition(var, cmp)
            .or_else(|| self.inline_valid_count_condition(var.trim(), cmp))
            .or_else(|| self.extra_svar_condition_clause(var, cmp))
            .or_else(|| self.inline_svar_condition(var, cmp))
    }

    fn inline_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let def = var.trim();
        let cmp = cmp.trim();
        if def == "PlayerCountPropertyYou$HasPropertyMaxSpeed" {
            return match cmp {
                "GE1" | "EQ1" => Some("Condition::MaxSpeed".to_string()),
                "EQ0" | "NE1" | "LT1" => Some("Condition::Not(&Condition::MaxSpeed)".to_string()),
                _ => None,
            };
        }
        if def.starts_with("Count$")
            && let Some(amt) = self.static_count_expr(def)
        {
            return match count_bound(cmp)? {
                Bound::AtLeast(n) if n > 0 => Some(format!("Condition::AmountAtLeast({amt}, {n})")),
                Bound::AtMost(max) => {
                    let bound = u32::from(max).checked_add(1)?;
                    Some(format!(
                        "Condition::Not(&Condition::AmountAtLeast({amt}, {bound}))"
                    ))
                }
                Bound::AtLeast(_) => None,
            };
        }
        None
    }

    /// Reads a `CheckSVar$` + `SVarCompare$` (or `ConditionCheckSVar$` +
    /// `ConditionSVarCompare$`) pair into a `Condition` expression.
    #[allow(clippy::too_many_lines)]
    pub(super) fn svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.svars.get(var.trim())?.trim().to_string();
        let cmp = cmp.trim();
        let eq_num = cmp.strip_prefix("EQ").and_then(|n| n.parse::<u16>().ok());
        let bound = count_bound(cmp);

        if raw == "Count$xPaid" {
            if !self.on_a_spell || !self.has_x {
                return None;
            }
            return match bound {
                Some(Bound::AtLeast(n)) => Some(format!("Condition::XAtLeast({n})")),
                _ => None,
            };
        }
        if raw == "Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature" {
            return match bound {
                Some(Bound::AtLeast(n)) => {
                    Some(format!("Condition::CreaturesDiedThisTurnAtLeast({n})"))
                }
                Some(Bound::AtMost(0)) => {
                    Some("Condition::Not(&Condition::CreaturesDiedThisTurnAtLeast(1))".to_string())
                }
                _ => None,
            };
        }
        if raw == "Count$ThisTurnEntered_Battlefield_Creature.YouCtrl" {
            return self.amount_bound_condition(
                "Amount::CreaturesEnteredUnderYourControlThisTurn",
                bound,
                eq_num,
            );
        }
        if matches!(
            raw.as_str(),
            "Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl"
                | "Count$ThisTurnCast_Sorcery.YouCtrl,Instant.YouCtrl"
                | "Count$ThisTurnCast_Card.Instant,Sorcery+YouCtrl"
                | "Count$ThisTurnCast_Card.Instant+YouCtrl,Card.Sorcery+YouCtrl"
        ) {
            return match cmp {
                "GE1" => Some("Condition::CastInstantOrSorceryThisTurn".to_string()),
                "EQ0" => {
                    Some("Condition::Not(&Condition::CastInstantOrSorceryThisTurn)".to_string())
                }
                _ => None,
            };
        }
        if matches!(
            raw.as_str(),
            "Count$ThisTurnCast_Creature.YouCtrl" | "Count$ThisTurnCast_Card.Creature+YouCtrl"
        ) {
            return match cmp {
                "GE1" => Some("Condition::CastCreatureThisTurn".to_string()),
                "EQ0" => Some("Condition::Not(&Condition::CastCreatureThisTurn)".to_string()),
                _ => None,
            };
        }
        if raw == "Count$ThisTurnCast_Card.YouCtrl" {
            return match cmp {
                "EQ0" => Some("Condition::YouCastNoSpellThisTurn".to_string()),
                "GE1" => Some("Condition::Not(&Condition::YouCastNoSpellThisTurn)".to_string()),
                _ => None,
            };
        }
        if matches!(raw.as_str(), "Count$Morbid.1.0" | "Count$Morbid") {
            return match cmp {
                "GE1" | "EQ1" => Some("Condition::CreaturesDiedThisTurnAtLeast(1)".to_string()),
                "EQ0" => {
                    Some("Condition::Not(&Condition::CreaturesDiedThisTurnAtLeast(1))".to_string())
                }
                _ => None,
            };
        }
        if matches!(raw.as_str(), "Count$Raid.1.0" | "Count$Raid") {
            return match cmp {
                "GE1" | "EQ1" => Some("Condition::AttackedWithAtLeast(1)".to_string()),
                "EQ0" => Some("Condition::Not(&Condition::AttackedWithAtLeast(1))".to_string()),
                _ => None,
            };
        }
        if matches!(raw.as_str(), "Count$Revolt.1.0" | "Count$Revolt") {
            return match cmp {
                "GE1" | "EQ1" => Some("Condition::Revolt".to_string()),
                "EQ0" => Some("Condition::Not(&Condition::Revolt)".to_string()),
                _ => None,
            };
        }
        if matches!(
            raw.as_str(),
            "Count$DescendedThisTurn" | "Count$ThisTurnDescended"
        ) {
            return match cmp {
                "GE1" | "EQ1" => Some("Condition::DescendedThisTurn".to_string()),
                "EQ0" => Some("Condition::Not(&Condition::DescendedThisTurn)".to_string()),
                _ => None,
            };
        }
        if raw == "Count$AltCost.1.0" {
            return match cmp {
                "GE1" | "EQ1" => Some("Condition::AltCostPaid".to_string()),
                "EQ0" => Some("Condition::Not(&Condition::AltCostPaid)".to_string()),
                _ => None,
            };
        }
        if raw == "Count$Bargained.1.0" && self.body.bargain {
            return match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::Bargained".to_string()),
                "EQ0" | "NE1" | "LT1" => Some("Condition::Not(&Condition::Bargained)".to_string()),
                _ => None,
            };
        }
        if raw == "PlayerCountPropertyYou$HasPropertyActive" {
            return match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::YourTurn".to_string()),
                "EQ0" | "NE1" | "LT1" => Some("Condition::Not(&Condition::YourTurn)".to_string()),
                _ => None,
            };
        }
        if raw == "PlayerCountRegisteredOpponents$HasPropertyLostLifeThisTurn" {
            return match cmp {
                "GE1" | "EQ1" => Some("Condition::OpponentLostLifeThisTurn".to_string()),
                "EQ0" => Some("Condition::Not(&Condition::OpponentLostLifeThisTurn)".to_string()),
                _ => None,
            };
        }
        if raw == "Count$LastTurnCast_Card" {
            return match bound {
                Some(Bound::AtMost(0)) => Some("Condition::NoSpellsCastLastTurn".to_string()),
                Some(Bound::AtLeast(n)) if n >= 1 => {
                    Some(format!("Condition::APlayerCastLastTurnAtLeast({n})"))
                }
                _ => None,
            };
        }
        if let Some(entered_valid) = raw.strip_prefix("Count$ThisTurnEntered_Battlefield_")
            && matches!(cmp, "GE1" | "EQ0")
        {
            let mut paths = Vec::new();
            let mut ok = true;
            for alt in entered_valid.split(',').map(str::trim) {
                let Some(base) = alt
                    .strip_suffix(".YouCtrl")
                    .or_else(|| {
                        alt.strip_prefix("Permanent.")
                            .and_then(|s| s.strip_suffix("+YouCtrl"))
                    })
                    .or_else(|| {
                        alt.strip_prefix("Card.")
                            .and_then(|s| s.strip_suffix("+YouCtrl"))
                    })
                else {
                    ok = false;
                    break;
                };
                if let Some(path) = self.cats.const_path(base) {
                    paths.push(path);
                } else {
                    ok = false;
                    break;
                }
            }
            if ok && !paths.is_empty() {
                let inner = format!(
                    "Condition::EnteredSubtypeUnderYourControlThisTurn(&[{}])",
                    paths.join(", ")
                );
                return Some(if cmp == "EQ0" {
                    format!("Condition::Not(&{inner})")
                } else {
                    inner
                });
            }
        }
        if raw == "Count$ThisTurnLandPlayed" {
            return match bound {
                Some(Bound::AtLeast(n)) => {
                    Some(format!("Condition::LandsPlayedThisTurnAtLeast({n})"))
                }
                _ => None,
            };
        }
        if raw == "Count$AttackersDeclared" {
            return match bound {
                Some(Bound::AtLeast(n)) => Some(format!("Condition::AttackedWithAtLeast({n})")),
                _ => None,
            };
        }
        if raw == "Count$LifeOppsLostThisTurn/LimitMax.1" {
            return match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::OpponentLostLifeThisTurn".to_string()),
                "EQ0" | "LT1" | "LE0" => {
                    Some("Condition::Not(&Condition::OpponentLostLifeThisTurn)".to_string())
                }
                _ => None,
            };
        }
        if raw == "Count$LifeOppsLostThisTurn" {
            return match (bound, eq_num) {
                (Some(Bound::AtMost(0)), _) => {
                    Some("Condition::Not(&Condition::OpponentLostLifeThisTurn)".to_string())
                }
                (Some(Bound::AtLeast(1)), _) => {
                    Some("Condition::OpponentLostLifeThisTurn".to_string())
                }
                (_, Some(1)) => {
                    let inner = "Condition::AmountAtLeast(Amount::OpponentsLifeLostThisTurn, 2)";
                    let name = self.body.condition_static("CHECK", inner);
                    Some(format!(
                        "Condition::All(&[Condition::OpponentLostLifeThisTurn, Condition::Not(&{name})])"
                    ))
                }
                _ => {
                    self.amount_bound_condition("Amount::OpponentsLifeLostThisTurn", bound, eq_num)
                }
            };
        }
        if raw == "Count$LifeYouGainedThisTurn" {
            return self.amount_bound_condition(
                "Amount::LifeGainedThisTurn(PlayerRel::You)",
                bound,
                eq_num,
            );
        }
        if raw == "Count$YourLifeTotal" {
            if let Some(n) = eq_num
                && n > 0
            {
                return Some(format!(
                    "Condition::All(&[Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), {n}), Condition::LifeAtMost({n})])"
                ));
            }
            if let Some(bound) = bound {
                return Some(match bound {
                    Bound::AtLeast(n) => {
                        format!("Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), {n})")
                    }
                    Bound::AtMost(n) => format!("Condition::LifeAtMost({n})"),
                });
            }
        }
        if matches!(
            raw.as_str(),
            "Count$ValidHand Card.YouOwn" | "Count$CardsInYourHand"
        ) {
            if let Some(n) = eq_num.and_then(|v| u8::try_from(v).ok()) {
                return Some(if n == 0 {
                    "Condition::HandSizeAtMost(0)".to_string()
                } else {
                    format!("Condition::HandSizeExactly({n})")
                });
            }
            return match bound {
                Some(Bound::AtMost(n)) => Some(format!("Condition::HandSizeAtMost({n})")),
                Some(Bound::AtLeast(n)) => Some(format!(
                    "Condition::AmountAtLeast(Amount::HandSize(PlayerRel::You), {n})"
                )),
                None => None,
            };
        }
        if raw == "Count$ValidGraveyard Card.YouOwn"
            && let Some(Bound::AtLeast(n)) = bound
        {
            return Some(format!("Condition::GraveyardCountAtLeast({n})"));
        }
        if raw == "Count$ValidGraveyard Card.YouOwn$CardTypes" {
            return match bound {
                Some(Bound::AtLeast(n)) => Some(format!("Condition::GraveyardTypesAtLeast({n})")),
                _ => None,
            };
        }
        if raw == "Count$ValidGraveyard Card.OppOwn" {
            return match bound {
                Some(Bound::AtLeast(n)) => {
                    Some(format!("Condition::OpponentGraveyardCountAtLeast({n})"))
                }
                _ => None,
            };
        }
        if let Some(kind_code) = raw.strip_prefix("Count$CardCounters.") {
            let kind = counter_kind(kind_code)?;
            if let Some(n) = eq_num.and_then(|v| u8::try_from(v).ok()) {
                return Some(format!("Condition::CountersOnSelfExactly({kind}, {n})"));
            }
            if let Some(Bound::AtLeast(n)) = bound {
                return Some(format!("Condition::CountersOnSelf({kind}, {n})"));
            }
        }
        if matches!(
            raw.as_str(),
            "Count$Valid Creature.YouCtrl$CardPower"
                | "Count$Valid Creature.YouCtrl$GreatestCardPower"
        ) {
            return match bound {
                Some(Bound::AtLeast(n)) => {
                    Some(format!("Condition::ControlledCreaturePowerAtLeast({n})"))
                }
                _ => None,
            };
        }
        if let Some(valid_raw) = raw.strip_prefix("Count$Valid ") {
            if let Some(distinct_valid) = valid_raw.strip_suffix("$DifferentCardNames") {
                let Some(Bound::AtLeast(n)) = bound else {
                    return None;
                };
                let relative = distinct_valid.split(',').any(|alt| {
                    alt.split(['.', '+']).any(|a| {
                        matches!(
                            a.trim(),
                            "Self"
                                | "Other"
                                | "EnchantedBy"
                                | "EquippedBy"
                                | "AttachedBy"
                                | "Attached"
                        )
                    })
                });
                let yours = distinct_valid
                    .split(',')
                    .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"));
                if relative || !yours {
                    return None;
                }
                let expr = self.filter_expr(distinct_valid)?;
                let name = self.body.filter_static("CHECK", &expr);
                return Some(format!("Condition::ControlDistinctNames(&{name}, {n})"));
            }
            let valid = match valid_raw.split_once("/Plus.") {
                None => valid_raw.to_string(),
                Some((first, second_var)) => {
                    if let Some(second) = self
                        .svars
                        .get(second_var.trim())
                        .and_then(|s| s.trim().strip_prefix("Count$Valid "))
                    {
                        format!("{first},{second}")
                    } else {
                        String::new()
                    }
                }
            };
            if !valid.is_empty() && !valid.contains('/') && !valid.contains('$') {
                let relative = valid.split(',').any(|alt| {
                    alt.split(['.', '+']).any(|a| {
                        matches!(
                            a.trim(),
                            "Self"
                                | "Other"
                                | "EnchantedBy"
                                | "EquippedBy"
                                | "AttachedBy"
                                | "Attached"
                        )
                    })
                });
                let yours = valid
                    .split(',')
                    .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"));
                let opps = valid
                    .split(',')
                    .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "OppCtrl"))
                    && matches!(bound, Some(Bound::AtMost(0)));
                let nobodys = !valid
                    .split(',')
                    .flat_map(|alt| alt.split(['.', '+']))
                    .any(|atom| atom.contains("Ctrl") || atom.contains("Own"));
                if !relative
                    && let Some(n) = eq_num.filter(|&n| n > 0)
                    && (yours || nobodys)
                {
                    let expr = self.filter_expr(&valid)?;
                    let name = self.body.filter_static("CHECK", &expr);
                    return Some(if yours {
                        format!(
                            "Condition::All(&[Condition::ControlCount(&{name}, {n}), Condition::ControlCountAtMost(&{name}, {n})])"
                        )
                    } else {
                        format!(
                            "Condition::All(&[Condition::BattlefieldCount(&{name}, {n}), Condition::BattlefieldCountAtMost(&{name}, {n})])"
                        )
                    });
                }
                if !relative
                    && (yours || opps || nobodys)
                    && let Some(bound) = bound
                {
                    let expr = self.filter_expr(&valid)?;
                    let name = self.body.filter_static("CHECK", &expr);
                    let (variant, n) = match (yours, bound) {
                        (true, Bound::AtLeast(n)) => ("ControlCount", n),
                        (true, Bound::AtMost(n)) => ("ControlCountAtMost", n),
                        (false, Bound::AtLeast(n)) => ("BattlefieldCount", n),
                        (false, Bound::AtMost(n)) => ("BattlefieldCountAtMost", n),
                    };
                    return Some(format!("Condition::{variant}(&{name}, {n})"));
                }
            }
        }
        let amt = self.static_count_expr(&raw)?;
        self.amount_bound_condition(&amt, bound, eq_num)
    }

    fn amount_bound_condition(
        &mut self,
        amt: &str,
        bound: Option<Bound>,
        eq_num: Option<u16>,
    ) -> Option<String> {
        if let Some(n) = eq_num
            && n > 0
        {
            let above = u32::from(n).checked_add(1)?;
            let inner = format!("Condition::AmountAtLeast({amt}, {above})");
            let name = self.body.condition_static("CHECK", &inner);
            return Some(format!(
                "Condition::All(&[Condition::AmountAtLeast({amt}, {n}), Condition::Not(&{name})])"
            ));
        }
        match bound? {
            Bound::AtLeast(n) => Some(format!("Condition::AmountAtLeast({amt}, {n})")),
            Bound::AtMost(n) => {
                let above = u32::from(n).checked_add(1)?;
                let inner = format!("Condition::AmountAtLeast({amt}, {above})");
                let name = self.body.condition_static("CHECK", &inner);
                Some(format!("Condition::Not(&{name})"))
            }
        }
    }

    /// The reference's `IsPresent$` family as an intervening-`if` clause
    /// (CR 603.4).
    ///
    /// What comes back is the macro argument and not the condition: three
    /// answers fit in one `Option<String>` that way — `None` is a line
    /// refused with a reason, an empty string one that prints no clause at
    /// all, and anything else the `, condition = Some(…)` to splice in.
    /// `triggered!`, `activated!` and `mana_ability!` all spell the field
    /// the same, so the next caller needs no second shape.
    ///
    /// The family is seven keys and they are read here once, rather than at
    /// each `Mode$` that might carry them: 605 `T:` lines in the corpus
    /// write `IsPresent$`, spread across every trigger mode there is.
    ///
    /// **Two sentences come out of it**, and which one depends on what the
    /// clause is *about*. `PresentDefined$ Self`, or a valid-string whose
    /// every alternative pins the object with `Self`, is a clause about
    /// this card — `Condition::SourceMatches`. Anything else is a count,
    /// and a count is only readable here when the filter says whose: every
    /// alternative has to carry `YouCtrl`, or the sentence is "there exists
    /// a creature" and `Condition::ControlCount` would answer a narrower
    /// question than the card asks.
    ///
    /// **The zone is written into the filter**, and that is not a
    /// redundancy. `IsPresent$` asks whether an object is present *in a
    /// zone* — `PresentZone$`, defaulting to the battlefield — so the
    /// clause about this card is "this permanent is on the battlefield and
    /// matches", and a filter that left the zone out would be a different
    /// sentence at CR 603.4's **second** check: the source is a battlefield
    /// permanent when the trigger is collected, and need not still be one
    /// when the ability resolves. This engine keeps an object's id across a
    /// zone change, so `SourceMatches` on its own answers for a card that
    /// has died. What is cleared on the way out (CR 400.7) is the half a
    /// permanent has — status, damage, counters — so `Card.tapped` would
    /// have been right by accident, and `Card.Self+YouCtrl` wrong, since a
    /// card in a graveyard keeps the controller it had.
    ///
    /// `NoResolvingCheck$ True` is the one that has to be named rather than
    /// ignored: 61 scripts carry it, and it is the reference opting *out*
    /// of CR 603.4's second check — a clause asked once instead of twice,
    /// which this DSL cannot say at all.
    pub(super) fn condition(&mut self, p: &mut Params) -> Option<String> {
        if let Some(cond_raw) = p.peek("Condition")
            && !p.has("IsPresent")
            && let Some(cond) = Self::named_condition(cond_raw)
        {
            p.take("Condition");
            return Some(format!(", condition = Some({cond})"));
        }
        if p.peek("LifeTotal") == Some("You") && !p.has("IsPresent") {
            p.take("LifeTotal");
            let life_amount = p.take("LifeAmount").unwrap_or_else(|| "GE1".to_string());
            return match count_bound(&life_amount) {
                Some(Bound::AtLeast(n)) => Some(format!(
                    ", condition = Some(Condition::AmountAtLeast(Amount::LifeTotal(PlayerRel::You), {n}))"
                )),
                Some(Bound::AtMost(n)) => {
                    Some(format!(", condition = Some(Condition::LifeAtMost({n}))"))
                }
                _ => self.deny(format!("`LifeAmount$ {life_amount}`")),
            };
        }
        let Some(valid) = p.take("IsPresent") else {
            return Some(String::new());
        };
        let clause = self.present_clause(p, &valid)?;
        Some(format!(", condition = Some({clause})"))
    }

    /// The rest of the `IsPresent$` family, `valid` being that key's value,
    /// as a `Condition` expression, or `None` when it was refused. What
    /// [`Self::condition`] splices in as an intervening "if", and what a
    /// state trigger (`Mode$ Always`, CR 603.8) triggers on.
    pub(super) fn present_clause(&mut self, p: &mut Params, valid: &str) -> Option<String> {
        if p.take("NoResolvingCheck").is_some() {
            return self.deny("`NoResolvingCheck$`, a clause checked once".to_string());
        }
        if p.take("IsPresent2").is_some() {
            return self.deny("a second `IsPresent2$` clause".to_string());
        }
        if p.peek("PresentZone") == Some("Graveyard")
            && (p.peek("PresentPlayer") == Some("You")
                || (p.peek("PresentPlayer").is_none()
                    && (matches!(valid, "Card.StrictlySelf" | "Card.Self")
                        || has_you_own_every_alt(valid))))
        {
            return self.graveyard_present_clause(p, valid);
        }
        if let Some(who) = p.take("PresentPlayer") {
            return self.deny(format!("`PresentPlayer$ {who}`"));
        }
        match p.take("PresentZone").as_deref() {
            None | Some("Battlefield") => {}
            Some(zone) => return self.deny(format!("`PresentZone$ {zone}`")),
        }
        let compare = p
            .take("PresentCompare")
            .unwrap_or_else(|| "GE1".to_string());
        if let Some(clause) =
            Self::commander_present_clause(valid, &compare, p.peek("PresentDefined").is_some())
        {
            return Some(clause);
        }
        if matches!(compare.as_str(), "GE1" | "EQ1")
            && matches!(p.peek("PresentDefined"), None | Some("Self"))
            && let Some(raw) = valid.strip_prefix("Card.Self+counters_GE")
        {
            let Some((n, code)) = raw.split_once('_') else {
                return self.deny(format!("a source counter clause `{valid}`"));
            };
            let n = n.parse::<u8>().ok()?;
            let kind = counter_kind(code)?;
            p.take("PresentDefined");
            return Some(format!("Condition::CountersOnSelf({kind}, {n})"));
        }
        let pins_self = |alt: &str| alt.split(['.', '+']).any(|atom| atom.trim() == "Self");
        let about_source = match p.take("PresentDefined").as_deref() {
            Some("Self") => true,
            Some(other) => return self.deny(format!("`PresentDefined$ {other}`")),
            None => valid.split(',').all(pins_self),
        };
        // Both readings need a number before the filter is worth building,
        // so that a refusal names the clause rather than an atom inside it.
        if !about_source {
            return self.battlefield_count_present_clause(valid, &compare);
        }
        if !matches!(compare.as_str(), "GE1" | "EQ1") {
            return self.deny(format!("a clause about this card compared `{compare}`"));
        }
        let expr = self.filter_expr(valid)?;
        let zoned = on_the_battlefield(&expr);
        let name = self.body.filter_static("CHECK", &zoned);
        Some(format!("Condition::SourceMatches(&{name})"))
    }

    fn battlefield_count_present_clause(&mut self, valid: &str, compare: &str) -> Option<String> {
        let exact_one = compare == "EQ1";
        let bound = if exact_one {
            None
        } else {
            let Some(bound) = count_bound(compare) else {
                return self.deny(format!("`PresentCompare$ {compare}`"));
            };
            Some(bound)
        };
        // Whose permanents: yours (`YouCtrl` in every alternative), or
        // everybody's when no alternative names a controller or an
        // owner at all ("if no creatures are on the battlefield").
        let atoms = || valid.split(',').flat_map(|alt| alt.split(['.', '+']));
        let yours = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"));
        let opponents = valid
            .split(',')
            .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "OppCtrl"))
            && matches!(bound, Some(Bound::AtMost(0)));
        let nobodys = !atoms().any(|atom| atom.contains("Ctrl") || atom.contains("Own"));
        if !yours && !nobodys && !opponents {
            return self.deny(format!("`IsPresent$ {valid}`, a count of somebody else's"));
        }
        // And a count says nothing about the card that states it.
        // `eval::condition_holds` walks a battlefield handing each
        // candidate its *own* id as the object a filter's `This` and
        // `Another` compare against, so "another creature you control"
        // would count nothing at all — 28 corpus lines write one, and
        // a trigger that can never fire is exactly the wrong card the
        // honest-stub rule exists to refuse.
        // The attachment atoms are relative too: `AttachedToBySource`
        // asks what *this* card is attached to.
        if valid.split(',').any(|alt| {
            alt.split(['.', '+']).any(|a| {
                matches!(
                    a.trim(),
                    "Self" | "Other" | "EnchantedBy" | "EquippedBy" | "AttachedBy" | "Attached"
                )
            })
        }) {
            return self.deny(format!(
                "`IsPresent$ {valid}`, a count relative to this card"
            ));
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("CHECK", &expr);
        // `ControlCount` counts one player's battlefield and nothing
        // else, and `BattlefieldCount` all of it, so the zone and the
        // player are both already in the sentence each is.
        Some(match bound {
            Some(bound) => {
                let (variant, n) = match (yours, bound) {
                    (true, Bound::AtLeast(n)) => ("ControlCount", n),
                    (true, Bound::AtMost(n)) => ("ControlCountAtMost", n),
                    (false, Bound::AtLeast(n)) => ("BattlefieldCount", n),
                    (false, Bound::AtMost(n)) => ("BattlefieldCountAtMost", n),
                };
                format!("Condition::{variant}(&{name}, {n})")
            }
            None if yours => format!(
                "Condition::All(&[Condition::ControlCount(&{name}, 1), Condition::ControlCountAtMost(&{name}, 1)])"
            ),
            None => format!(
                "Condition::All(&[Condition::BattlefieldCount(&{name}, 1), Condition::BattlefieldCountAtMost(&{name}, 1)])"
            ),
        })
    }

    fn commander_present_clause(valid: &str, compare: &str, has_defined: bool) -> Option<String> {
        if has_defined
            || !matches!(
                valid,
                "Card.IsCommander+YouOwn+YouCtrl"
                    | "Card.IsCommander+YouCtrl+YouOwn"
                    | "Permanent.IsCommander+YouOwn+YouCtrl"
                    | "Permanent.IsCommander+YouCtrl+YouOwn"
            )
        {
            return None;
        }
        match compare {
            "GE1" => Some("Condition::ControlYourCommander".to_string()),
            "EQ0" => Some("Condition::Not(&Condition::ControlYourCommander)".to_string()),
            _ => None,
        }
    }

    fn graveyard_present_clause(&mut self, p: &mut Params, valid: &str) -> Option<String> {
        p.take("PresentZone");
        let has_you = p.take("PresentPlayer").as_deref() == Some("You");
        let compare = p
            .take("PresentCompare")
            .unwrap_or_else(|| "GE1".to_string());
        if matches!(valid, "Card.StrictlySelf" | "Card.Self") {
            if !matches!(compare.as_str(), "GE1" | "EQ1") {
                return self.deny(format!("a graveyard self clause compared `{compare}`"));
            }
            return Some(
                "Condition::SourceMatches(&Filter::InZone(ZoneRef::Graveyard))".to_string(),
            );
        }
        let Some((stripped, above)) = strip_graveyard_filter(valid, !has_you) else {
            return self.deny(format!("`IsPresent$ {valid}` in graveyard"));
        };
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let name = self.body.filter_static("CHECK", &expr);
        if above {
            let Some(Bound::AtLeast(n)) = count_bound(&compare) else {
                return self.deny(format!("`PresentCompare$ {compare}` with `Above`"));
            };
            return Some(format!("Condition::GraveyardCardsAbove(&{name}, {n})"));
        }
        let amt = format!(
            "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::GraveyardOf(PlayerRel::You) }}"
        );
        if compare == "EQ1" {
            return Some(format!(
                "Condition::All(&[Condition::AmountAtLeast({amt}, 1), Condition::Not(&Condition::AmountAtLeast({amt}, 2))])"
            ));
        }
        match count_bound(&compare) {
            Some(Bound::AtLeast(n)) => Some(format!("Condition::AmountAtLeast({amt}, {n})")),
            Some(Bound::AtMost(n)) => {
                let Some(threshold) = u8::try_from(n).ok().and_then(|v| v.checked_add(1)) else {
                    return self.deny(format!("`PresentCompare$ {compare}`"));
                };
                Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast({amt}, {threshold}))"
                ))
            }
            None => self.deny(format!("`PresentCompare$ {compare}`")),
        }
    }

    /// The executed line of a "you may pay {N}. If you do, …" trigger with
    /// its price taken off, and the price: `AB$ GainLife | Cost$ 1 | …`
    /// under `OptionalDecider$ You` (or without it, as on the Mercadian
    /// Masques Torques, since an `AB$` executed by a trigger is always an
    /// optional cost). `None` for anything else — no `Cost$`, or a cost
    /// that is not mana alone — which leaves the line as it was for the
    /// chain to read or refuse.
    pub(super) fn optional_price(body: &str) -> Option<(String, Price)> {
        if !body.trim_start().starts_with("AB$") {
            return None;
        }
        let parts: Vec<&str> = body.split(" | ").collect();
        let cost = parts
            .iter()
            .find_map(|part| part.strip_prefix("Cost$"))?
            .trim();
        let price = match cost.parse::<u32>() {
            Ok(0) => return None,
            Ok(n) => Price::Generic(format!("Amount::Fixed({n})")),
            Err(_) => Price::Printed(printed_mana(cost)?),
        };
        let rest: Vec<&str> = parts
            .into_iter()
            .filter(|part| !part.starts_with("Cost$"))
            .collect();
        Some((rest.join(" | "), price))
    }

    /// Extends [`Self::optional_price`] for trigger-executed `AB$` lines whose
    /// optional cost is paying life or a single object-selection `CostPart`
    /// (`Discard<1/...>`, `Sac<1/...>`, `ExileFromGrave<1/...>`, etc.).
    pub(super) fn optional_trigger_price(
        &mut self,
        body: &str,
    ) -> Option<(String, OptionalTriggerPrice)> {
        if let Some((stripped, price)) = Self::optional_price(body) {
            return Some((
                stripped,
                match price {
                    Price::Generic(m) => OptionalTriggerPrice::Generic(m),
                    Price::Printed(c) => OptionalTriggerPrice::Printed(c),
                    Price::Part(p) => OptionalTriggerPrice::CostPart(p),
                },
            ));
        }
        if !body.trim_start().starts_with("AB$") {
            return None;
        }
        let parts: Vec<&str> = body.split(" | ").collect();
        let cost = parts
            .iter()
            .find_map(|part| part.strip_prefix("Cost$"))?
            .trim();
        let rest = parts
            .into_iter()
            .filter(|part| !part.starts_with("Cost$"))
            .collect::<Vec<_>>()
            .join(" | ");
        if cost == "Sac<1/CARDNAME>"
            || (cost.starts_with("Sac<1/CARDNAME/") && cost.find('>') == Some(cost.len() - 1))
        {
            return Some((
                rest,
                OptionalTriggerPrice::CostPart("Sacrifice(&Filter::This)".to_string()),
            ));
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let parsed = self.cost_pieces(cost);
        let Some((mana, cost_parts)) = parsed else {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return None;
        };
        if !mana.is_empty() {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return None;
        }
        let result = match cost_parts.as_slice() {
            [one] => {
                if let Some(n) = one
                    .strip_prefix("PayLife(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    Some(OptionalTriggerPrice::Life(format!("Amount::Fixed({n})")))
                } else if asks_for_an_object(one) {
                    Some(OptionalTriggerPrice::CostPart(one.clone()))
                } else {
                    None
                }
            }
            many if many.len() >= 2 => {
                let first = &many[0];
                if let Some(filter) = first
                    .strip_prefix("Sacrifice(")
                    .and_then(|s| s.strip_suffix(')'))
                    && many.iter().all(|p| p == first)
                    && let Ok(count) = u8::try_from(many.len())
                {
                    Some(OptionalTriggerPrice::SacrificeN {
                        filter: filter.to_string(),
                        count,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };
        if result.is_none() {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
        }
        result.map(|price| (rest, price))
    }
}

fn has_you_own_every_alt(valid: &str) -> bool {
    valid
        .split(',')
        .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouOwn"))
}

fn strip_graveyard_filter(valid: &str, require_you_own: bool) -> Option<(String, bool)> {
    let mut cleaned_alts = Vec::new();
    let mut saw_above: Option<bool> = None;
    for alt in valid.split(',') {
        let mut parts = alt
            .split('.')
            .flat_map(|s| s.split('+'))
            .map(str::trim)
            .filter(|a| !a.is_empty());
        let base = parts.next()?;
        let mut has_you = false;
        let mut alt_above = false;
        let mut kept = Vec::new();
        for atom in parts {
            match atom {
                "DirectlyAbove" => return None,
                "Above" => alt_above = true,
                "YouOwn" => has_you = true,
                other if other.contains("Ctrl") || other.contains("Own") => return None,
                other => kept.push(other),
            }
        }
        if require_you_own && !has_you {
            return None;
        }
        match saw_above {
            Some(prev) if prev != alt_above => return None,
            None => saw_above = Some(alt_above),
            Some(_) => {}
        }
        if kept.is_empty() {
            cleaned_alts.push(base.to_string());
        } else {
            cleaned_alts.push(format!("{base}.{}", kept.join("+")));
        }
    }
    Some((cleaned_alts.join(","), saw_above.unwrap_or(false)))
}
