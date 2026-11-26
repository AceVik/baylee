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
    SacrificeN {
        filter: String,
        count: u8,
    },
    DiscardHand,
    RemoveCounterSelf {
        kind: String,
        n: u32,
    },
    ExileFromGraveyardN {
        count: u8,
        filter: String,
    },
    ManaAndLife {
        mana: String,
        life: u32,
    },
    ChooseNumberPay {
        pay_kind: &'static str,
        prompt: &'static str,
        max: String,
    },
}

enum Wave14aCost {
    Matched(String),
    Denied,
    Pass,
}

/// `(mode_expr, first_target_spec, has_second_target, could_add_mana)`.
pub(super) type CharmMode = (String, Option<String>, bool, bool);

/// Components of a parsed `AB$` activated ability line or granted `AddAbility$` `SVar`.
pub(super) struct ParsedActivated {
    pub cost: String,
    pub effects: String,
    pub mana_ability: bool,
    pub is_loyalty: bool,
    pub extras: String,
    pub ability_expr: String,
    pub alternate_ability_expr: Option<String>,
    pub non_battlefield_zone: bool,
    pub simple_target: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActivationKind {
    Ordinary,
    Loyalty,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActivatorRestriction {
    Default,
    AnyPlayer,
    Opponent,
    Owner,
    AttachedController,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParsedLimit {
    Unlimited,
    PerTurnRaw(String),
    Exhaust,
    Boast,
    OncePerPermanent,
}

struct StrippedXMin {
    cost: String,
    explicit: Option<u32>,
    implicit: Option<u32>,
}

#[allow(clippy::struct_excessive_bools)]
struct ExtractedAbilitySpec {
    cost: Option<String>,
    alternate_cost: Option<String>,
    limit: ParsedLimit,
    x_min_param: u32,
    activation_zone: Option<String>,
    non_battlefield_zone: bool,
    zone_suffix: &'static str,
    activator: ActivatorRestriction,
    sorcery_speed: bool,
    instant_speed: bool,
    is_forecast: bool,
    kind: ActivationKind,
    condition: String,
    cost_reduction: String,
    target_req_override: Option<String>,
    same_controller_targets: bool,
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
            "BeginCombat->Declare Attackers" => {
                "Condition::Any(&[Condition::DuringStep(StepKind::CombatBegin), Condition::DuringStep(StepKind::DeclareAttackers)])"
            }
            "Main2->Cleanup" => "Condition::AfterCombat",
            "Upkeep->Main1,Main2->Cleanup" => "Condition::Not(&Condition::DuringCombat)",
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

    /// When an ability may be activated (CR 602.5) or a spell may be cast
    /// (CR 601.3), beyond its `IsPresent$` clause: a restriction that the
    /// `condition` field says. Hands back the keys it claimed.
    ///
    /// - `PlayerTurn$ True` / `OpponentTurn$ True`: turn-owner gating.
    /// - `ActivationPhases$`: combat phase, individual turn steps, or bounded
    ///   pre-combat/pre-attacker/post-blocker/post-combat windows.
    /// - `Activation$` / `CheckSVar$`: activated abilities only.
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
            if your_turn != "True" {
                return self.deny(format!("`PlayerTurn$ {your_turn}` beside another clause"));
            }
            clauses.push("Condition::YourTurn".to_string());
            claimed.push("PlayerTurn$");
        }
        if let Some(opp_turn) = probe.take("OpponentTurn") {
            if opp_turn != "True" || clauses.iter().any(|c| c == "Condition::YourTurn") {
                return self.deny(format!("`OpponentTurn$ {opp_turn}` beside another clause"));
            }
            clauses.push("Condition::OpponentsTurn".to_string());
            claimed.push("OpponentTurn$");
        }
        if let Some(phases) = probe.take("ActivationPhases") {
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
        let cmp = probe.peek("SVarCompare").map(str::to_string);
        let inline = name.trim().starts_with("Count$") || name.trim().starts_with("PlayerCount");
        if !inline {
            let Some(raw) = self.svars.get(name).cloned() else {
                return self.deny(format!("activation check `{name}` without an SVar"));
            };
            if let Some(alternatives) = raw.strip_prefix("Count$ThisTurnEntered_Battlefield_")
                && cmp.is_none()
            {
                let mut subtypes = Vec::new();
                let mut all_subtypes = true;
                for alternative in alternatives.split(',') {
                    let Some(subtype) = alternative.strip_suffix(".YouCtrl") else {
                        all_subtypes = false;
                        break;
                    };
                    let Some(path) = self.cats.const_path(subtype) else {
                        all_subtypes = false;
                        break;
                    };
                    subtypes.push(path);
                }
                if all_subtypes && !subtypes.is_empty() {
                    if !clauses.is_empty() {
                        return self.deny(
                            "an entry-history activation clause beside another clause".to_string(),
                        );
                    }
                    clauses.push(format!(
                        "Condition::EnteredSubtypeUnderYourControlThisTurn(&[{}])",
                        subtypes.join(", ")
                    ));
                    claimed.push("CheckSVar$");
                    return Some(());
                }
            }
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

    fn take_activation_limit(
        &self,
        probe: &mut Params,
        is_activated: bool,
    ) -> Option<(ParsedLimit, Vec<&'static str>)> {
        let limit_raw = probe.take("ActivationLimit");
        let exhaust_raw = probe.take("Exhaust");
        let game_limit_raw = probe.take("GameActivationLimit");
        let boast_raw = probe.take("Boast");
        let power_up_raw = probe.take("PowerUp");
        if !is_activated {
            if limit_raw.is_some() {
                return self.deny("`ActivationLimit$` on a spell line".to_string());
            }
            if exhaust_raw.is_some() {
                return self.deny("`Exhaust$` on a spell line".to_string());
            }
            if game_limit_raw.is_some() {
                return self.deny("`GameActivationLimit$` on a spell line".to_string());
            }
            if boast_raw.is_some() {
                return self.deny("`Boast$` on a spell line".to_string());
            }
            if power_up_raw.is_some() {
                return self.deny("`PowerUp$` on a spell line".to_string());
            }
            return Some((ParsedLimit::Unlimited, Vec::new()));
        }
        let count = usize::from(limit_raw.is_some())
            + usize::from(exhaust_raw.is_some())
            + usize::from(game_limit_raw.is_some())
            + usize::from(boast_raw.is_some())
            + usize::from(power_up_raw.is_some());
        if count > 1 {
            return self.deny("multiple activation limits on one ability".to_string());
        }
        if let Some(raw) = exhaust_raw {
            return if raw == "True" {
                Some((ParsedLimit::Exhaust, vec!["Exhaust$"]))
            } else {
                self.deny(format!("`Exhaust$ {raw}`"))
            };
        }
        if let Some(raw) = game_limit_raw {
            return if raw == "1" {
                Some((ParsedLimit::Exhaust, vec!["GameActivationLimit$"]))
            } else {
                self.deny(format!("`GameActivationLimit$ {raw}`"))
            };
        }
        if let Some(raw) = boast_raw {
            return if raw == "True" {
                Some((ParsedLimit::Boast, vec!["Boast$"]))
            } else {
                self.deny(format!("`Boast$ {raw}`"))
            };
        }
        if let Some(raw) = power_up_raw {
            return if raw == "True" {
                Some((ParsedLimit::OncePerPermanent, vec!["PowerUp$"]))
            } else {
                self.deny(format!("`PowerUp$ {raw}`"))
            };
        }
        match limit_raw {
            None => Some((ParsedLimit::Unlimited, Vec::new())),
            Some(raw) => Some((ParsedLimit::PerTurnRaw(raw), vec!["ActivationLimit$"])),
        }
    }

    fn parse_activator(
        &self,
        raw: Option<&str>,
        is_activated: bool,
    ) -> Option<ActivatorRestriction> {
        let Some(val) = raw else {
            return Some(ActivatorRestriction::Default);
        };
        if !is_activated {
            return self.deny(format!("`Activator$ {val}` on a spell"));
        }
        match val {
            "Player" => Some(ActivatorRestriction::AnyPlayer),
            "Player.Opponent" => Some(ActivatorRestriction::Opponent),
            "Player.Owner" => Some(ActivatorRestriction::Owner),
            "Player.EnchantedController" => Some(ActivatorRestriction::AttachedController),
            other => self.deny(format!("`Activator$ {other}`")),
        }
    }

    /// Read the activation count separately from cost and effect emission.
    fn activation_limit(&mut self, limit: &ParsedLimit) -> Option<String> {
        Some(match limit {
            ParsedLimit::Unlimited => String::new(),
            ParsedLimit::Exhaust => ", limit = ActivationLimit::Exhaust".to_string(),
            ParsedLimit::Boast => ", limit = ActivationLimit::Boast".to_string(),
            ParsedLimit::OncePerPermanent => {
                ", limit = ActivationLimit::OncePerPermanent".to_string()
            }
            ParsedLimit::PerTurnRaw(raw) => {
                if let Ok(n) = raw.parse::<u8>() {
                    format!(", limit = ActivationLimit::PerTurn({n})")
                } else if let Some(def) = self.svars.get(raw.trim()).cloned()
                    && let Some(expr) = self
                        .static_count_expr(&def)
                        .or_else(|| super::atoms::static_wave14_amount(&def, self.svars))
                {
                    format!(", limit = ActivationLimit::PerTurnAmount({expr})")
                } else {
                    return self.deny(format!("activation limit `{raw}`"));
                }
            }
        })
    }

    fn strip_activation_x_min(&self, raw: &str) -> Option<StrippedXMin> {
        let mut kept = Vec::new();
        let mut explicit: Option<u32> = None;
        let mut implicit: Option<u32> = None;
        for token in cost_parts(raw) {
            if let Some(n_str) = token.strip_prefix("XMin") {
                if explicit.is_some() {
                    return self.deny(format!("duplicate `XMin` in `Cost$ {raw}`"));
                }
                let Ok(n) = n_str.parse::<u32>() else {
                    return self.deny(format!("invalid `{token}` in `Cost$ {raw}`"));
                };
                if n == 0 {
                    return self.deny(format!("invalid `{token}` in `Cost$ {raw}`"));
                }
                explicit = Some(n);
            } else if let Some(rest) = token.strip_prefix("RemoveAnyCounter<X1+/") {
                implicit = Some(1);
                kept.push(format!("RemoveAnyCounter<X/{rest}"));
            } else if let Some(rest) = token.strip_prefix("SubCounter<X1+/") {
                implicit = Some(1);
                kept.push(format!("SubCounter<X/{rest}"));
            } else {
                kept.push(token);
            }
        }
        Some(StrippedXMin {
            cost: kept.join(" "),
            explicit,
            implicit,
        })
    }

    fn take_x_min_param(
        &self,
        probe: &mut Params,
        is_activated: bool,
        is_planeswalker: bool,
    ) -> Option<u32> {
        let Some(raw) = probe.take("XMin") else {
            return Some(0);
        };
        if !is_activated {
            return self.deny("`XMin$` on a spell".to_string());
        }
        if is_planeswalker {
            return self.deny("`XMin$` on a loyalty ability".to_string());
        }
        let Ok(n) = raw.trim().parse::<u32>() else {
            return self.deny(format!("`XMin$ {raw}`"));
        };
        if n == 0 {
            return self.deny(format!("`XMin$ {raw}`"));
        }
        Some(n)
    }

    fn merge_x_min(&self, stripped: &StrippedXMin, param_x_min: u32) -> Option<u32> {
        let explicit = match (stripped.explicit, param_x_min) {
            (Some(_), 1..) => {
                return self.deny("duplicate `XMin` in cost and `XMin$`".to_string());
            }
            (Some(n), 0) | (None, n @ 1..) => Some(n),
            (None, 0) => None,
        };
        match (explicit, stripped.implicit) {
            (Some(n), Some(m)) if n != m => {
                self.deny(format!("conflicting `XMin` ({n}) with `X1+`"))
            }
            (Some(n), _) | (None, Some(n)) => Some(n),
            (None, None) => Some(0),
        }
    }

    fn is_sac_self_token(token: &str) -> bool {
        token == "Sac<1/CARDNAME>"
            || token.starts_with("Sac<1/CARDNAME/")
            || token == "Sac<1/NICKNAME>"
            || token.starts_with("Sac<1/NICKNAME/")
    }

    fn is_sac_other_token(token: &str) -> bool {
        token.starts_with("Sac<1/") && !Self::is_sac_self_token(token)
    }

    pub(super) fn cost_has_variable_x(cost: &str) -> bool {
        cost.contains("{X}")
            || cost.contains("SacrificeX(")
            || cost.contains("TapX(")
            || cost.contains("DiscardX(")
            || cost.contains("ExileFromGraveyardX(")
            || cost.contains("PayEnergyX")
            || cost.contains("PayLifeX")
            || cost.contains("RemoveCounterSelfX {")
            || cost.contains("RemoveAnyCounterX {")
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
            let res = self.divided_damage_spell(p);
            if res.is_none() && self.unclaimed.borrow().is_none() {
                self.note("an unsupported `DividedAsYouChoose` damage spell".to_string());
            }
            return res;
        }
        if let Some((api, p)) = Params::parse(spec)
            && api == "ManaReflected"
        {
            if !is_activated {
                return self.deny("reflected Gate mana as a spell".into());
            }
            return self.activated_mana_reflected(p);
        }
        if !is_activated
            && Params::parse(spec).is_some_and(|(api, _)| {
                matches!(api.as_str(), "PermanentCreature" | "PermanentNoncreature")
            })
        {
            return self.wave16b_permanent_spell(spec);
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
                &extracted.limit,
                &extracted.condition,
                &effects,
                &extracted.chain,
                extracted.target_req_override.as_deref(),
                extracted.same_controller_targets,
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

    fn take_planeswalker_flags(
        &self,
        probe: &mut Params,
        is_activated: bool,
        allow_oracle_tap: bool,
    ) -> Option<(ActivationKind, bool)> {
        let planeswalker_raw = probe.take("Planeswalker");
        let ultimate_raw = probe.take("Ultimate");
        let kind = match planeswalker_raw.as_deref() {
            None => ActivationKind::Ordinary,
            Some(v) if v.eq_ignore_ascii_case("true") => {
                if !is_activated {
                    return self.deny("`Planeswalker$` on a spell".to_string());
                }
                let _ = allow_oracle_tap;
                ActivationKind::Loyalty
            }
            Some(other) => return self.deny(format!("`Planeswalker$ {other}`")),
        };
        let is_loyalty = kind == ActivationKind::Loyalty;
        match ultimate_raw.as_deref() {
            None => {}
            Some(v) if is_loyalty && v.eq_ignore_ascii_case("true") => {}
            Some(_) if !is_loyalty => {
                return self.deny("`Ultimate$` without `Planeswalker$ True`".to_string());
            }
            Some(other) => return self.deny(format!("`Ultimate$ {other}`")),
        }
        Some((kind, ultimate_raw.is_some()))
    }

    fn take_reduce_cost(
        &mut self,
        probe: &mut Params,
        is_activated: bool,
        activation_zone: Option<&str>,
        is_power_up: bool,
    ) -> Option<String> {
        if is_power_up {
            if activation_zone.is_some_and(|zone| zone != "Battlefield") {
                return self.deny("`PowerUp$` outside the battlefield".to_string());
            }
            if probe.has("ReduceCost") || probe.has("ReduceAmount") {
                return self.deny("`PowerUp$` beside `ReduceCost$`".to_string());
            }
            return Some(
                ", cost_reduction = Some(CostReduction::OwnManaCostIfEnteredThisTurn)".to_string(),
            );
        }
        let Some(raw) = probe.take("ReduceCost") else {
            return Some(String::new());
        };
        if !is_activated {
            return self.deny("`ReduceCost$` on a spell".to_string());
        }
        if probe.has("ReduceAmount") {
            return self.deny("a `ReduceCost$` with `ReduceAmount$`".to_string());
        }
        self.activation_cost_reduction(&raw, activation_zone)
    }

    #[allow(clippy::too_many_lines)]
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
        let line_prose = Self::params_prose(&probe);
        probe.drop_prose();
        let (kind, has_ultimate) =
            self.take_planeswalker_flags(&mut probe, is_activated, allow_oracle_tap)?;
        let is_planeswalker = kind == ActivationKind::Loyalty;
        let mut cost = probe.take("Cost");
        if is_activated && allow_oracle_tap && !is_planeswalker {
            cost = self.infer_single_ability_tap(cost);
        }
        let alternate_cost = probe.take("AlternateCost");
        if !is_activated && alternate_cost.is_some() {
            return self.deny("an `AlternateCost` on a spell".to_string());
        }
        let (mut limit, mut limit_keys) = self.take_activation_limit(&mut probe, is_activated)?;
        let x_min_param = self.take_x_min_param(&mut probe, is_activated, is_planeswalker)?;
        let activator = self.parse_activator(probe.take("Activator").as_deref(), is_activated)?;
        let mut activation_zone = probe.take("ActivationZone");
        if !is_activated && let Some(zone) = activation_zone.as_deref() {
            return self.deny(format!("a spell in `ActivationZone$ {zone}`"));
        }
        let is_forecast = match probe.take("Forecast").as_deref() {
            None => false,
            Some("True") if is_activated => {
                if !matches!(activation_zone.as_deref(), None | Some("Hand"))
                    || limit != ParsedLimit::Unlimited
                    || activator != ActivatorRestriction::Default
                    || is_planeswalker
                    || alternate_cost.is_some()
                    || !cost.as_deref().is_some_and(|c| {
                        cost_parts(c).iter().any(|t| {
                            t == "Reveal<1/CARDNAME>"
                                || t.starts_with("Reveal<1/CARDNAME/")
                                || t == "Reveal<1/NICKNAME>"
                                || t.starts_with("Reveal<1/NICKNAME/")
                        })
                    })
                {
                    return self.deny("an unsupported `Forecast$ True` ability".to_string());
                }
                activation_zone = Some("Hand".to_string());
                limit = ParsedLimit::PerTurnRaw("1".to_string());
                limit_keys.push("Forecast$");
                true
            }
            Some(other) => return self.deny(format!("`Forecast$ {other}`")),
        };
        let cost_reduction = self.take_reduce_cost(
            &mut probe,
            is_activated,
            activation_zone.as_deref(),
            limit == ParsedLimit::OncePerPermanent,
        )?;
        let (non_battlefield_zone, zone_suffix) = if is_activated {
            self.activation_zone_expr(activation_zone.as_deref(), activator, false)?
        } else {
            (false, "")
        };
        let zone_handled = is_activated && activation_zone.is_some();
        let sorcery_speed = is_activated && probe.take("SorcerySpeed").as_deref() == Some("True");
        let instant_speed = is_activated && probe.peek("InstantSpeed") == Some("True");
        if instant_speed {
            probe.take("InstantSpeed");
        }
        if is_forecast && (sorcery_speed || instant_speed) {
            return self.deny("`Forecast$ True` with timing override".to_string());
        }
        let had_life_total = probe.has("LifeTotal");
        let had_present_player = probe.has("PresentPlayer");
        let had_defined_player = probe.has("CheckDefinedPlayer");
        let mut condition = self.condition_with_prose(&mut probe, &line_prose)?;
        let present_read = !condition.is_empty();
        if !is_activated && present_read {
            return self.deny("`IsPresent$` on a spell line".to_string());
        }
        let life_total_read = had_life_total && !probe.has("LifeTotal");
        let present_player_read = had_present_player && !probe.has("PresentPlayer");
        let defined_player_read = had_defined_player && !probe.has("CheckDefinedPlayer");
        let restrictions = self.activation_restriction(&mut probe, is_activated, &mut condition)?;
        if is_forecast {
            let forecast_cond =
                "Condition::All(&[Condition::YourTurn, Condition::DuringStep(StepKind::Upkeep)])";
            condition = if let Some(inner) = condition
                .strip_prefix(", condition = Some(")
                .and_then(|s| s.strip_suffix(')'))
            {
                format!(", condition = Some(Condition::All(&[{inner}, {forecast_cond}]))")
            } else {
                format!(", condition = Some({forecast_cond})")
            };
        }
        let is_divided_counters = api == "PutCounter" && probe.has("DividedAsYouChoose");
        let is_divided_damage =
            is_activated && api == "DealDamage" && probe.has("DividedAsYouChoose");
        let is_excess_damage = api == "DealDamage" && probe.has("ExcessSVar");
        let same_controller_targets = if is_divided_counters || is_divided_damage {
            match probe.take("TargetsWithSameController").as_deref() {
                None | Some("False") => false,
                Some("True") => {
                    if !probe.has("ValidTgts") {
                        return self
                            .deny("`TargetsWithSameController$` without `ValidTgts$`".to_string());
                    }
                    true
                }
                Some(other) => {
                    return self.deny(format!("`TargetsWithSameController$ {other}`"));
                }
            }
        } else {
            false
        };
        let mut drop_keys: Vec<&'static str> = [
            (true, "Cost$"),
            (is_planeswalker, "Planeswalker$"),
            (has_ultimate, "Ultimate$"),
            (alternate_cost.is_some(), "AlternateCost$"),
            (x_min_param > 0, "XMin$"),
            (!cost_reduction.is_empty(), "ReduceCost$"),
            (activator != ActivatorRestriction::Default, "Activator$"),
            (zone_handled, "ActivationZone$"),
            (sorcery_speed, "SorcerySpeed$"),
            (instant_speed, "InstantSpeed$"),
            (life_total_read, "LifeTotal$"),
            (life_total_read, "LifeAmount$"),
            (present_player_read, "PresentPlayer$"),
            (defined_player_read, "CheckDefinedPlayer$"),
            (same_controller_targets, "TargetsWithSameController$"),
        ]
        .into_iter()
        .filter_map(|(on, key)| on.then_some(key))
        .collect();
        drop_keys.extend(limit_keys);
        drop_keys.extend(restrictions);
        if present_read {
            drop_keys.extend([
                "Condition$",
                "IsPresent$",
                "PresentZone$",
                "PresentCompare$",
                "PresentDefined$",
            ]);
        }
        let (chain, target_req_override) = if is_divided_counters {
            let stripped: Vec<&str> = spec
                .split(" | ")
                .filter(|part| {
                    let p = part.trim_start();
                    !drop_keys.iter().any(|key| p.starts_with(key))
                })
                .collect();
            let (_, dp) = Params::parse(&stripped.join(" | "))?;
            self.cost_sacrifices_self = is_activated
                && cost
                    .as_deref()
                    .is_some_and(|c| cost_parts(c).iter().any(|t| Self::is_sac_self_token(t)));
            self.cost_sacrifices_other = cost
                .as_deref()
                .is_some_and(|c| cost_parts(c).iter().any(|t| Self::is_sac_other_token(t)));
            let parsed = self.read_divided_counters(dp);
            self.cost_sacrifices_self = false;
            self.cost_sacrifices_other = false;
            let (effects, target_spec, req_expr) = parsed?;
            let chain = Chain {
                effects,
                target: Some(target_spec),
                ..Chain::default()
            };
            (chain, Some(req_expr))
        } else if is_divided_damage {
            let stripped: Vec<&str> = spec
                .split(" | ")
                .filter(|part| {
                    let p = part.trim_start();
                    !drop_keys.iter().any(|key| p.starts_with(key))
                })
                .collect();
            let (_, dp) = Params::parse(&stripped.join(" | "))?;
            self.cost_sacrifices_self = is_activated
                && cost
                    .as_deref()
                    .is_some_and(|c| cost_parts(c).iter().any(|t| Self::is_sac_self_token(t)));
            self.cost_sacrifices_other = cost
                .as_deref()
                .is_some_and(|c| cost_parts(c).iter().any(|t| Self::is_sac_other_token(t)));
            let parsed = self.wave28a_read_divided_damage(dp, false);
            self.cost_sacrifices_self = false;
            self.cost_sacrifices_other = false;
            let (effects, target_spec, req_expr, second_req) = parsed?;
            if second_req.is_some() {
                return self
                    .deny("an activated divided damage ability with a second target".to_string());
            }
            let chain = Chain {
                effects,
                target: Some(target_spec),
                ..Chain::default()
            };
            (chain, Some(req_expr))
        } else if is_excess_damage {
            let stripped: Vec<&str> = spec
                .split(" | ")
                .filter(|part| {
                    let p = part.trim_start();
                    !drop_keys.iter().any(|key| p.starts_with(key))
                })
                .collect();
            let mut chain = Chain::default();
            self.cost_sacrifices_self = is_activated
                && cost
                    .as_deref()
                    .is_some_and(|c| cost_parts(c).iter().any(|t| Self::is_sac_self_token(t)));
            self.cost_sacrifices_other = cost
                .as_deref()
                .is_some_and(|c| cost_parts(c).iter().any(|t| Self::is_sac_other_token(t)));
            let ok = self.excess_damage_chain(&stripped.join(" | "), &mut chain);
            self.cost_sacrifices_self = false;
            self.cost_sacrifices_other = false;
            ok?;
            (chain, None)
        } else {
            (
                self.read_ability_chain(spec, is_activated, cost.as_deref(), &drop_keys)?,
                None,
            )
        };
        Some(ExtractedAbilitySpec {
            cost,
            alternate_cost,
            limit,
            x_min_param,
            activation_zone,
            non_battlefield_zone,
            zone_suffix,
            activator,
            sorcery_speed,
            instant_speed,
            is_forecast,
            kind,
            condition,
            cost_reduction,
            target_req_override,
            same_controller_targets,
            chain,
        })
    }

    fn read_ability_chain(
        &mut self,
        spec: &str,
        is_activated: bool,
        cost: Option<&str>,
        drop_keys: &[&str],
    ) -> Option<Chain> {
        let stripped: Vec<&str> = spec
            .split(" | ")
            .filter(|part| {
                let p = part.trim_start();
                !drop_keys.iter().any(|key| p.starts_with(key))
            })
            .collect();
        let mut chain = Chain::default();
        self.cost_sacrifices_self = is_activated
            && cost.is_some_and(|c| cost_parts(c).iter().any(|t| Self::is_sac_self_token(t)));
        self.cost_sacrifices_other =
            cost.is_some_and(|c| cost_parts(c).iter().any(|t| Self::is_sac_other_token(t)));
        self.cost_discards_single_for_cost = cost.is_some_and(Self::wave28b_cost_discards_single);
        self.cost_beholds_or_reveals = !is_activated && Self::cost_raw_beholds_or_reveals(cost);
        (
            self.cost_sacrifices_all,
            self.cost_sacrifices_x,
            self.cost_returns_self,
        ) = Self::wave29b_cost_flags(cost);
        let chain_ok = self.chain(&stripped.join(" | "), &mut chain);
        self.cost_sacrifices_self = false;
        self.cost_sacrifices_other = false;
        self.cost_discards_single_for_cost = false;
        self.cost_beholds_or_reveals = false;
        self.cost_sacrifices_all = false;
        self.cost_sacrifices_x = false;
        self.cost_returns_self = false;
        if chain_ok.is_none() {
            if self.unclaimed.borrow().is_none() {
                let api = Params::parse(spec).map_or_else(|| "ability".to_string(), |(a, _)| a);
                self.note(format!("an unreadable `{api}` ability chain"));
            }
            return None;
        }
        if chain.effects.is_empty() {
            return self.deny("an ability that reads as no effect at all".to_string());
        }
        Some(chain)
    }

    fn activation_zone_expr(
        &self,
        activation_zone: Option<&str>,
        activator: ActivatorRestriction,
        modal: bool,
    ) -> Option<(bool, &'static str)> {
        match (activation_zone, activator) {
            (None | Some("Battlefield"), ActivatorRestriction::Default) => Some((false, "")),
            (None | Some("Battlefield"), ActivatorRestriction::AnyPlayer) => {
                Some((false, ", zone = ActivationZone::BattlefieldAnyPlayer"))
            }
            (None | Some("Battlefield"), ActivatorRestriction::Opponent) => {
                Some((false, ", zone = ActivationZone::BattlefieldOpponent"))
            }
            (None | Some("Battlefield"), ActivatorRestriction::Owner) => {
                Some((false, ", zone = ActivationZone::BattlefieldOwner"))
            }
            (None | Some("Battlefield"), ActivatorRestriction::AttachedController) => Some((
                false,
                ", zone = ActivationZone::BattlefieldAttachedController",
            )),
            (Some("Graveyard"), ActivatorRestriction::Default) => {
                Some((true, ", zone = ActivationZone::Graveyard"))
            }
            (Some("Hand"), ActivatorRestriction::Default) => {
                Some((true, ", zone = ActivationZone::Hand"))
            }
            (Some("Command"), ActivatorRestriction::Default) => {
                Some((true, ", zone = ActivationZone::Command"))
            }
            (
                Some("Battlefield,Command" | "Command,Battlefield"),
                ActivatorRestriction::Default,
            ) => Some((true, ", zone = ActivationZone::BattlefieldOrCommand")),
            (Some("Hand,Command" | "Command,Hand"), ActivatorRestriction::Default) => {
                Some((true, ", zone = ActivationZone::HandOrCommand"))
            }
            (Some(zone), ActivatorRestriction::AnyPlayer) => {
                self.deny(format!("`Activator$ Player` in `ActivationZone$ {zone}`"))
            }
            (Some(zone), ActivatorRestriction::Default) => {
                let prefix = if modal { "a modal " } else { "an " };
                self.deny(format!(
                    "{prefix}activated ability in `ActivationZone$ {zone}`"
                ))
            }
            (Some(zone), _) => self.deny(format!("`Activator$` in `ActivationZone$ {zone}`")),
        }
    }

    fn activated_target_suffix(chain: &Chain) -> String {
        chain.target.as_ref().map_or_default(|t| {
            if let Some(max) = chain.target_up_to {
                if max == 1 {
                    format!(", targets = Some(TargetReq::up_to_one({t}))")
                } else {
                    format!(", targets = Some(TargetReq::up_to({t}, {max}))")
                }
            } else if let Some(n) = chain.target_exact {
                format!(", targets = Some(TargetReq::exactly({t}, {n}))")
            } else if let Some((min, max)) = chain.target_range {
                format!(
                    ", targets = Some({})",
                    Self::wave28a_format_range_req(t, min, max)
                )
            } else if chain.target_up_to_x {
                format!(", targets = Some(TargetReq::up_to_x({t}))")
            } else if chain.target_is_x {
                format!(", targets = Some(TargetReq::x_targets({t}))")
            } else {
                format!(", target = Some({t})")
            }
        })
    }

    fn activated_timing_suffix(
        &self,
        spec: &ExtractedAbilitySpec,
        mana_ability: bool,
    ) -> Option<&'static str> {
        if !spec.zone_suffix.is_empty()
            && mana_ability
            && !matches!(
                spec.zone_suffix,
                ", zone = ActivationZone::Hand" | ", zone = ActivationZone::Graveyard"
            )
        {
            return self.deny("a non-default activation zone on a mana ability".to_string());
        }
        if spec.sorcery_speed && mana_ability {
            return self.deny("`SorcerySpeed$ True` on a mana ability".to_string());
        }
        if spec.instant_speed && (!mana_ability || spec.sorcery_speed) {
            return self.deny("`InstantSpeed$ True` on a non-mana ability".to_string());
        }
        Some(if spec.sorcery_speed {
            ", timing = ActivationTiming::SorcerySpeed"
        } else if spec.instant_speed {
            ", timing = ActivationTiming::PriorityOnly"
        } else {
            ""
        })
    }

    fn resolve_spec_target_suffix(&self, spec: &ExtractedAbilitySpec) -> Option<String> {
        if let Some(req) = spec.target_req_override.as_deref() {
            let req = if spec.same_controller_targets {
                let t = spec.chain.target.as_deref()?;
                let multi = !req.starts_with("TargetReq::one(")
                    && !req.starts_with("TargetReq::up_to_one(");
                self.apply_same_controller_target_req(req, t, multi)?
            } else {
                req.to_string()
            };
            Some(format!(", targets = Some({req})"))
        } else if let Some(req) = Self::target_req_expr(&spec.chain) {
            Some(format!(", targets = Some({req})"))
        } else if spec.same_controller_targets {
            self.deny(
                "`TargetsWithSameController$` on a non-multi-object target requirement".into(),
            )
        } else {
            Some(String::new())
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "dispatches all activated ability parts and X/min rules"
    )]
    fn build_activated_parts(&mut self, spec: &ExtractedAbilitySpec) -> Option<ParsedActivated> {
        if spec.kind == ActivationKind::Loyalty {
            return self.build_loyalty_parts(spec);
        }
        let Some(cost_raw) = spec.cost.as_deref() else {
            return self.deny("an activated ability with no `Cost$`".to_string());
        };
        let stripped_xmin = self.strip_activation_x_min(cost_raw)?;
        let x_min = self.merge_x_min(&stripped_xmin, spec.x_min_param)?;
        self.in_forecast_activation = spec.is_forecast;
        self.allow_defined_cost_x =
            self.wave30c_allows_defined_cost_x(&spec.chain.effects, spec.non_battlefield_zone);
        let cost = self.parse_activated_cost(&stripped_xmin.cost, spec.activation_zone.as_deref());
        self.in_forecast_activation = false;
        self.allow_defined_cost_x = false;
        let cost = cost?;
        if x_min > 0 && !Self::cost_has_variable_x(&cost) {
            return self.deny("unused `XMin` in activation cost".to_string());
        }
        if x_min > 0 && spec.alternate_cost.is_some() {
            return self.deny("`XMin` beside `AlternateCost$`".to_string());
        }
        let effects = format!("&[{}]", spec.chain.effects.join(", "));
        let second_targets = Self::second_target_req_expr(&spec.chain)
            .map_or_default(|req| format!(", second_targets = Some({req})"));
        let zone = spec.zone_suffix;
        let has_targets = spec.chain.target.is_some() || spec.chain.second_target.is_some();
        let mana_ability =
            spec.chain.could_add_mana && !spec.chain.moves_library && spec.chain.target.is_none();
        self.reject_non_mana_counter_x(&cost, mana_ability, has_targets)?;
        let timing = self.activated_timing_suffix(spec, mana_ability)?;
        if spec.chain.target_is_x && !Self::cost_has_variable_x(&cost) {
            return self.deny("`TargetMin$/TargetMax$ X` on an activated ability".to_string());
        }
        let target = if spec.target_req_override.is_some()
            || spec.same_controller_targets
            || spec.chain.target_constraint.is_some()
        {
            self.resolve_spec_target_suffix(spec)?
        } else {
            Self::activated_target_suffix(&spec.chain)
        };
        let limit_str = self.activation_limit(&spec.limit)?;
        let x_value = self.wave30c_activated_x_or_at_least(
            x_min,
            &cost,
            &spec.chain.effects,
            spec.non_battlefield_zone,
        );
        let condition = &spec.condition;
        let cost_reduction = spec.cost_reduction.as_str();
        let extras = format!(
            "{timing}{zone}{target}{second_targets}{limit_str}{x_value}{condition}{cost_reduction}"
        );
        let ability_expr = Self::format_activated_macro(mana_ability, &cost, &effects, &extras);
        let alternate_ability_expr = match spec.alternate_cost.as_deref() {
            Some(alt) => {
                let alt_cost = self.parse_activated_cost(alt, spec.activation_zone.as_deref())?;
                self.reject_non_mana_counter_x(&alt_cost, mana_ability, has_targets)?;
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
            && spec.activator == ActivatorRestriction::Default
            && !spec.sorcery_speed
            && !spec.instant_speed
            && limit_str.is_empty()
            && x_min == 0
            && x_value.is_empty()
            && condition.is_empty()
            && spec.cost_reduction.is_empty()
            && spec.target_req_override.is_none()
            && !spec.same_controller_targets
            && spec.chain.target_constraint.is_none()
            && spec.chain.second_target.is_none()
            && spec.chain.target_up_to.is_none()
            && spec.chain.target_exact.is_none()
            && spec.chain.target_range.is_none()
            && !spec.chain.target_is_x
        {
            spec.chain.target.clone()
        } else {
            None
        };
        Some(ParsedActivated {
            cost,
            effects,
            mana_ability,
            is_loyalty: false,
            extras,
            ability_expr,
            alternate_ability_expr,
            non_battlefield_zone: spec.non_battlefield_zone,
            simple_target,
        })
    }

    fn build_loyalty_parts(&self, spec: &ExtractedAbilitySpec) -> Option<ParsedActivated> {
        if spec.alternate_cost.is_some() {
            return self.deny("`AlternateCost$` on a loyalty ability".to_string());
        }
        match spec.limit {
            ParsedLimit::Unlimited => {}
            ParsedLimit::PerTurnRaw(_) => {
                return self.deny("`ActivationLimit$` on a loyalty ability".to_string());
            }
            ParsedLimit::Exhaust => {
                return self.deny("`Exhaust$`/`GameActivationLimit$` on a loyalty ability".into());
            }
            ParsedLimit::Boast => {
                return self.deny("`Boast$` on a loyalty ability".to_string());
            }
            ParsedLimit::OncePerPermanent => {
                return self.deny("`PowerUp$` on a loyalty ability".to_string());
            }
        }
        if spec.non_battlefield_zone
            || spec
                .activation_zone
                .as_deref()
                .is_some_and(|zone| zone != "Battlefield")
        {
            return self
                .deny("a non-battlefield `ActivationZone$` on a loyalty ability".to_string());
        }
        match spec.activator {
            ActivatorRestriction::Default => {}
            ActivatorRestriction::AnyPlayer => {
                return self.deny("`Activator$ Player` on a loyalty ability".to_string());
            }
            _ => return self.deny("`Activator$` on a loyalty ability".to_string()),
        }
        if spec.sorcery_speed {
            return self.deny("`SorcerySpeed$` on a loyalty ability".to_string());
        }
        if spec.instant_speed {
            return self.deny("`InstantSpeed$` on a loyalty ability".to_string());
        }
        if !spec.condition.is_empty() {
            return self.deny("an activation condition on a loyalty ability".to_string());
        }
        if !spec.cost_reduction.is_empty() {
            return self.deny("`ReduceCost$` on a loyalty ability".to_string());
        }
        if spec.chain.target_is_x {
            return self.deny("`TargetMin$/TargetMax$ X` on a loyalty ability".to_string());
        }
        let Some(raw_cost) = spec.cost.as_deref() else {
            return self.deny("a loyalty ability with no `Cost$`".to_string());
        };
        let delta = self.parse_loyalty_cost(raw_cost)?;
        let effects = format!("&[{}]", spec.chain.effects.join(", "));
        let targets = self.resolve_spec_target_suffix(spec)?;
        let second_targets = Self::second_target_req_expr(&spec.chain)
            .map_or_default(|req| format!(", second_targets = Some({req})"));
        let extras = format!("{targets}{second_targets}");
        // CR 605.1a: a loyalty ability is never a mana ability even when its effect adds mana.
        let ability_expr = format!("loyalty!({delta}, {effects}{extras})");
        Some(ParsedActivated {
            cost: delta.to_string(),
            effects,
            mana_ability: false,
            is_loyalty: true,
            extras,
            ability_expr,
            alternate_ability_expr: None,
            non_battlefield_zone: false,
            simple_target: None,
        })
    }

    fn parse_loyalty_cost(&self, raw: &str) -> Option<i8> {
        let tokens = cost_parts(raw);
        let [token] = tokens.as_slice() else {
            return self.deny(format!("a non-loyalty cost `{raw}` on a loyalty ability"));
        };
        let (sign, inner) = if let Some(inner) = token
            .strip_prefix("AddCounter<")
            .and_then(|t| t.strip_suffix('>'))
        {
            (1_i8, inner)
        } else if let Some(inner) = token
            .strip_prefix("SubCounter<")
            .and_then(|t| t.strip_suffix('>'))
        {
            (-1_i8, inner)
        } else {
            return self.deny(format!("a non-loyalty cost `{raw}` on a loyalty ability"));
        };
        let Some((n, kind)) = inner.split_once('/') else {
            return self.deny(format!("a non-loyalty cost `{raw}` on a loyalty ability"));
        };
        let mut kind_fields = kind.splitn(3, '/');
        if kind_fields.next() != Some("LOYALTY")
            || kind_fields
                .next()
                .is_some_and(|t| !matches!(t, "CARDNAME" | "NICKNAME"))
        {
            return self.deny(format!("a non-loyalty cost `{raw}` on a loyalty ability"));
        }
        if n == "X" {
            return self.deny("a variable-X loyalty cost".to_string());
        }
        // Cap magnitude at 127 so `-cost` on `i8` can never overflow in the engine.
        let Ok(magnitude) = n.parse::<i8>() else {
            return self.deny(format!("a loyalty cost count `{n}`"));
        };
        if magnitude < 0 {
            return self.deny(format!("a loyalty cost count `{n}`"));
        }
        Some(sign * magnitude)
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
        limit: &ParsedLimit,
        condition: &str,
        effects: &str,
        chain: &Chain,
        target_req_override: Option<&str>,
        same_controller_targets: bool,
        second_targets: &str,
    ) -> Option<()> {
        if *limit != ParsedLimit::Unlimited {
            // A spell is cast, not activated, so there is nothing for the
            // key to restrict and dropping it quietly would be the
            // unclaimed-parameter fault one level down.
            return self.deny("`ActivationLimit$` on a spell line".to_string());
        }
        if let Some(raw) = cost {
            self.spell_mana_cost(raw)?;
        }
        let target_req = if let Some(req) = target_req_override {
            let mut req = req.to_string();
            if same_controller_targets {
                let t = chain.target.as_deref()?;
                let multi = !req.starts_with("TargetReq::one(")
                    && !req.starts_with("TargetReq::up_to_one(");
                req = self.apply_same_controller_target_req(&req, t, multi)?;
            }
            Some(req)
        } else if let Some(req) = Self::target_req_expr(chain) {
            Some(req)
        } else {
            if same_controller_targets {
                return self.deny(
                    "`TargetsWithSameController$` on a non-multi-object target requirement".into(),
                );
            }
            None
        };
        let targets = target_req.map_or_default(|req| format!(", targets = Some({req})"));
        self.body.abilities.push(format!(
            "spell!({effects}{targets}{second_targets}{condition})"
        ));
        Some(())
    }

    /// Validate the spell's cost tokens and record any non-mana mandatory
    /// additional costs (`FaceDef::mandatory_additional_costs`) that
    /// `cast_wizard::paid_as_a_mandatory_additional_cost` pays.
    pub(super) fn spell_mana_cost(&mut self, raw: &str) -> Option<()> {
        let mut extra = Vec::new();
        for token in cost_parts(raw) {
            let is_mana = token.chars().all(|c| c.is_ascii_digit())
                || matches!(
                    token.as_str(),
                    "W" | "U" | "B" | "R" | "G" | "C" | "S" | "X"
                )
                || hybrid_pair(&token).is_some()
                || super::mana::special_mana_symbol(&token).is_some();
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
        let has_wave14a = extra.iter().any(|t| Self::is_wave14a_cost_token(t));
        let prev_alt = self.in_alternative_cost;
        self.in_alternative_cost = true;
        let parsed = if has_wave14a {
            self.parse_chunked_cost_tokens(extra.clone(), false, false, true)
        } else {
            self.cost_pieces(&extra.join(" "))
        };
        self.in_alternative_cost = prev_alt;
        let Some((mana, parts)) = parsed else {
            if self.unclaimed.borrow().is_some() {
                return None;
            }
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
                || part == "Forage"
                || part == "DiscardRandom"
                || part == "DiscardHand"
                || part == "DiscardLastDrawnThisTurn"
                || part.starts_with("DealDamageToYou(")
                || part.starts_with("PayLife(")
                || part.starts_with("PayEnergy(")
                || part.starts_with("Sacrifice(")
                || part.starts_with("SacrificeX(")
                || part.starts_with("SacrificeAll(")
                || part.starts_with("ReturnToHand(")
                || part.starts_with("ExilePermanent(")
                || part.starts_with("ExileFromHand(")
                || part.starts_with("ExileFromGraveyard(")
                || part.starts_with("ExileFromGraveyardX(")
                || part.starts_with("ExileTopOfLibrary(")
                || part.starts_with("Discard(")
                || part.starts_with("DiscardX(")
                || part.starts_with("TapOther(")
                || part.starts_with("TapX(")
                || part.starts_with("TapN {")
                || part.starts_with("RemoveAnyCounter {")
                || part.starts_with("RemoveAnyCounterX {")
                || part.starts_with("Behold(")
                || part.starts_with("BeholdAndExile(")
                || part.starts_with("CollectEvidence(")
                || part.starts_with("RevealFromHand(")
                || part.starts_with("RevealFromHandX(")
                || part.starts_with("PutCounterOnOther {")
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
        let base = if let Some(max) = chain.target_up_to {
            if max == 1 {
                format!("TargetReq::up_to_one({t})")
            } else {
                format!("TargetReq::up_to({t}, {max})")
            }
        } else if let Some(n) = chain.target_exact {
            format!("TargetReq::exactly({t}, {n})")
        } else if let Some((min, max)) = chain.target_range {
            Self::wave28a_format_range_req(t, min, max)
        } else if chain.target_up_to_x {
            format!("TargetReq::up_to_x({t})")
        } else if chain.target_is_x {
            format!("TargetReq::x_targets({t})")
        } else {
            format!("TargetReq::one({t})")
        };
        Some(match chain.target_constraint {
            Some(constraint) => format!("{base}.with_constraint({constraint})"),
            None => base,
        })
    }

    /// Formats a `TargetReq` from a chain's second (and optional third) target fields.
    pub(super) fn second_target_req_expr(chain: &Chain) -> Option<String> {
        Self::wave27c_extra_target_req_expr(chain)
    }

    /// Reads the `Choices$` list of a `Charm` ability into [`CharmMode`] tuples.
    pub(super) fn read_charm_modes(&mut self, choices: &str) -> Option<Vec<CharmMode>> {
        self.read_charm_modes_with(choices, false, false)
    }

    #[allow(clippy::too_many_lines)]
    fn read_charm_modes_with(
        &mut self,
        choices: &str,
        allow_mode_cost: bool,
        allow_divided_counters: bool,
    ) -> Option<Vec<CharmMode>> {
        let mut modes = Vec::new();
        for name in choices.split(',').map(str::trim) {
            let Some(line) = self.svars.get(name).cloned() else {
                return self.deny(format!("charm choice `{name}` with no `SVar`"));
            };
            let mut mode_cost_raw: Option<String> = None;
            let mut mode_same_controller = false;
            let line_has_valid_tgts = line
                .split(" | ")
                .any(|part| part.trim_start().starts_with("ValidTgts$"));
            let mode_is_divided_counters = allow_divided_counters
                && Params::parse(&line)
                    .is_some_and(|(api, dp)| api == "PutCounter" && dp.has("DividedAsYouChoose"));
            let mut kept_parts: Vec<String> = Vec::new();
            for part in line.split(" | ") {
                let trimmed = part.trim_start();
                if let Some(val) = trimmed.strip_prefix("ModeCost$") {
                    if mode_cost_raw.is_some() {
                        return self.deny(format!("multiple `ModeCost$` keys in mode `{name}`"));
                    }
                    mode_cost_raw = Some(val.trim().to_string());
                } else if mode_is_divided_counters
                    && let Some(val) = trimmed.strip_prefix("TargetsWithSameController$")
                {
                    match val.trim() {
                        "True" if line_has_valid_tgts => mode_same_controller = true,
                        "False" => {}
                        other => {
                            return self.deny(format!(
                                "`TargetsWithSameController$ {other}` in charm mode `{name}`"
                            ));
                        }
                    }
                } else if line_has_valid_tgts
                    && trimmed.strip_prefix("UnlessPayer$").map(str::trim)
                        == Some("ThisTargetedController")
                {
                    kept_parts.push("UnlessPayer$ TargetedController".to_string());
                } else {
                    kept_parts.push(part.to_string());
                }
            }
            let additional_cost = match mode_cost_raw {
                Some(raw) => {
                    if !allow_mode_cost {
                        return self.deny("`ModeCost$` on a non-Spree charm mode".to_string());
                    }
                    let mana = self.parse_fixed_spree_mana(&raw)?;
                    format!(", additional_cost = Some(mana!(\"{mana}\"))")
                }
                None if allow_mode_cost => {
                    return self.deny(format!("a `Spree` mode `{name}` without `ModeCost$`"));
                }
                None => String::new(),
            };
            let stripped = kept_parts.join(" | ");
            if allow_divided_counters
                && let Some((api, dp)) = Params::parse(&stripped)
                && api == "PutCounter"
                && dp.has("DividedAsYouChoose")
            {
                let (effects, target_spec, mut req_expr) = self.read_divided_counters(dp)?;
                if mode_same_controller {
                    let multi = !req_expr.starts_with("TargetReq::one(")
                        && !req_expr.starts_with("TargetReq::up_to_one(");
                    req_expr =
                        self.apply_same_controller_target_req(&req_expr, &target_spec, multi)?;
                }
                modes.push((
                    format!(
                        "mode!(&[{}], targets = Some({req_expr}){additional_cost})",
                        effects.join(", ")
                    ),
                    Some(target_spec),
                    false,
                    false,
                ));
                continue;
            }
            if allow_divided_counters
                && let Some((api, dp)) = Params::parse(&stripped)
                && api == "DealDamage"
                && dp.has("DividedAsYouChoose")
            {
                let (effects, target_spec, mut req_expr, second_req) =
                    self.wave28a_read_divided_damage(dp, false)?;
                if mode_same_controller {
                    let multi = !req_expr.starts_with("TargetReq::one(")
                        && !req_expr.starts_with("TargetReq::up_to_one(");
                    req_expr =
                        self.apply_same_controller_target_req(&req_expr, &target_spec, multi)?;
                }
                let has_second = second_req.is_some();
                let second =
                    second_req.map_or_default(|req| format!(", second_targets = Some({req})"));
                modes.push((
                    format!(
                        "mode!(&[{}], targets = Some({req_expr}){second}{additional_cost})",
                        effects.join(", ")
                    ),
                    Some(target_spec),
                    false,
                    has_second,
                ));
                continue;
            }
            let mut chain = Chain::default();
            if self.chain(&stripped, &mut chain).is_none() {
                if self.unclaimed.borrow().is_none() {
                    let api =
                        Params::parse(&stripped).map_or_else(|| "mode".to_string(), |(a, _)| a);
                    self.note(format!("an unreadable `{api}` charm mode"));
                }
                return None;
            }
            if chain.effects.is_empty() {
                return self.deny("a charm mode that reads as no effect at all".to_string());
            }
            let targets = Self::target_req_expr(&chain)
                .map_or_default(|req| format!(", targets = Some({req})"));
            if chain.third_target.is_some() {
                return None;
            }
            let has_second = chain.second_target.is_some();
            let second = Self::second_target_req_expr(&chain)
                .map_or_default(|req| format!(", second_targets = Some({req})"));
            modes.push((
                format!(
                    "mode!(&[{}]{targets}{second}{additional_cost})",
                    chain.effects.join(", ")
                ),
                chain.target,
                has_second,
                chain.could_add_mana,
            ));
        }
        Some(modes)
    }

    fn parse_fixed_spree_mana(&self, raw: &str) -> Option<String> {
        let mut mana = String::new();
        for token in cost_parts(raw) {
            if token == "X" || token.starts_with("XMin") {
                return self.deny(format!("variable `X` in `ModeCost$ {raw}`"));
            }
            if token.chars().all(|c| c.is_ascii_digit())
                || matches!(token.as_str(), "W" | "U" | "B" | "R" | "G" | "C" | "S")
            {
                mana.push('{');
                mana.push_str(&token);
                mana.push('}');
            } else if let Some(pair) =
                hybrid_pair(&token).or_else(|| super::mana::special_mana_symbol(&token))
            {
                mana.push_str(&pair);
            } else {
                return self.deny(format!("non-mana `ModeCost$ {raw}`"));
            }
        }
        if mana.is_empty() || baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
            return self.deny(format!("invalid `ModeCost$ {raw}`"));
        }
        Some(mana)
    }

    /// "Choose one —" (CR 700.2): `SP$ Charm | Choices$ A,B` is a modal
    /// spell whose modes are the named `SVar`s, each read as the chain it
    /// is and each targeting for itself, since a target a mode names is
    /// chosen only when that mode is (CR 700.2c).
    #[allow(clippy::too_many_lines)]
    pub(super) fn charm(&mut self, spec: &str) -> Option<()> {
        let Some((_, mut p)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        p.drop_prose();
        let raw_cost = p.take("Cost");
        match p.take("Defined").as_deref() {
            None | Some("You") => {}
            Some(who) => return self.deny(format!("a charm with `Defined$ {who}`")),
        }
        let Some(choices) = p.take("Choices") else {
            return self.deny("a charm with no `Choices$`".to_string());
        };
        let can_repeat_modes = match p.take("CanRepeatModes").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("a charm with `CanRepeatModes$ {other}`")),
        };
        let mut optional_cost_entwine = false;
        let max: u8 = match p.take("CharmNum").as_deref() {
            None => 1,
            Some(raw) if !can_repeat_modes && self.wave16b_is_optional_cost_entwine_charm(raw) => {
                optional_cost_entwine = true;
                Self::wave16b_claim_entwine_charm_desc(&mut p);
                1
            }
            Some(raw) => match raw.trim().parse::<u8>() {
                Ok(n) if n > 0 => n,
                _ => return self.deny(format!("a charm choosing `{raw}`")),
            },
        };
        let min: u8 = match p.take("MinCharmNum").as_deref() {
            None => max,
            Some(raw) => match raw.trim().parse::<u8>() {
                Ok(n) if n > 0 || (can_repeat_modes && n == 0) => n,
                _ => return self.deny(format!("a charm with `MinCharmNum$ {raw}`")),
            },
        };
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `Charm.{key}`"));
            return None;
        }
        if min > max || (!can_repeat_modes && min == 0) {
            return self.deny(format!("a charm with min {min} > max {max}"));
        }
        if can_repeat_modes && max > 8 {
            return self.deny(format!("a repeating charm choosing `{max}`"));
        }
        let has_spree = self.body.keywords.iter().any(|k| k == "KeywordSet::SPREE");
        if has_spree && (max == 1 || can_repeat_modes) {
            return self.deny("a `Spree` charm with unsupported mode selection".to_string());
        }
        if let Some(raw) = raw_cost.as_deref() {
            self.spell_mana_cost(raw)?;
        }
        self.cost_sacrifices_other = raw_cost
            .as_deref()
            .is_some_and(|c| cost_parts(c).iter().any(|t| Self::is_sac_other_token(t)));
        let allow_divided_counters = min == 1 && max == 1 && !can_repeat_modes;
        let parsed =
            self.read_charm_modes_with(&choices, has_spree && max > 1, allow_divided_counters);
        self.cost_sacrifices_other = false;
        let parsed = parsed?;
        if parsed.len() < 2 {
            return self.deny("a charm with fewer than two modes".to_string());
        }
        if !can_repeat_modes && usize::from(min) > parsed.len() {
            return self.deny("a charm choosing more modes than it has".to_string());
        }
        // Mirror `crates/cards/src/lints/shapes.rs::modes_fault` for multi-mode
        // and repeating modal spells so no generated card violates the pool invariant.
        if max > 1 || can_repeat_modes {
            if parsed.len() > 8 {
                return self.deny("a multi-mode charm with more than eight modes".to_string());
            }
            let total_target_clauses: usize = if can_repeat_modes {
                let per_mode = parsed
                    .iter()
                    .map(|(_, tgt, has_second, _)| {
                        usize::from(tgt.is_some()) + usize::from(*has_second)
                    })
                    .max()
                    .unwrap_or(0);
                usize::from(max).saturating_mul(per_mode)
            } else {
                parsed
                    .iter()
                    .map(|(_, tgt, has_second, _)| {
                        usize::from(tgt.is_some()) + usize::from(*has_second)
                    })
                    .sum()
            };
            if total_target_clauses > 16 {
                return self
                    .deny("a multi-mode charm with more than sixteen target clauses".to_string());
            }
        }
        let choose = match (min, max) {
            (1, 1) => "ModeCount::ONE".to_string(),
            (2, 2) => "ModeCount::TWO".to_string(),
            (3, 3) => "ModeCount::THREE".to_string(),
            (4, 4) => "ModeCount::FOUR".to_string(),
            (5, 5) => "ModeCount::FIVE".to_string(),
            _ => format!("ModeCount {{ min: {min}, max: {max} }}"),
        };
        let modes: Vec<String> = parsed.into_iter().map(|(m, _, _, _)| m).collect();
        if optional_cost_entwine {
            let has_spree = self.body.keywords.iter().any(|k| k == "KeywordSet::SPREE");
            if modes.len() != 2
                || min != 1
                || max != 1
                || has_spree
                || self.body.optional_cost_grants_flash
                || self
                    .body
                    .abilities
                    .iter()
                    .any(|a| a.contains("OptionalCostPaid"))
                || modes.iter().any(|m| m.contains("OptionalCostPaid"))
            {
                return self.deny("an unsupported optional-cost modal count".to_string());
            }
            self.body.entwine = self.body.optional_cost.take();
        }
        if can_repeat_modes {
            self.body.abilities.push(format!(
                "AbilityDef::Modal {{ modes: &[{}], choose: {choose}, can_repeat_modes: true }}",
                modes.join(", ")
            ));
        } else {
            self.body.abilities.push(format!(
                "AbilityDef::ModalSpell {{ choose: {choose}, modes: &[{}] }}",
                modes.join(", ")
            ));
        }
        Some(())
    }

    /// Claims `AdditionalDescription$` on `AB$ Charm` ONLY when it is a
    /// verified pure restatement of already-claimed timing, limit, condition,
    /// or `XMin1` parameters. Any semantic/conditional rule stays unclaimed so
    /// the caller refuses it as `unclaimed parameter Charm.AdditionalDescription`.
    fn claim_modal_additional_description(
        p: &mut Params,
        sorcery_speed: bool,
        limit: &ParsedLimit,
        has_condition: bool,
        x_min: Option<u32>,
    ) {
        let Some(raw) = p.peek("AdditionalDescription") else {
            return;
        };
        let norm = raw
            .trim()
            .trim_start_matches('.')
            .trim()
            .trim_end_matches('.')
            .trim()
            .to_ascii_lowercase();
        let once_each_turn = matches!(limit, ParsedLimit::PerTurnRaw(s) if s == "1");
        let is_exhaust = matches!(*limit, ParsedLimit::Exhaust | ParsedLimit::OncePerPermanent);
        let pure_restatement = match norm.as_str() {
            "activate only as a sorcery" => sorcery_speed,
            "activate only once each turn" | "activate this ability only once each turn" => {
                once_each_turn
            }
            "activate only once" | "activate this ability only once" => is_exhaust,
            "activate only as a sorcery and only once"
            | "activate only once and only as a sorcery" => sorcery_speed && is_exhaust,
            "activate only as a sorcery and only once each turn"
            | "activate only once each turn and only as a sorcery" => {
                sorcery_speed && once_each_turn
            }
            "x can't be 0" | "x can't be zero" => x_min == Some(1),
            other if other.starts_with("activate only if ") && has_condition => {
                const FORBIDDEN: &[&str] = &[
                    "sorcery",
                    "once",
                    "turn",
                    "choose",
                    "mode",
                    "instead",
                    "x can",
                    "different",
                    "kicked",
                    "commander",
                    " and ",
                ];
                !FORBIDDEN.iter().any(|word| other.contains(word))
            }
            _ => false,
        };
        if pure_restatement {
            p.take("AdditionalDescription");
        }
    }

    /// `AB$ Charm | Cost$ ... | Choices$ A,B`: a modal activated ability
    /// (`modal_activated!`). Choose-one only; non-mana only.
    fn modal_activated(&mut self, spec: &str) -> Option<()> {
        let Some((_, mut p)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        let line_prose = Self::params_prose(&p);
        p.drop_prose();
        let Some(cost_raw) = p.take("Cost") else {
            return self.deny("a modal activated ability with no `Cost$`".to_string());
        };
        match p.take("Defined").as_deref() {
            None | Some("You") => {}
            Some(who) => {
                return self.deny(format!("a modal activated ability with `Defined$ {who}`"));
            }
        }
        let Some(choices) = p.take("Choices") else {
            return self.deny("a charm with no `Choices$`".to_string());
        };
        match p.take("CharmNum").as_deref() {
            None | Some("1") => {}
            Some(n) => return self.deny(format!("a modal activated ability choosing `{n}`")),
        }
        match p.take("MinCharmNum").as_deref() {
            None | Some("1") => {}
            Some(n) => {
                return self.deny(format!("a modal activated ability with `MinCharmNum$ {n}`"));
            }
        }
        let (limit, _) = self.take_activation_limit(&mut p, true)?;
        let x_min_param = self.take_x_min_param(&mut p, true, false)?;
        let activator = self.parse_activator(p.take("Activator").as_deref(), true)?;
        let activation_zone = p.take("ActivationZone");
        let cost_reduction = self.take_reduce_cost(
            &mut p,
            true,
            activation_zone.as_deref(),
            limit == ParsedLimit::OncePerPermanent,
        )?;
        let sorcery_speed = match p.take("SorcerySpeed").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`SorcerySpeed$ {other}`")),
        };
        let (_non_battlefield, zone) =
            self.activation_zone_expr(activation_zone.as_deref(), activator, true)?;
        let mut condition = self.condition_with_prose(&mut p, &line_prose)?;
        let _restrictions = self.activation_restriction(&mut p, true, &mut condition)?;
        let stripped_xmin = self.strip_activation_x_min(&cost_raw)?;
        let x_min = self.merge_x_min(&stripped_xmin, x_min_param)?;
        Self::claim_modal_additional_description(
            &mut p,
            sorcery_speed,
            &limit,
            !condition.is_empty(),
            (x_min > 0).then_some(x_min),
        );
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Charm.{key}`"));
        }
        let cost = self.parse_activated_cost(&stripped_xmin.cost, activation_zone.as_deref())?;
        if x_min > 0 && !Self::cost_has_variable_x(&cost) {
            return self.deny("unused `XMin` in activation cost".to_string());
        }
        self.reject_non_mana_counter_x(&cost, false, true)?;
        self.cost_sacrifices_self = cost_parts(&stripped_xmin.cost)
            .iter()
            .any(|t| Self::is_sac_self_token(t));
        self.cost_sacrifices_other = cost_parts(&stripped_xmin.cost)
            .iter()
            .any(|t| Self::is_sac_other_token(t));
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
        let limit = self.activation_limit(&limit)?;
        let x_value = if x_min > 0 {
            format!(", x_value = ActivationX::AtLeast({x_min})")
        } else {
            String::new()
        };
        let reduction_suffix = cost_reduction.as_str();
        let modes: Vec<String> = parsed.into_iter().map(|(m, _, _, _)| m).collect();
        self.body.abilities.push(format!(
            "modal_activated!({cost}, &[{}]{timing}{zone}{limit}{x_value}{condition}{reduction_suffix})",
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
            "ExileFromGrave" if in_graveyard && matches!(spec, "CARDNAME" | "NICKNAME") => {
                Some("ExileSelf")
            }
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

    fn reject_non_mana_counter_x(
        &self,
        cost: &str,
        mana_ability: bool,
        has_targets: bool,
    ) -> Option<()> {
        if !mana_ability
            && cost.contains("RemoveCounterSelfX {")
            && (has_targets || !cost.contains("ExileSelf"))
        {
            return self.deny("`SubCounter<X/...>` on a non-mana activated ability".to_string());
        }
        Some(())
    }

    pub(super) fn parse_activated_cost(
        &mut self,
        raw: &str,
        activation_zone: Option<&str>,
    ) -> Option<String> {
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
        let has_wave14a = tokens.iter().any(|t| Self::is_wave14a_cost_token(t));
        if !in_graveyard && !in_hand && !has_nickname_exert && !has_wave14a {
            return self.cost_expr(raw);
        }
        let (mana, parts) = self.parse_chunked_cost_tokens(tokens, in_graveyard, in_hand, false)?;
        Some(crate::body::cost_literal(&mana, &parts))
    }

    fn is_wave14a_cost_token(token: &str) -> bool {
        if token == "Forage"
            || token.starts_with("DamageYou<")
            || token.starts_with("Behold<")
            || token.starts_with("BeholdExile<")
            || token.starts_with("CollectEvidence<")
        {
            return true;
        }
        matches!(
            super::object_cost(token),
            Some(("Discard" | "ExileFromGrave", body)) if body.split('/').next() == Some("X")
        )
    }

    /// Parses `Behold<1/<valid>[/<desc>]>` into `Behold(&<filter>)` (`CR 701.4a`).
    pub(super) fn parse_behold_cost_token(&mut self, token: &str) -> Option<String> {
        let Some(inner) = token
            .strip_prefix("Behold<")
            .and_then(|t| t.strip_suffix('>'))
        else {
            return self.deny(format!("cost `{token}`"));
        };
        let mut fields = inner.splitn(3, '/');
        let (Some(count_str), Some(valid)) = (fields.next(), fields.next()) else {
            return self.deny(format!("cost `{token}`"));
        };
        if count_str.trim() != "1" {
            return self.deny(format!("cost `{token}`"));
        }
        let valid = valid.trim();
        if valid.is_empty()
            || valid.split(',').any(|alt| {
                alt.split(['.', '+']).any(|atom| {
                    let atom = atom.trim();
                    matches!(
                        atom,
                        "CARDNAME" | "NICKNAME" | "Self" | "Other" | "StrictlyOther"
                    ) || atom.contains("Ctrl")
                        || atom.contains("Own")
                })
            })
        {
            return self.deny(format!("cost `{token}`"));
        }
        let expr = self.filter_expr(valid)?;
        let filter_name = self.body.filter_static("COST", &expr);
        Some(format!("Behold(&{filter_name})"))
    }

    /// Parses `CollectEvidence<N>` (`1..=255`) into `CollectEvidence(N)` (`CR 701.59a`).
    pub(super) fn parse_collect_evidence_cost_token(&self, token: &str) -> Option<String> {
        let Some(raw_n) = token
            .strip_prefix("CollectEvidence<")
            .and_then(|t| t.strip_suffix('>'))
        else {
            return self.deny(format!("cost `{token}`"));
        };
        let Ok(n) = raw_n.trim().parse::<u8>() else {
            return self.deny(format!("cost `{token}`"));
        };
        if n == 0 {
            return self.deny(format!("cost `{token}`"));
        }
        Some(format!("CollectEvidence({n})"))
    }

    fn damage_you_cost_token(token: &str) -> Option<Result<String, String>> {
        let raw_n = token.strip_prefix("DamageYou<")?.strip_suffix('>')?;
        Some(match raw_n.trim().parse::<u16>() {
            Ok(n) if n > 0 => Ok(format!("DealDamageToYou({n})")),
            _ => Err(format!("cost `{token}`")),
        })
    }

    fn wave14a_cost_part(&mut self, token: &str, for_mandatory_spell: bool) -> Wave14aCost {
        if token == "Forage" {
            return Wave14aCost::Matched("Forage".to_string());
        }
        if token.starts_with("Behold<") {
            return match self.parse_behold_cost_token(token) {
                Some(part) => Wave14aCost::Matched(part),
                None => Wave14aCost::Denied,
            };
        }
        if token.starts_with("BeholdExile<") {
            return match self.wave17c_parse_behold_exile_cost_token(token) {
                Some(part) => Wave14aCost::Matched(part),
                None => Wave14aCost::Denied,
            };
        }
        if token.starts_with("CollectEvidence<") {
            return match self.parse_collect_evidence_cost_token(token) {
                Some(part) => Wave14aCost::Matched(part),
                None => Wave14aCost::Denied,
            };
        }
        if let Some(res) = Self::damage_you_cost_token(token) {
            return match res {
                Ok(part) => Wave14aCost::Matched(part),
                Err(msg) => {
                    self.deny::<()>(msg);
                    Wave14aCost::Denied
                }
            };
        }
        let Some((kind, body)) = super::object_cost(token) else {
            return Wave14aCost::Pass;
        };
        if !matches!(kind, "Discard" | "ExileFromGrave") {
            return Wave14aCost::Pass;
        }
        let mut fields = body.splitn(3, '/');
        if fields.next() != Some("X") {
            return Wave14aCost::Pass;
        }
        let Some(spec) = fields.next() else {
            self.deny::<()>(format!("cost `{token}`"));
            return Wave14aCost::Denied;
        };
        if (self.in_alternative_cost && !for_mandatory_spell)
            || !self.has_x
            || self.trigger_mode.is_some()
            || self.svars.get("X").map(String::as_str) != Some("Count$xPaid")
            || self.oracle.to_ascii_lowercase().contains("x is ")
            || matches!(spec, "CARDNAME" | "NICKNAME" | "Hand" | "Random")
            || spec.contains("Ctrl")
            || (kind == "Discard" && spec.contains("Random"))
        {
            self.deny::<()>(format!("cost `{token}`"));
            return Wave14aCost::Denied;
        }
        let normalized = spec.replace(';', ",");
        let Some(expr) = self.filter_expr_in_zone(&normalized, true) else {
            return Wave14aCost::Denied;
        };
        let name = self.body.filter_static("COST", &expr);
        let variant = if kind == "Discard" {
            "DiscardX"
        } else {
            "ExileFromGraveyardX"
        };
        Wave14aCost::Matched(format!("{variant}(&{name})"))
    }

    fn parse_chunked_cost_tokens(
        &mut self,
        tokens: Vec<String>,
        in_graveyard: bool,
        in_hand: bool,
        for_mandatory_spell: bool,
    ) -> Option<(String, Vec<String>)> {
        let mut mana = String::new();
        let mut parts = Vec::new();
        let mut chunk = Vec::new();
        for token in tokens {
            let intercepted = Self::self_zone_cost_part(&token, in_graveyard, in_hand)
                .map(str::to_string)
                .or_else(|| {
                    (!in_graveyard && !in_hand && Self::exert_self_cost_part(&token))
                        .then(|| "ExertSelf".to_string())
                });
            let intercepted = match intercepted {
                Some(part) => Some(part),
                None => match self.wave14a_cost_part(&token, for_mandatory_spell) {
                    Wave14aCost::Matched(part) => Some(part),
                    Wave14aCost::Denied => return None,
                    Wave14aCost::Pass => None,
                },
            };
            if let Some(part) = intercepted {
                if !chunk.is_empty() {
                    let (m, p) = self.cost_pieces(&chunk.join(" "))?;
                    mana.push_str(&m);
                    parts.extend(p);
                    chunk.clear();
                }
                parts.push(part);
            } else {
                chunk.push(token);
            }
        }
        if !chunk.is_empty() {
            let (m, p) = self.cost_pieces(&chunk.join(" "))?;
            mana.push_str(&m);
            parts.extend(p);
        }
        Some((mana, parts))
    }

    fn activation_cost_reduction(
        &mut self,
        raw: &str,
        activation_zone: Option<&str>,
    ) -> Option<String> {
        let raw = raw.trim();
        let def = if raw.starts_with("Count$")
            || raw.starts_with("PlayerCount")
            || raw.starts_with("DungeonsCompleted$")
        {
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
        let Some(amt) = self
            .static_count_expr(&def)
            .or_else(|| super::atoms::static_wave14_amount(&def, self.svars))
        else {
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
            "Morbid" => "Condition::CreaturesDiedThisTurnAtLeast(1)",
            "Raid" => "Condition::AttackedWithAtLeast(1)",
            "Monarch" | "IsMonarch" => "Condition::IsMonarch(PlayerRel::You)",
            "Initiative" | "HasInitiative" => "Condition::HasInitiative(PlayerRel::You)",
            "CompletedDungeon" | "CompletedADungeon" => {
                "Condition::CompletedADungeon(PlayerRel::You)"
            }
            _ => return None,
        })
    }

    pub(super) fn defined_player_condition(raw: &str) -> Option<&'static str> {
        Some(match raw.trim() {
            "You.isMonarch" => "Condition::IsMonarch(PlayerRel::You)",
            "You.!isMonarch" => "Condition::Not(&Condition::IsMonarch(PlayerRel::You))",
            "Opponent.isMonarch" => "Condition::IsMonarch(PlayerRel::EachOpponent)",
            "Player.isMonarch" => "Condition::IsMonarch(PlayerRel::EachPlayer)",
            "You.hasInitiative" => "Condition::HasInitiative(PlayerRel::You)",
            "You.!hasInitiative" => "Condition::Not(&Condition::HasInitiative(PlayerRel::You))",
            "Opponent.hasInitiative" => "Condition::HasInitiative(PlayerRel::EachOpponent)",
            "Player.hasInitiative" => "Condition::HasInitiative(PlayerRel::EachPlayer)",
            "You.committedCrimeThisTurn" => "Condition::CommittedCrimeThisTurn(PlayerRel::You)",
            "You.completedDungeon" => "Condition::CompletedADungeon(PlayerRel::You)",
            _ => return None,
        })
    }

    pub(super) fn crime_or_opp_damaged_condition(def: &str, cmp: &str) -> Option<String> {
        let dungeon_count_cond = match def {
            "DungeonsCompleted$Amount" => Some("Condition::CompletedADungeon(PlayerRel::You)"),
            "PlayerCountPlayers$HasPropertycompletedDungeon" => {
                Some("Condition::CompletedADungeon(PlayerRel::EachPlayer)")
            }
            "PlayerCountOpponents$HasPropertycompletedDungeon"
            | "PlayerCountRegisteredOpponents$HasPropertycompletedDungeon" => {
                Some("Condition::CompletedADungeon(PlayerRel::EachOpponent)")
            }
            _ => None,
        };
        if let Some(cond) = dungeon_count_cond {
            return match cmp {
                "GE1" | "GT0" | "NE0" => Some(cond.to_string()),
                "EQ0" | "LE0" | "LT1" => Some(format!("Condition::Not(&{cond})")),
                _ => None,
            };
        }
        let cond = match def {
            "Count$CommittedCrimeThisTurn.1.0" | "Count$CommittedCrimeThisTurn" => {
                "Condition::CommittedCrimeThisTurn(PlayerRel::You)"
            }
            "PlayerCountOpponents$HasPropertywasDealtDamageThisTurn"
            | "PlayerCountRegisteredOpponents$HasPropertywasDealtDamageThisTurn" => {
                "Condition::OpponentDealtDamageThisTurn"
            }
            "Count$Monarch.1.0"
            | "Count$Monarch"
            | "PlayerCountPropertyYou$HasPropertyisMonarch" => {
                "Condition::IsMonarch(PlayerRel::You)"
            }
            "PlayerCountPlayers$HasPropertyisMonarch" => {
                "Condition::IsMonarch(PlayerRel::EachPlayer)"
            }
            "PlayerCountOpponents$HasPropertyisMonarch"
            | "PlayerCountRegisteredOpponents$HasPropertyisMonarch" => {
                "Condition::IsMonarch(PlayerRel::EachOpponent)"
            }
            "Count$Initiative.1.0"
            | "Count$Initiative"
            | "Count$HasInitiative.1.0"
            | "Count$HasInitiative"
            | "PlayerCountPropertyYou$HasPropertyhasInitiative" => {
                "Condition::HasInitiative(PlayerRel::You)"
            }
            "PlayerCountPlayers$HasPropertyhasInitiative" => {
                "Condition::HasInitiative(PlayerRel::EachPlayer)"
            }
            "PlayerCountOpponents$HasPropertyhasInitiative"
            | "PlayerCountRegisteredOpponents$HasPropertyhasInitiative" => {
                "Condition::HasInitiative(PlayerRel::EachOpponent)"
            }
            "Count$CompletedDungeon.1.0"
            | "Count$CompletedDungeon"
            | "Count$CompletedADungeon"
            | "PlayerCountPropertyYou$HasPropertycompletedDungeon" => {
                "Condition::CompletedADungeon(PlayerRel::You)"
            }
            _ => return None,
        };
        match cmp {
            "GE1" | "EQ1" | "GT0" => Some(cond.to_string()),
            "EQ0" | "LE0" | "LT1" | "NE1" => Some(format!("Condition::Not(&{cond})")),
            _ => None,
        }
    }

    /// Unified `CheckSVar$` + `SVarCompare$` entry point across activated,
    /// static, and cost-reduction readers.
    pub(super) fn svar_condition_clause(&mut self, var: &str, cmp: &str) -> Option<String> {
        self.svar_condition(var, cmp)
            .or_else(|| self.inline_valid_count_condition(var.trim(), cmp))
            .or_else(|| self.extra_svar_condition_clause(var, cmp))
            .or_else(|| self.inline_svar_condition(var, cmp))
    }

    fn optional_cost_svar_condition(&self, def: &str, cmp: &str) -> Option<String> {
        if self.in_delayed {
            return None;
        }
        if let Some(cond) = self.wave16b_presence_condition(def, cmp) {
            return Some(cond);
        }
        if self.body.optional_cost.is_some()
            && matches!(
                def,
                "Count$OptionalGenericCostPaid.1.0" | "CastSA>Count$OptionalGenericCostPaid.1.0"
            )
        {
            return match cmp {
                "GE1" | "EQ1" | "GT0" | "NE0" => Some("Condition::OptionalCostPaid".to_string()),
                "EQ0" | "LE0" | "LT1" | "NE1" => {
                    Some("Condition::Not(&Condition::OptionalCostPaid)".to_string())
                }
                _ => None,
            };
        }
        if self
            .body
            .optional_cost
            .as_deref()
            .is_some_and(|c| c.contains("CollectEvidence("))
            && matches!(def, "Collected$Amount" | "CastSA>Collected$Amount")
        {
            return match cmp {
                "GE1" | "GT0" | "NE0" => Some("Condition::OptionalCostPaid".to_string()),
                "EQ0" | "LE0" | "LT1" => {
                    Some("Condition::Not(&Condition::OptionalCostPaid)".to_string())
                }
                _ => None,
            };
        }
        None
    }

    fn inline_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let def = var.trim();
        let cmp = cmp.trim();
        if let Some(cond) = self.optional_cost_svar_condition(def, cmp) {
            return Some(cond);
        }
        if def == "PlayerCountPropertyYou$HasPropertyMaxSpeed" {
            return match cmp {
                "GE1" | "EQ1" => Some("Condition::MaxSpeed".to_string()),
                "EQ0" | "NE1" | "LT1" => Some("Condition::Not(&Condition::MaxSpeed)".to_string()),
                _ => None,
            };
        }
        if let Some(cond) = Self::crime_or_opp_damaged_condition(def, cmp) {
            return Some(cond);
        }
        if def.starts_with("Count$")
            && let Some(amt) = self
                .static_count_expr(def)
                .or_else(|| self.static_wave14_condition_amount(def))
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

        if let Some(valid) = raw.strip_prefix("Targeted$Valid ")
            && matches!(cmp, "GE1" | "EQ1" | "GT0")
        {
            let expr = self.filter_expr(valid.trim())?;
            return Some(format!(
                "Condition::ObjectMatches {{ what: &TargetSpec::Object(&Filter::Any), filter: &{expr} }}"
            ));
        }

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
        if raw == "Count$CastTotalManaSpent Snow" {
            return match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::SnowManaWasSpent".to_string()),
                "EQ0" | "LE0" | "LT1" => {
                    Some("Condition::Not(&Condition::SnowManaWasSpent)".to_string())
                }
                _ => self.amount_bound_condition("Amount::SnowManaSpentToCast", bound, eq_num),
            };
        }
        if raw == "Count$Bargained.1.0" && self.body.bargain {
            return match cmp {
                "GE1" | "EQ1" | "GT0" => Some("Condition::Bargained".to_string()),
                "EQ0" | "NE1" | "LT1" => Some("Condition::Not(&Condition::Bargained)".to_string()),
                _ => None,
            };
        }
        if let Some(cond) = self.optional_cost_svar_condition(&raw, cmp) {
            return Some(cond);
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
        if let Some(cond) = Self::crime_or_opp_damaged_condition(&raw, cmp) {
            return Some(cond);
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
                Some(Bound::AtMost(n)) => u8::try_from(n)
                    .ok()
                    .map(|n| format!("Condition::HandSizeAtMost({n})")),
                Some(Bound::AtLeast(n)) => Some(format!(
                    "Condition::AmountAtLeast(Amount::HandSize(PlayerRel::You), {n})"
                )),
                None => None,
            };
        }
        if matches!(
            raw.as_str(),
            "Count$ValidLibrary Card.YouOwn" | "Count$CardsInYourLibrary"
        ) {
            return self.amount_bound_condition(
                "Amount::LibrarySize(PlayerRel::You)",
                bound,
                eq_num,
            );
        }
        if raw == "Count$ValidGraveyard Card.YouOwn" {
            return match bound {
                Some(Bound::AtLeast(n)) if n >= 1 => u8::try_from(n)
                    .ok()
                    .map(|n| format!("Condition::GraveyardCountAtLeast({n})")),
                Some(Bound::AtMost(n)) => {
                    u8::try_from(n)
                        .ok()
                        .and_then(|v| v.checked_add(1))
                        .map(|above| {
                            format!("Condition::Not(&Condition::GraveyardCountAtLeast({above}))")
                        })
                }
                _ => None,
            };
        }
        if raw == "Count$ValidGraveyard Card.YouOwn$CardTypes" {
            return match bound {
                Some(Bound::AtLeast(n)) if n >= 1 => u8::try_from(n)
                    .ok()
                    .map(|n| format!("Condition::GraveyardTypesAtLeast({n})")),
                Some(Bound::AtMost(n)) => {
                    u8::try_from(n)
                        .ok()
                        .and_then(|v| v.checked_add(1))
                        .map(|above| {
                            format!("Condition::Not(&Condition::GraveyardTypesAtLeast({above}))")
                        })
                }
                _ => None,
            };
        }
        if raw == "Count$ValidGraveyard Card.OppOwn" {
            return match bound {
                Some(Bound::AtLeast(n)) if n >= 1 => u8::try_from(n)
                    .ok()
                    .map(|n| format!("Condition::OpponentGraveyardCountAtLeast({n})")),
                Some(Bound::AtMost(n)) => {
                    u8::try_from(n)
                        .ok()
                        .and_then(|v| v.checked_add(1))
                        .map(|above| {
                            format!(
                                "Condition::Not(&Condition::OpponentGraveyardCountAtLeast({above}))"
                            )
                        })
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
            if let Some(Bound::AtMost(_)) = bound {
                return self.amount_bound_condition(
                    &format!("Amount::CountersOnSource({kind})"),
                    bound,
                    None,
                );
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
        let amt = self
            .static_count_expr(&raw)
            .or_else(|| self.static_wave14_condition_amount(&raw))?;
        self.amount_bound_condition(&amt, bound, eq_num)
    }

    fn static_wave14_condition_amount(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        // Preserve pre-existing canonical condition spellings tested outside this module:
        // `Count$TotalPower Creature.YouCtrl` (`saga_ward_convoke` -> `ControlledCreaturePowerAtLeast`)
        // and `Count$ThisTurnCast_Card.YouCtrl` (`costs_and_refusals` -> `YouCastNoSpellThisTurn`).
        if trimmed == "Count$TotalPower Creature.YouCtrl"
            || trimmed == "Count$ThisTurnCast_Card.YouCtrl"
        {
            return None;
        }
        super::atoms::static_wave14_amount(trimmed, self.svars)
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
        self.condition_with_prose(p, "")
    }

    fn condition_with_prose(&mut self, p: &mut Params, line_prose: &str) -> Option<String> {
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
        if let Some(player_raw) = p.peek("CheckDefinedPlayer")
            && !p.has("IsPresent")
            && let Some(cond) = Self::defined_player_condition(player_raw)
        {
            p.take("CheckDefinedPlayer");
            return Some(format!(", condition = Some({cond})"));
        }
        let Some(valid) = p.take("IsPresent") else {
            return Some(String::new());
        };
        let clause = self.present_clause_with_prose(p, &valid, line_prose)?;
        Some(format!(", condition = Some({clause})"))
    }

    /// The rest of the `IsPresent$` family, `valid` being that key's value,
    /// as a `Condition` expression, or `None` when it was refused. What
    /// [`Self::condition`] splices in as an intervening "if", and what a
    /// state trigger (`Mode$ Always`, CR 603.8) triggers on.
    pub(super) fn present_clause(&mut self, p: &mut Params, valid: &str) -> Option<String> {
        self.present_clause_with_prose(p, valid, "")
    }

    fn present_clause_with_prose(
        &mut self,
        p: &mut Params,
        valid: &str,
        line_prose: &str,
    ) -> Option<String> {
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
            return self.graveyard_present_clause(p, valid, line_prose);
        }
        if p.peek("PresentZone") == Some("Hand") {
            return self.hand_present_clause(p, valid);
        }
        if p.peek("PresentZone") == Some("Library") {
            return self.library_present_clause(p, valid);
        }
        if matches!(
            p.peek("PresentZone"),
            Some("Battlefield,Graveyard" | "Graveyard,Battlefield")
        ) {
            return self.battlefield_or_graveyard_present_clause(p, valid);
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
        if let Some(clause) =
            Self::ringbearer_present_clause(valid, &compare, p.peek("PresentDefined").is_some())
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
            let Ok(n) = n.parse::<u8>() else {
                return self.deny(format!("a source counter clause `{valid}`"));
            };
            let Some(kind) = counter_kind(code) else {
                return self.deny(format!("counter `{code}`"));
            };
            p.take("PresentDefined");
            return Some(format!("Condition::CountersOnSelf({kind}, {n})"));
        }
        let pins_self = |alt: &str| alt.split(['.', '+']).any(|atom| atom.trim() == "Self");
        let defined = p.take("PresentDefined");
        let about_source = match defined.as_deref() {
            Some("Self") => true,
            Some(other) => return self.deny(format!("`PresentDefined$ {other}`")),
            None => valid.split(',').all(pins_self),
        };
        // A disjunctive `GE1` clause mixing source predicates with a
        // battlefield count (e.g. "if this land entered this turn or you
        // control a basic land") lowers to `Condition::Any`.
        if !about_source && defined.is_none() && compare == "GE1" && valid.split(',').any(pins_self)
        {
            let (self_alts, other_alts): (Vec<&str>, Vec<&str>) =
                valid.split(',').partition(|alt| pins_self(alt));
            let self_expr = self.filter_expr(&self_alts.join(","))?;
            let zoned = on_the_battlefield(&self_expr);
            let self_name = self.body.filter_static("CHECK", &zoned);
            let other_cond = self.battlefield_count_present_clause(&other_alts.join(","), "GE1")?;
            return Some(format!(
                "Condition::Any(&[Condition::SourceMatches(&{self_name}), {other_cond}])"
            ));
        }
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

    pub(super) fn commander_present_clause(
        valid: &str,
        compare: &str,
        has_defined: bool,
    ) -> Option<String> {
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

    pub(super) fn ringbearer_present_clause(
        valid: &str,
        compare: &str,
        has_defined: bool,
    ) -> Option<String> {
        if has_defined
            || !matches!(
                valid,
                "Card.IsRingbearer+YouCtrl"
                    | "Card.YouCtrl+IsRingbearer"
                    | "Creature.IsRingbearer+YouCtrl"
                    | "Creature.YouCtrl+IsRingbearer"
                    | "Permanent.IsRingbearer+YouCtrl"
                    | "Permanent.YouCtrl+IsRingbearer"
            )
        {
            return None;
        }
        match compare {
            "GE1" | "EQ1" => Some("Condition::HasRingBearer".to_string()),
            "EQ0" => Some("Condition::Not(&Condition::HasRingBearer)".to_string()),
            _ => None,
        }
    }

    fn graveyard_present_clause(
        &mut self,
        p: &mut Params,
        valid: &str,
        line_prose: &str,
    ) -> Option<String> {
        p.take("PresentZone");
        let has_you = p.take("PresentPlayer").as_deref() == Some("You");
        let compare = match p.take("PresentCompare") {
            Some(cmp) => cmp,
            None => self.infer_graveyard_present_compare(p, valid, line_prose)?,
        };
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

    fn present_amount_condition(&self, amt: &str, compare: &str) -> Option<String> {
        if let Some(eq_raw) = compare.strip_prefix("EQ")
            && let Ok(n) = eq_raw.parse::<u32>()
            && n >= 1
            && let Some(above) = n.checked_add(1)
        {
            return Some(format!(
                "Condition::All(&[Condition::AmountAtLeast({amt}, {n}), Condition::Not(&Condition::AmountAtLeast({amt}, {above}))])"
            ));
        }
        match count_bound(compare) {
            Some(Bound::AtLeast(n)) if n >= 1 => {
                Some(format!("Condition::AmountAtLeast({amt}, {n})"))
            }
            Some(Bound::AtMost(n)) => {
                let Some(above) = u32::from(n).checked_add(1) else {
                    return self.deny(format!("`PresentCompare$ {compare}`"));
                };
                Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast({amt}, {above}))"
                ))
            }
            _ => self.deny(format!("`PresentCompare$ {compare}`")),
        }
    }

    fn hand_present_clause(&mut self, p: &mut Params, valid: &str) -> Option<String> {
        p.take("PresentZone");
        if let Some(other) = p.take("PresentDefined") {
            return self.deny(format!("`PresentDefined$ {other}`"));
        }
        let has_you = match p.take("PresentPlayer").as_deref() {
            None => false,
            Some("You") => true,
            Some(who) => return self.deny(format!("`PresentPlayer$ {who}`")),
        };
        let compare = p
            .take("PresentCompare")
            .unwrap_or_else(|| "GE1".to_string());
        if matches!(valid.trim(), "Card.YouOwn" | "Card.YouCtrl")
            || (has_you && valid.trim() == "Card")
        {
            if let Some(eq_raw) = compare.strip_prefix("EQ") {
                let Ok(n) = eq_raw.parse::<u8>() else {
                    return self.deny(format!("`PresentCompare$ {compare}`"));
                };
                return Some(if n == 0 {
                    "Condition::HandSizeAtMost(0)".to_string()
                } else {
                    format!("Condition::HandSizeExactly({n})")
                });
            }
            return match count_bound(&compare) {
                Some(Bound::AtMost(n)) => {
                    let Ok(n) = u8::try_from(n) else {
                        return self.deny(format!("`PresentCompare$ {compare}`"));
                    };
                    Some(format!("Condition::HandSizeAtMost({n})"))
                }
                Some(Bound::AtLeast(n)) if n >= 1 => Some(format!(
                    "Condition::AmountAtLeast(Amount::HandSize(PlayerRel::You), {n})"
                )),
                _ => self.deny(format!("`PresentCompare$ {compare}`")),
            };
        }
        let Some(stripped) = strip_your_zone_filter(valid, !has_you) else {
            return self.deny(format!("`IsPresent$ {valid}` in hand"));
        };
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let name = self.body.filter_static("CHECK", &expr);
        let amt = format!("Amount::CountOf {{ filter: &{name}, zone: ZoneSel::HandYou }}");
        self.present_amount_condition(&amt, &compare)
    }

    fn library_present_clause(&mut self, p: &mut Params, valid: &str) -> Option<String> {
        p.take("PresentZone");
        if let Some(other) = p.take("PresentDefined") {
            return self.deny(format!("`PresentDefined$ {other}`"));
        }
        let has_you = match p.take("PresentPlayer").as_deref() {
            None => false,
            Some("You") => true,
            Some(who) => return self.deny(format!("`PresentPlayer$ {who}`")),
        };
        let compare = p
            .take("PresentCompare")
            .unwrap_or_else(|| "GE1".to_string());
        let amt = if matches!(valid.trim(), "Card.YouOwn" | "Card.YouCtrl")
            || (has_you && valid.trim() == "Card")
        {
            "Amount::LibrarySize(PlayerRel::You)".to_string()
        } else {
            let Some(stripped) = strip_your_zone_filter(valid, !has_you) else {
                return self.deny(format!("`IsPresent$ {valid}` in library"));
            };
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            let name = self.body.filter_static("CHECK", &expr);
            format!("Amount::CountOf {{ filter: &{name}, zone: ZoneSel::LibraryYou }}")
        };
        self.present_amount_condition(&amt, &compare)
    }

    fn battlefield_or_graveyard_present_clause(
        &mut self,
        p: &mut Params,
        valid: &str,
    ) -> Option<String> {
        p.take("PresentZone");
        if let Some(other) = p.take("PresentDefined") {
            return self.deny(format!("`PresentDefined$ {other}`"));
        }
        let has_you = match p.take("PresentPlayer").as_deref() {
            None => false,
            Some("You") => true,
            Some(who) => return self.deny(format!("`PresentPlayer$ {who}`")),
        };
        let compare = p
            .take("PresentCompare")
            .unwrap_or_else(|| "GE1".to_string());
        let Some(stripped) = strip_your_zone_filter(valid, !has_you) else {
            return self.deny(format!("`IsPresent$ {valid}` in battlefield or graveyard"));
        };
        let bf_expr = self.filter_expr(&stripped)?;
        let gy_expr = self.filter_expr_in_zone(&stripped, true)?;
        if compare == "GE1" {
            let bf_name = self.body.filter_static("CHECK", &bf_expr);
            let gy_name = self.body.filter_static("CHECK", &gy_expr);
            return Some(format!(
                "Condition::Any(&[Condition::ControlCount(&{bf_name}, 1), Condition::AmountAtLeast(Amount::CountOf {{ filter: &{gy_name}, zone: ZoneSel::GraveyardOf(PlayerRel::You) }}, 1)])"
            ));
        }
        let bf_you_expr = if bf_expr == "Filter::Any" {
            "Filter::ControlledByYou".to_string()
        } else {
            format!("Filter::And(&[{bf_expr}, Filter::ControlledByYou])")
        };
        let bf_you_name = self.body.filter_static("CHECK", &bf_you_expr);
        let gy_name = self.body.filter_static("CHECK", &gy_expr);
        let sum = format!(
            "Amount::Sum {{ left: &Amount::CountOf {{ filter: &{bf_you_name}, zone: ZoneSel::Battlefield }}, right: &Amount::CountOf {{ filter: &{gy_name}, zone: ZoneSel::GraveyardOf(PlayerRel::You) }} }}"
        );
        self.present_amount_condition(&sum, &compare)
    }

    fn params_prose(p: &Params) -> String {
        [
            "PrecostDesc",
            "SpellDescription",
            "Description",
            "TriggerDescription",
        ]
        .into_iter()
        .filter_map(|k| p.peek(k))
        .collect::<Vec<_>>()
        .join(" ")
    }

    fn infer_graveyard_present_compare(
        &self,
        p: &Params,
        valid: &str,
        saved_prose: &str,
    ) -> Option<String> {
        if matches!(valid, "Card.StrictlySelf" | "Card.Self") {
            return Some("GE1".to_string());
        }
        let fresh = Self::params_prose(p);
        let prose = if !fresh.is_empty() {
            fresh.to_ascii_lowercase()
        } else if !saved_prose.trim().is_empty() {
            saved_prose.to_ascii_lowercase()
        } else {
            self.oracle.to_ascii_lowercase()
        };
        let has_d4 = prose.contains("descend 4")
            || prose.contains("four or more permanent cards in your graveyard");
        let has_d8 = prose.contains("descend 8")
            || prose.contains("eight or more permanent cards in your graveyard");
        let has_other_threshold = prose.contains("descend ")
            || prose.contains("threshold")
            || prose.contains("seven or more cards in your graveyard")
            || prose.contains("or more permanent cards in your graveyard");
        match (has_d4, has_d8) {
            (true, false) => Some("GE4".to_string()),
            (false, true) => Some("GE8".to_string()),
            (true, true) => self.deny(
                "ambiguous graveyard `Descend` threshold without `PresentCompare$`".to_string(),
            ),
            (false, false) if has_other_threshold => {
                self.deny("multi-card graveyard threshold without `PresentCompare$`".to_string())
            }
            (false, false) => Some("GE1".to_string()),
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
        let parsed = if cost == "Forage" {
            Some((String::new(), vec!["Forage".to_string()]))
        } else if cost.starts_with("Behold<") {
            self.parse_behold_cost_token(cost)
                .map(|part| (String::new(), vec![part]))
        } else if let Some(dmg) = Self::damage_you_cost_token(cost) {
            dmg.ok().map(|part| (String::new(), vec![part]))
        } else {
            self.cost_pieces(cost)
        };
        let Some((mana, cost_parts)) = parsed else {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return self.wave28b_optional_trigger_price(body);
        };
        if !mana.is_empty() {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return self.wave28b_optional_trigger_price(body);
        }
        let result = match cost_parts.as_slice() {
            [one] => {
                if let Some(n) = one
                    .strip_prefix("PayLife(")
                    .and_then(|s| s.strip_suffix(')'))
                {
                    Some(OptionalTriggerPrice::Life(format!("Amount::Fixed({n})")))
                } else if asks_for_an_object(one)
                    || one == "Forage"
                    || one.starts_with("Behold(")
                    || one.starts_with("DealDamageToYou(")
                {
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
        if let Some(price) = result {
            return Some((rest, price));
        }
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        self.wave28b_optional_trigger_price(body)
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

fn strip_your_zone_filter(valid: &str, require_you: bool) -> Option<String> {
    let mut cleaned_alts = Vec::new();
    for alt in valid.split(',') {
        let mut parts = alt
            .split('.')
            .flat_map(|s| s.split('+'))
            .map(str::trim)
            .filter(|a| !a.is_empty());
        let base = parts.next()?;
        if matches!(
            base,
            "CARDNAME" | "NICKNAME" | "Self" | "Other" | "StrictlyOther"
        ) || base.contains("Ctrl")
            || base.contains("Own")
        {
            return None;
        }
        let mut has_you = false;
        let mut kept = Vec::new();
        for atom in parts {
            match atom {
                "YouOwn" | "YouCtrl" => has_you = true,
                "Self" | "Other" | "StrictlyOther" | "Above" | "DirectlyAbove" | "EnchantedBy"
                | "EquippedBy" | "AttachedBy" | "Attached" => return None,
                other
                    if other.contains("Ctrl")
                        || other.contains("Own")
                        || other.contains("ControlledBy")
                        || other.contains("OwnedBy") =>
                {
                    return None;
                }
                other => kept.push(other),
            }
        }
        if require_you && !has_you {
            return None;
        }
        if kept.is_empty() {
            cleaned_alts.push(base.to_string());
        } else {
            cleaned_alts.push(format!("{base}.{}", kept.join("+")));
        }
    }
    (!cleaned_alts.is_empty()).then(|| cleaned_alts.join(","))
}
