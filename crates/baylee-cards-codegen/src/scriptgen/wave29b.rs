//! Wave 29b pure-`scriptgen` lowerings across:
//! - `DealDamage` & `DealDamage.TargetMin` (generalized single-dealer bite to
//!   any target / scaled power / kicked multiplier, conditional self-bite,
//!   post-damage target retaliation, kicked target count)
//! - `Amount` / `Count$` / `SVar` counts (`Sac<All/...>` `SacrificedTotalPower`,
//!   `Sac<X/...>` `Amount::X`, `SacrificedThisTurn`, `CardsDiscardedThisTurn`,
//!   `LifeLostThisTurn`, `LeftBattlefieldThisTurn`, `LeftGraveyardThisTurn`,
//!   `CastTotalManaSpent`, `TriggerObjectsAttackers$Amount`, `LimitMin` /
//!   `LimitMax`, `Adamant` / `Void` / `CommittedCrimeThisTurn` two-arm counts,
//!   `Kicker:X` self-ETB `Count$xPaid`, `Lock and Load` other instant/sorcery
//!   casts, `RepeatEach` remembered-player turn-history counts, `NMinus` on
//!   resolution-chosen numbers)
//! - Multi-target chains & `TargetOnly.TargetMin` (Slot-2 `DamageAll` /
//!   `AnimateAll`, Slot-2 non-targeting follow-ups via `WithSecondTarget`,
//!   2-slot `TargetOnly` battlefield+graveyard exile & `WithAllTargets` blink,
//!   multi-target `TargetOnly` -> `DelayedTrigger` `EachTarget`)
//! - `PutCounter` & `Pump` (`AttachedBy Returned` `ChangeAttachedCounters`,
//!   `AttackersDeclaredOneTarget` sole-attacker follow-up counters,
//!   opponent/chosen-player protection & instant/sorcery spell protection)
//!
//! Every rule lowers strictly onto existing, engine-verified `baylee-cards-dsl`
//! variants with zero silent parameter drops and zero Oracle approximations.

use super::chains::LineFlags;
use super::wave27c::MultiTargetAction;
use super::{Chain, Params, Tx, cost_parts, counter_kind, mana_color_const};

impl Tx<'_> {
    /// Extract `(sac_all, sac_x, returns_self)` flags from a spell/ability `Cost$`.
    pub(super) fn wave29b_cost_flags(cost: Option<&str>) -> (bool, bool, bool) {
        let Some(raw) = cost else {
            return (false, false, false);
        };
        let parts = cost_parts(raw);
        let sac_all = parts.iter().any(|t| t.starts_with("Sac<All/"));
        let sac_x = parts.iter().any(|t| t.starts_with("Sac<X/"));
        let returns_self = parts.iter().any(|t| {
            t == "Return<1/CARDNAME>"
                || t.starts_with("Return<1/CARDNAME/")
                || t == "Return<1/NICKNAME>"
                || t.starts_with("Return<1/NICKNAME/")
        });
        (sac_all, sac_x, returns_self)
    }

    /// True when the script declares a single variable-`X` kicker (`K:Kicker:X ...`)
    /// without a mana-cost `X` (e.g. *Kangee, Aerie Keeper*: `K:Kicker:X 2`).
    pub(super) fn wave29b_script_has_x_kicker(script: &super::CardScript) -> bool {
        !script.has_x_mana_cost
            && script.keywords.iter().any(|kw| {
                kw.strip_prefix("Kicker:").is_some_and(|rest| {
                    !rest.contains(':')
                        && cost_parts(rest).iter().any(|tok| tok == "X")
                        && !cost_parts(rest).iter().any(|tok| tok.starts_with("XMin"))
                })
            })
    }

    fn wave29b_filter_ref(&mut self, expr: &str) -> String {
        match expr {
            "Filter::Any" | "Filter::PERMANENT" => "&Filter::Any".to_string(),
            "Filter::CREATURE" => "&Filter::CREATURE".to_string(),
            "Filter::ARTIFACT" => "&Filter::ARTIFACT".to_string(),
            "Filter::ENCHANTMENT" => "&Filter::ENCHANTMENT".to_string(),
            "Filter::LAND" => "&Filter::LAND".to_string(),
            "Filter::NONLAND" => "&Filter::NONLAND".to_string(),
            "Filter::YOUR_CREATURE" => "&Filter::YOUR_CREATURE".to_string(),
            "Filter::OPPONENT_CREATURE" => "&Filter::OPPONENT_CREATURE".to_string(),
            "Filter::ControlledByYou" => "&Filter::ControlledByYou".to_string(),
            "Filter::ControlledByOpponent" => "&Filter::ControlledByOpponent".to_string(),
            _ => format!("&{}", self.body.filter_static("W29B", expr)),
        }
    }

    // =========================================================================
    // Part 1: Bite / `DealDamage` extensions (`ParentTarget` & `Targeted` sources)
    // =========================================================================

    /// Allows Slot-2 `DealDamage | DamageSource$ ParentTarget` across all target
    /// specs (including `AnyTarget`, `AnyPlayer`, `AnyOpponent`) when Slot 1 is
    /// a single object (`single_object_first`).
    pub(super) fn wave29b_second_target_bite_allowed(
        p: &Params,
        single_object_first: bool,
    ) -> bool {
        single_object_first
            && p.peek("DamageSource") == Some("ParentTarget")
            && p.peek("Defined").is_none()
            && !p.has("DividedAsYouChoose")
            && !p.has("ExcessSVar")
            && ["UnlessCost", "ReplaceDyingDefined", "AtEOT"]
                .iter()
                .all(|k| p.peek(k).is_none())
    }

    /// Lowers Slot-2 `DealDamage | DamageSource$ ParentTarget` shapes beyond
    /// plain object-to-object `DamageEqualToPower`:
    /// - Any-target / player-capable bite (*Soul's Fire*, *Syrix*, *Thing Swing*)
    /// - Multi-recipient Slot-2 bite
    /// - Scaled power bite (*Animist's Might*, *Polliwallop*: `Targeted$CardPower/Twice`)
    /// - Kicked-multiplier bite (*Chocobo Kick*: `Targeted$CardPower/Times.Y`, `Y = Count$Kicked.2.1`)
    pub(super) fn wave29b_bite_effect(
        &mut self,
        p: &mut Params,
        second_target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.peek("Defined").is_some() || p.peek("DamageSource") != Some("ParentTarget") {
            return None;
        }
        let mut probe = p.clone();
        probe.take("DamageSource");
        let num_dmg = probe.take("NumDmg")?;
        if probe.take("DamageMap").is_some_and(|v| v != "True") {
            return None;
        }
        let may = match probe.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(_) => return None,
        };
        if !probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let eff_tgt = Self::effect_target_of(second_target?);
        let inner_dmg = self.wave29b_dealer_damage_op(&num_dmg, &eff_tgt)?;
        let wrapped = format!(
            "Effect::ForEachTarget {{ effects: &[Effect::WithSecondTarget(&[{inner_dmg}])] }}"
        );
        *p = probe;
        Some(if may {
            vec![format!("Effect::MayDo {{ effects: &[{wrapped}] }}")]
        } else {
            vec![wrapped]
        })
    }

    fn wave29b_dealer_damage_op(&mut self, num_dmg: &str, eff_tgt: &str) -> Option<String> {
        let trimmed = num_dmg.trim();
        if self.is_power_of(trimmed, &["ParentTargeted$CardPower", "Targeted$CardPower"]) {
            return Some(format!(
                "Effect::EventObjectDealsDamageEqualToPower {{ target: {eff_tgt} }}"
            ));
        }
        let resolved = self
            .svars
            .get(trimmed)
            .map_or(trimmed, String::as_str)
            .trim()
            .to_string();
        // Kicked multiplier (`Targeted$CardPower/Times.Y` where `Y = Count$Kicked.<a>.<b>`).
        if let Some((lhs, rhs)) = resolved.split_once("/Times.")
            && self.can_check_kicked()
            && self.is_power_of(
                lhs.trim().trim_start_matches("SVar$"),
                &["ParentTargeted$CardPower", "Targeted$CardPower"],
            )
            && let Some((when_kicked, when_unkicked)) = self.kicked_pair(rhs.trim())
            && when_kicked > 0
            && when_unkicked > 0
        {
            let arm = |factor: i64| -> String {
                if factor == 1 {
                    format!("Effect::EventObjectDealsDamageEqualToPower {{ target: {eff_tgt} }}")
                } else {
                    format!(
                        "Effect::EventObjectDealsDamage {{ amount: Amount::Times {{ \
                         base: &Amount::EventPower, factor: {factor} }}, target: {eff_tgt} }}"
                    )
                }
            };
            return Some(format!(
                "Effect::IfKicked {{ then: &[{}], otherwise: &[{}] }}",
                arm(i64::from(when_kicked)),
                arm(i64::from(when_unkicked))
            ));
        }
        // Kicked two-arm SVar (`Count$Kicked.<Y>.<Z>`, e.g. *Chocobo Kick*).
        if let Some(arms) = resolved.strip_prefix("Count$Kicked.")
            && self.can_check_kicked()
            && let Some((then_var, else_var)) = arms.split_once('.')
            && !else_var.contains('.')
        {
            let then_op = self.wave29b_dealer_damage_op(then_var.trim(), eff_tgt)?;
            let else_op = self.wave29b_dealer_damage_op(else_var.trim(), eff_tgt)?;
            return Some(format!(
                "Effect::IfKicked {{ then: &[{then_op}], otherwise: &[{else_op}] }}"
            ));
        }
        let amt = self.wave29b_dealer_power_amount(&resolved, 0)?;
        Some(format!(
            "Effect::EventObjectDealsDamage {{ amount: {amt}, target: {eff_tgt} }}"
        ))
    }

    fn wave29b_dealer_power_amount(&mut self, def: &str, depth: u8) -> Option<String> {
        if depth >= 4 {
            return None;
        }
        let def = def.trim();
        if matches!(def, "ParentTargeted$CardPower" | "Targeted$CardPower") {
            return Some("Amount::EventPower".to_string());
        }
        if let Some(var) = def.strip_prefix("SVar$") {
            let inner = self.svars.get(var.trim())?.trim().to_string();
            return self.wave29b_dealer_power_amount(&inner, depth + 1);
        }
        if let Some(inner) = self.svars.get(def).map(|s| s.trim().to_string()) {
            return self.wave29b_dealer_power_amount(&inner, depth + 1);
        }
        if let Some(rest) = def.strip_prefix("Count$Compare ") {
            let (lhs, else_s) = rest.rsplit_once('.')?;
            let (cond_part, then_s) = lhs.rsplit_once('.')?;
            let (z_var, cmp) = cond_part.trim().rsplit_once(' ')?;
            let z_def = self.svars.get(z_var.trim())?.trim().to_string();
            let valid = z_def
                .strip_prefix("Targeted$Valid ")
                .or_else(|| z_def.strip_prefix("ParentTargeted$Valid "))?;
            if valid.contains(['$', '/']) {
                return None;
            }
            let expr = self.filter_expr(valid.trim())?;
            let f_ref = self.wave29b_filter_ref(&expr);
            let cond = format!(
                "Condition::ObjectMatches {{ what: &TargetSpec::EventObject, filter: {f_ref} }}"
            );
            let t = self.wave29b_dealer_power_amount(then_s.trim(), depth + 1)?;
            let e = self.wave29b_dealer_power_amount(else_s.trim(), depth + 1)?;
            let (first_arm, second_arm) = match cmp.trim() {
                "GE1" | "GT0" | "EQ1" | "NE0" => (t, e),
                "EQ0" | "LE0" | "LT1" => (e, t),
                _ => return None,
            };
            return Some(format!(
                "Amount::IfCondition {{ condition: &{cond}, then: &{first_arm}, otherwise: &{second_arm} }}"
            ));
        }
        let (lhs, op) = def.rsplit_once('/')?;
        let base = if let Some(var) = lhs.trim().strip_prefix("SVar$") {
            let inner = self.svars.get(var.trim())?.trim().to_string();
            self.wave29b_dealer_power_amount(&inner, depth + 1)?
        } else {
            self.wave29b_dealer_power_amount(lhs.trim(), depth + 1)?
        };
        let op = op.trim();
        match op {
            "Twice" => Some(format!("Amount::Times {{ base: &{base}, factor: 2 }}")),
            "Thrice" => Some(format!("Amount::Times {{ base: &{base}, factor: 3 }}")),
            "HalfDown" => Some(format!("Amount::Half {{ base: &{base}, round_up: false }}")),
            "HalfUp" => Some(format!("Amount::Half {{ base: &{base}, round_up: true }}")),
            _ => {
                if let Some(n) = op
                    .strip_prefix("Times.")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .filter(|&n| n >= 1)
                {
                    return Some(if n == 1 {
                        base
                    } else {
                        format!("Amount::Times {{ base: &{base}, factor: {n} }}")
                    });
                }
                if let Some(n) = op
                    .strip_prefix("Plus.")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                {
                    return Some(if n == 0 {
                        base
                    } else {
                        format!("Amount::Plus {{ base: &{base}, offset: {n} }}")
                    });
                }
                if let Some(rhs_var) = op.strip_prefix("Times.") {
                    return self.wave29b_event_object_compare_multiplier(rhs_var.trim(), &base);
                }
                None
            }
        }
    }

    /// Matches `Count$Compare <Z> GE1.<A>.<B>` where `<Z>` is `Targeted$Valid <F>`
    /// (e.g. *Cut Propulsion*: `Z = Targeted$Valid Card.YouCtrl`).
    fn wave29b_event_object_compare_multiplier(&mut self, var: &str, base: &str) -> Option<String> {
        let def = self.svars.get(var)?.trim().to_string();
        let rest = def.strip_prefix("Count$Compare ")?;
        let (lhs, else_s) = rest.rsplit_once('.')?;
        let (cond_part, then_s) = lhs.rsplit_once('.')?;
        let (z_var, cmp) = cond_part.trim().rsplit_once(' ')?;
        if !matches!(cmp.trim(), "GE1" | "GT0" | "EQ1" | "NE0") {
            return None;
        }
        let then_n: u32 = then_s.trim().parse().ok().filter(|&n| n >= 1)?;
        let else_n: u32 = else_s.trim().parse().ok().filter(|&n| n >= 1)?;
        let z_def = self.svars.get(z_var.trim())?.trim().to_string();
        let valid = z_def
            .strip_prefix("Targeted$Valid ")
            .or_else(|| z_def.strip_prefix("ParentTargeted$Valid "))?;
        if valid.contains(['$', '/']) {
            return None;
        }
        let expr = self.filter_expr(valid.trim())?;
        let f_ref = self.wave29b_filter_ref(&expr);
        let cond = format!(
            "Condition::ObjectMatches {{ what: &TargetSpec::EventObject, filter: {f_ref} }}"
        );
        let scale = |b: &str, n: u32| -> String {
            if n == 1 {
                b.to_string()
            } else {
                format!("Amount::Times {{ base: &{b}, factor: {n} }}")
            }
        };
        let t = scale(base, then_n);
        let e = scale(base, else_n);
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond}, then: &{t}, otherwise: &{e} }}"
        ))
    }

    /// Slot-1 `DealDamage` with `DamageSource$ Targeted` / `ParentTarget` where:
    /// - Target deals scaled/conditional power damage to itself (*Cut Propulsion*), or
    /// - Target deals power damage to a player (*Form of the Dinosaur* `Defined$ You`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn wave29b_targeted_damage_source_fallback(
        &mut self,
        defined: Option<&str>,
        num_dmg: &str,
        damage_source: Option<&str>,
        may: bool,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        if !single_target || !target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) {
            return None;
        }
        match (damage_source, defined, targets_here) {
            // Self-bite with scaled/conditional power (e.g. *Cut Propulsion*).
            (Some("Targeted"), None, true) => {
                let resolved = self
                    .svars
                    .get(num_dmg.trim())
                    .map_or(num_dmg.trim(), String::as_str)
                    .trim()
                    .to_string();
                let amt = self.wave29b_dealer_power_amount(&resolved, 0)?;
                let wrapped = format!(
                    "Effect::ForEachTarget {{ effects: &[Effect::EventObjectDealsDamage {{ \
                     amount: {amt}, target: TargetSpec::EventObject }}] }}"
                );
                Some(if may {
                    vec![format!("Effect::MayDo {{ effects: &[{wrapped}] }}")]
                } else {
                    vec![wrapped]
                })
            }
            // Post-damage retaliation to a player (e.g. *Form of the Dinosaur*: `Defined$ You`).
            (Some("Targeted" | "ParentTarget"), Some(who), false) => {
                let rel = self.wave11_player_of_line(Some(who), target, false)?;
                let player_spec = format!("TargetSpec::Player({rel})");
                let inner = self.wave29b_dealer_damage_op(num_dmg, &player_spec)?;
                let wrapped = format!("Effect::ForEachTarget {{ effects: &[{inner}] }}");
                Some(if may {
                    vec![format!("Effect::MayDo {{ effects: &[{wrapped}] }}")]
                } else {
                    vec![wrapped]
                })
            }
            _ => None,
        }
    }

    // =========================================================================
    // Part 2: `Amount` / `Count$` / `SVar` & `Condition` extensions
    // =========================================================================

    /// Additional `Count$` / `SVar` expressions hooked into `mana::count_expr_depth`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29b_count_expr(
        &mut self,
        def: &str,
        depth: u8,
        static_only: bool,
    ) -> Option<String> {
        if static_only || !self.in_chain_resolution || depth >= 6 {
            return None;
        }
        let def = def.trim();

        if let Some(expr) = self.wave27c_remembered_amount(def) {
            return Some(expr);
        }
        if let Some(expr) = self.wave29b_two_arm_conditional_amount(def, depth) {
            return Some(expr);
        }

        if let Some(valid_raw) = def
            .strip_prefix("PlayerCountPropertyYou$SacrificedThisTurn ")
            .or_else(|| def.strip_prefix("Count$SacrificedThisTurn "))
            && !valid_raw.contains(['$', '/'])
            && !valid_raw.contains("Self")
        {
            return self.wave29b_sacrificed_this_turn_amount(valid_raw.trim());
        }
        match def {
            "PlayerCountPropertyYou$CardsDiscardedThisTurn" | "Count$YouDiscardedThisTurn" => {
                return Some("Amount::CardsDiscardedThisTurn(PlayerRel::You)".to_string());
            }
            "PlayerCountOpponents$CardsDiscardedThisTurn" => {
                return Some("Amount::CardsDiscardedThisTurn(PlayerRel::EachOpponent)".to_string());
            }
            "PlayerCountPlayers$CardsDiscardedThisTurn" => {
                return Some("Amount::CardsDiscardedThisTurn(PlayerRel::EachPlayer)".to_string());
            }
            "Count$LifeYouGainedThisTurn" | "PlayerCountPropertyYou$LifeGainedThisTurn" => {
                return Some("Amount::LifeGainedThisTurn(PlayerRel::You)".to_string());
            }
            "Count$LifeYouLostThisTurn" | "PlayerCountPropertyYou$LifeLostThisTurn" => {
                return Some("Amount::LifeLostThisTurn(PlayerRel::You)".to_string());
            }
            _ => {}
        }
        if let Some(valid_raw) = def.strip_prefix("Count$LeftBattlefieldThisTurn ")
            && !valid_raw.contains(['$', '/'])
            && !valid_raw.contains("Self")
        {
            return self.wave29b_left_zone_this_turn_amount(valid_raw.trim(), true);
        }
        if let Some(valid_raw) = def.strip_prefix("Count$LeftGraveyardThisTurn ")
            && !valid_raw.contains(['$', '/'])
            && !valid_raw.contains("Self")
        {
            return self.wave29b_left_zone_this_turn_amount(valid_raw.trim(), false);
        }

        // Resolution-scoped amounts:
        if let Some(player_rel) = self.current_single_player_rel {
            match def {
                "TargetedPlayer$CardsDiscardedThisTurn"
                | "ThisTargetedPlayer$CardsDiscardedThisTurn" => {
                    return Some(format!("Amount::CardsDiscardedThisTurn({player_rel})"));
                }
                "TargetedPlayer$LifeLostThisTurn" | "ThisTargetedPlayer$LifeLostThisTurn" => {
                    return Some(format!("Amount::LifeLostThisTurn({player_rel})"));
                }
                "TargetedPlayer$LifeGainedThisTurn" | "ThisTargetedPlayer$LifeGainedThisTurn" => {
                    return Some(format!("Amount::LifeGainedThisTurn({player_rel})"));
                }
                "TargetedPlayer$DomainPlayer" | "ThisTargetedPlayer$DomainPlayer" => {
                    let dom_expr =
                        format!("Filter::And(&[Filter::LAND, Filter::ControlledBy({player_rel})])");
                    let f_ref = self.wave29b_filter_ref(&dom_expr);
                    return Some(format!("Amount::BasicLandTypesAmong({f_ref})"));
                }
                _ => {}
            }
        }
        match def {
            "Sacrificed$CardPower" if self.cost_sacrifices_all => {
                return Some("Amount::SacrificedTotalPower".to_string());
            }
            "Sacrificed$Amount"
                if self.has_x
                    && self.cost_sacrifices_x
                    && self.svars.get("X").map(|s| s.trim()) == Some("Count$xPaid") =>
            {
                return Some("Amount::X".to_string());
            }
            "Count$xPaid"
                if self.has_x_kicker
                    && !self.in_delayed
                    && (self.is_etb_self_trigger || self.trigger_self_etb_kicked) =>
            {
                return Some("Amount::X".to_string());
            }
            "TriggeredCard$CastTotalManaSpent"
            | "TriggeredSpellAbility$CastTotalManaSpent"
            | "TriggeredCardLKI$CastTotalManaSpent"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("SpellCast" | "SpellCastOrCopy")
                ) =>
            {
                return Some("Amount::EventManaSpentToCast".to_string());
            }
            "TriggerObjectsAttackers$Amount"
                if self.trigger_mode.as_deref() == Some("AttackersDeclared") =>
            {
                return Some("Amount::EventAmount".to_string());
            }
            // *Lock and Load*: on a printed Instant/Sorcery spell (card types
            // Instant/Sorcery cannot become permanent types on the stack, CR 304.4),
            // other instant and sorcery spells you've cast this turn is exact.
            "Count$ThisTurnCast_Instant.YouCtrl,Sorcery.YouCtrl+!CastSaSource"
            | "Count$ThisTurnCast_Instant.YouCtrl+!CastSaSource,Sorcery.YouCtrl+!CastSaSource"
                if self.on_a_spell && self.wave29b_is_pure_instant_or_sorcery() =>
            {
                return Some(
                    "Amount::Difference { left: &Amount::SpellsCastMatchingThisTurn { \
                     filter: &Filter::INSTANT_OR_SORCERY, caster: PlayerRel::You }, \
                     right: &Amount::IfCondition { condition: &Condition::SourceWasCastByYou, \
                     then: &Amount::Fixed(1), otherwise: &Amount::Fixed(0) } }"
                        .to_string(),
                );
            }
            _ => {}
        }

        // `SVar$<A>/LimitMax.<B>` (= min(A, B)), where `<B>` is an SVar or positive literal
        // (`LimitMin` is already handled by `wave28c_count_expr`).
        if let Some((lhs_raw, rhs_raw)) = def.rsplit_once("/LimitMax.")
            && rhs_raw.trim().parse::<u32>() != Ok(0)
        {
            let lhs = self.wave29b_resolve_sub_amount(lhs_raw.trim(), depth + 1)?;
            if lhs.starts_with("Amount::Fixed(") {
                return None;
            }
            let rhs = self.wave29b_resolve_sub_amount(rhs_raw.trim(), depth + 1)?;
            return Some(format!(
                "Amount::IfCondition {{ condition: &Condition::AmountCompare {{ \
                 left: &{lhs}, op: AmountCmp::AtMost, right: &{rhs} }}, then: &{lhs}, otherwise: &{rhs} }}"
            ));
        }

        // `Count$Compare <Var> <CMP><N>.<Then>.<Else>` where `<Var>` resolves via
        // `wave29b_resolve_sub_amount` (e.g. *Tackle Artist*, *That's Rough Buddy*).
        if let Some(rest) = def.strip_prefix("Count$Compare ") {
            return self.wave29b_count_compare_amount(rest, depth + 1);
        }

        None
    }

    /// Fallback for `effects::amount_or_count_ext` and `pump_side_ext`.
    pub(super) fn wave29b_amount_ext(&mut self, _raw: &str, resolved: &str) -> Option<String> {
        let resolved = resolved.trim();
        if let Some(expr) = self.wave29b_count_expr(resolved, 0, false) {
            return Some(expr);
        }
        if let Some((lhs, op)) = resolved.rsplit_once('/')
            && !op.starts_with("LimitMin.")
            && !op.starts_with("LimitMax.")
        {
            let base = if let Some(var) = lhs.trim().strip_prefix("SVar$") {
                let inner = self.svars.get(var.trim())?.trim().to_string();
                self.wave29b_amount_ext(var.trim(), &inner)?
            } else {
                self.wave29b_count_expr(lhs.trim(), 1, false)?
            };
            return Self::wave29b_apply_arith_op(&base, op.trim());
        }
        None
    }

    fn wave29b_apply_arith_op(base: &str, op: &str) -> Option<String> {
        match op {
            "Twice" => Some(format!("Amount::Times {{ base: &{base}, factor: 2 }}")),
            "Thrice" => Some(format!("Amount::Times {{ base: &{base}, factor: 3 }}")),
            "HalfDown" => Some(format!("Amount::Half {{ base: &{base}, round_up: false }}")),
            "HalfUp" => Some(format!("Amount::Half {{ base: &{base}, round_up: true }}")),
            _ => {
                if let Some(n) = op
                    .strip_prefix("Plus.")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                {
                    return Some(if n == 0 {
                        base.to_string()
                    } else {
                        format!("Amount::Plus {{ base: &{base}, offset: {n} }}")
                    });
                }
                if let Some(n) = op
                    .strip_prefix("Minus.")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                {
                    return Some(if n == 0 {
                        base.to_string()
                    } else {
                        format!("Amount::SaturatingSub {{ base: &{base}, subtract: {n} }}")
                    });
                }
                if let Some(n) = op
                    .strip_prefix("NMinus.")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                {
                    return Some(format!(
                        "Amount::ConstantMinus {{ constant: {n}, subtract: &{base} }}"
                    ));
                }
                if let Some(n) = op
                    .strip_prefix("Times.")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .filter(|&v| v >= 1)
                {
                    return Some(if n == 1 {
                        base.to_string()
                    } else {
                        format!("Amount::Times {{ base: &{base}, factor: {n} }}")
                    });
                }
                None
            }
        }
    }

    fn wave29b_resolve_sub_amount(&mut self, token: &str, depth: u8) -> Option<String> {
        if depth >= 6 {
            return None;
        }
        let trimmed = token.trim();
        if let Ok(n) = trimmed.parse::<u32>() {
            return Some(format!("Amount::Fixed({n})"));
        }
        if let Some(var) = trimmed.strip_prefix("SVar$") {
            let inner = self.svars.get(var.trim())?.trim().to_string();
            return self.wave29b_resolve_sub_amount(&inner, depth + 1);
        }
        if let Some(def) = self.svars.get(trimmed).map(|s| s.trim().to_string()) {
            return self.wave29b_resolve_sub_amount(&def, depth + 1);
        }
        let amt = self
            .count_expr_depth(trimmed, depth, false)
            .or_else(|| self.wave29b_count_expr(trimmed, depth, false))?;
        (!amt.contains("Neg")).then_some(amt)
    }

    fn wave29b_count_compare_amount(&mut self, rest: &str, depth: u8) -> Option<String> {
        let (lhs, else_raw) = rest.rsplit_once('.')?;
        let (cond_part, then_raw) = lhs.rsplit_once('.')?;
        let (var, cmp) = cond_part.trim().rsplit_once(' ')?;
        if self.on_a_spell
            && !self.in_delayed
            && let Some((cond_expr, swap_arms)) =
                self.wave29b_cast_time_subtype_condition(var.trim(), cmp.trim())
        {
            let then_amt = self.wave29b_resolve_sub_amount(then_raw.trim(), depth + 1)?;
            let else_amt = self.wave29b_resolve_sub_amount(else_raw.trim(), depth + 1)?;
            let (first_arm, second_arm) = if swap_arms {
                (else_amt, then_amt)
            } else {
                (then_amt, else_amt)
            };
            return Some(format!(
                "Amount::IfCondition {{ condition: &{cond_expr}, \
                 then: &{first_arm}, otherwise: &{second_arm} }}"
            ));
        }
        let (op_code, rhs_str) = cmp.trim().split_at_checked(2)?;
        let op = match op_code {
            "GE" => "AmountCmp::AtLeast",
            "GT" => "AmountCmp::GreaterThan",
            "LE" => "AmountCmp::AtMost",
            "LT" => "AmountCmp::LessThan",
            "EQ" => "AmountCmp::Equal",
            "NE" => "AmountCmp::NotEqual",
            _ => return None,
        };
        let rhs_amt = self.wave29b_resolve_sub_amount(rhs_str.trim(), depth + 1)?;
        let lhs_amt = self.wave29b_resolve_sub_amount(var.trim(), depth + 1)?;
        let then_amt = self.wave29b_resolve_sub_amount(then_raw.trim(), depth + 1)?;
        let else_amt = self.wave29b_resolve_sub_amount(else_raw.trim(), depth + 1)?;
        Some(format!(
            "Amount::IfCondition {{ condition: &Condition::AmountCompare {{ \
             left: &{lhs_amt}, op: {op}, right: &{rhs_amt} }}, \
             then: &{then_amt}, otherwise: &{else_amt} }}"
        ))
    }

    fn wave29b_is_pure_instant_or_sorcery(&self) -> bool {
        let has_inst_or_sorc = self
            .type_line
            .split_whitespace()
            .any(|w| matches!(w, "Instant" | "Sorcery"));
        let has_perm_type = self.type_line.split_whitespace().any(|w| {
            matches!(
                w,
                "Artifact" | "Battle" | "Creature" | "Enchantment" | "Land" | "Planeswalker"
            )
        });
        has_inst_or_sorc && !has_perm_type
    }

    fn wave29b_sacrificed_this_turn_amount(&mut self, valid_raw: &str) -> Option<String> {
        if matches!(valid_raw, "Permanent" | "Card") {
            return Some(
                "Amount::PermanentsSacrificedThisTurn { who: PlayerRel::You, filter: &Filter::Any }"
                    .to_string(),
            );
        }
        let cleaned = Self::wave29b_strip_ctrl_atom(valid_raw, "YouCtrl")
            .unwrap_or_else(|| valid_raw.to_string());
        if cleaned.contains("Ctrl") || cleaned.contains("Own") {
            return None;
        }
        let expr = self.filter_expr(&cleaned)?;
        let f_ref = self.wave29b_filter_ref(&expr);
        Some(format!(
            "Amount::PermanentsSacrificedThisTurn {{ who: PlayerRel::You, filter: {f_ref} }}"
        ))
    }

    pub(super) fn wave29b_left_zone_this_turn_amount(
        &mut self,
        valid_raw: &str,
        battlefield: bool,
    ) -> Option<String> {
        let (who, cleaned) = if battlefield {
            if let Some(s) = Self::wave29b_strip_ctrl_atom(valid_raw, "YouCtrl") {
                ("PlayerRel::You", s)
            } else if let Some(s) = Self::wave29b_strip_ctrl_atom(valid_raw, "OppCtrl") {
                ("PlayerRel::EachOpponent", s)
            } else if !valid_raw.contains("Ctrl") && !valid_raw.contains("Own") {
                ("PlayerRel::EachPlayer", valid_raw.to_string())
            } else {
                return None;
            }
        } else if let Some(s) = Self::wave29b_strip_ctrl_atom(valid_raw, "YouOwn")
            .or_else(|| Self::wave29b_strip_ctrl_atom(valid_raw, "YouCtrl"))
        {
            ("PlayerRel::You", s)
        } else if let Some(s) = Self::wave29b_strip_ctrl_atom(valid_raw, "OppOwn")
            .or_else(|| Self::wave29b_strip_ctrl_atom(valid_raw, "OppCtrl"))
        {
            ("PlayerRel::EachOpponent", s)
        } else if !valid_raw.contains("Ctrl") && !valid_raw.contains("Own") {
            ("PlayerRel::EachPlayer", valid_raw.to_string())
        } else {
            return None;
        };
        let expr = if battlefield {
            self.filter_expr(&cleaned)?
        } else {
            self.filter_expr_in_zone(&cleaned, true)?
        };
        let f_ref = self.wave29b_filter_ref(&expr);
        let zone = if battlefield {
            "ZoneRef::Battlefield"
        } else {
            "ZoneRef::Graveyard"
        };
        Some(format!(
            "Amount::LeftZoneThisTurn {{ filter: {f_ref}, from: {zone}, owner_or_ctrl: {who} }}"
        ))
    }

    fn wave29b_strip_ctrl_atom(valid_raw: &str, needle: &str) -> Option<String> {
        let mut out_alts = Vec::new();
        for alt in valid_raw.split(',').map(str::trim) {
            let (base, atoms_raw) = alt.split_once('.').unwrap_or((alt, ""));
            let mut saw = false;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                if atom == needle {
                    saw = true;
                } else {
                    kept.push(atom);
                }
            }
            if !saw {
                return None;
            }
            if kept.is_empty() {
                out_alts.push(base.to_string());
            } else {
                out_alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        (!out_alts.is_empty()).then(|| out_alts.join(","))
    }

    /// Two-arm conditional `Count$` extensions (`Adamant.<Color>.<A>.<B>`,
    /// `Void.<A>.<B>`, `CommittedCrimeThisTurn.<A>.<B>`, `PromisedGift.<A>.<B>`).
    pub(super) fn wave29b_two_arm_conditional_amount(
        &mut self,
        def: &str,
        depth: u8,
    ) -> Option<String> {
        let rest = def.strip_prefix("Count$")?;
        let spell_or_etb = !self.in_delayed && (self.on_a_spell || self.is_etb_self_trigger);
        if let Some(adamant_rest) = rest.strip_prefix("Adamant.") {
            if !spell_or_etb {
                return None;
            }
            let (color_word_raw, arms) = adamant_rest.split_once('.')?;
            let mana_color = match color_word_raw.trim() {
                "White" | "W" => "ManaColor::White",
                "Blue" | "U" => "ManaColor::Blue",
                "Black" | "B" => "ManaColor::Black",
                "Red" | "R" => "ManaColor::Red",
                "Green" | "G" => "ManaColor::Green",
                "Colorless" | "C" => "ManaColor::Colorless",
                other => mana_color_const(other)?,
            };
            let (then_raw, else_raw) = arms.split_once('.')?;
            if else_raw.contains('.') {
                return None;
            }
            let then_amt = self.wave29b_resolve_sub_amount(then_raw.trim(), depth + 1)?;
            let else_amt = self.wave29b_resolve_sub_amount(else_raw.trim(), depth + 1)?;
            return Some(format!(
                "Amount::IfCondition {{ condition: &Condition::AmountAtLeast(\
                 Amount::ManaColorSpentToCast({mana_color}), 3), \
                 then: &{then_amt}, otherwise: &{else_amt} }}"
            ));
        }
        let (word, arms) = rest.split_once('.')?;
        let cond = match word.trim() {
            "Void" => {
                "Condition::Any(&[Condition::AmountAtLeast(Amount::LeftZoneThisTurn { \
                 filter: &Filter::NONLAND, from: ZoneRef::Battlefield, \
                 owner_or_ctrl: PlayerRel::EachPlayer }, 1), \
                 Condition::SpellWarpedThisTurn(None)])"
            }
            "CommittedCrimeThisTurn" => "Condition::CommittedCrimeThisTurn(PlayerRel::You)",
            "PromisedGift" if spell_or_etb => "Condition::GiftWasPromised",
            _ => return None,
        };
        let (then_raw, else_raw) = arms.split_once('.')?;
        if else_raw.contains('.') {
            return None;
        }
        let then_amt = self.wave29b_resolve_sub_amount(then_raw.trim(), depth + 1)?;
        let else_amt = self.wave29b_resolve_sub_amount(else_raw.trim(), depth + 1)?;
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond}, then: &{then_amt}, otherwise: &{else_amt} }}"
        ))
    }

    // =========================================================================
    // Part 3: Multi-target chains (`WithSecondTarget` / `WithAllTargets`)
    // =========================================================================

    /// Slot-2 / Slot-3 effect adapter for `DamageAll` (e.g. *Seismic Wave*) and
    /// `AnimateAll` (e.g. *Quick Draw*) targeting a player.
    pub(super) fn wave29b_slot2_or_slot3_effect(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
        single_target: bool,
        _is_second_slot: bool,
    ) -> Option<Vec<String>> {
        if api == "DamageAll" && targets_a_player {
            let mut probe = p.clone();
            let raw_dmg = probe.take("NumDmg")?;
            let valid = probe.take("ValidCards")?;
            let desc = probe.take("ValidDescription");
            if probe.has("ValidPlayers") {
                return None;
            }
            let has_tgt_ctrl = valid.contains("TargetedPlayerCtrl");
            let desc_names_tgt_ctrl = desc.as_deref().is_some_and(|d| {
                let l = d.to_ascii_lowercase();
                l.contains("target opponent controls") || l.contains("target player controls")
            });
            if !has_tgt_ctrl && !desc_names_tgt_ctrl {
                return None;
            }
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition"))
            {
                return None;
            }
            let n = self.amount_or_count_ext(&raw_dmg)?;
            let stripped =
                Self::wave29b_strip_ctrl_atom(&valid, "TargetedPlayerCtrl").unwrap_or(valid);
            let base_expr = self.filter_expr(&stripped)?;
            let full_expr =
                format!("Filter::And(&[{base_expr}, Filter::ControlledBy(PlayerRel::Chosen)])");
            let name = self.body.filter_static("EACH", &full_expr);
            *p = probe;
            return Some(vec![format!(
                "Effect::DealDamageEach {{ amount: {n}, filter: &{name} }}"
            )]);
        }
        if api == "AnimateAll" && targets_a_player {
            let mut probe = p.clone();
            let effs = self.effect_of_inner(
                api,
                &mut probe,
                target,
                LineFlags {
                    targets_here: false,
                    targets_a_player: true,
                    single_target,
                    prior_target_used: false,
                },
            )?;
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition"))
            {
                return None;
            }
            *p = probe;
            return Some(effs);
        }
        None
    }

    /// Consumes a Slot-2 `Tap`'s immediate `PutCounter | Defined$ ParentTarget`
    /// subability (*Stunning Shot*) so the counter lands on Slot 2 rather than
    /// Slot 1.
    pub(super) fn wave29b_take_slot2_parent_put_counter(
        &mut self,
        sub: &mut Option<String>,
    ) -> Option<String> {
        let sub_name = sub.as_deref()?.trim();
        let body = self.svars.get(sub_name)?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "PutCounter" || sp.take("Defined").as_deref() != Some("ParentTarget") {
            return None;
        }
        sp.take("IsCurse");
        let code = sp.take("CounterType")?;
        if matches!(code.as_str(), "ENERGY" | "POISON" | "RAD") {
            return None;
        }
        let kind = counter_kind(&code)?;
        let n = self.amount_or_count_ext(sp.take("CounterNum").as_deref().unwrap_or("1"))?;
        let next_sub = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        *sub = next_sub;
        Some(format!(
            "Effect::WithSecondTarget(&[Effect::AddCounter {{ kind: {kind}, amount: {n} }}])"
        ))
    }

    /// Extra multi-target follow-up steps hooked into `wave27c_multi_target_step`:
    /// 1. When Slot 1 is a Player (`AnyPlayer` / `AnyOpponent`), Slot 2 is an
    ///    Object (`TargetSpec::Object(..)`), and a non-targeting follow-up line
    ///    operates on `Defined$ TargetedCard` / `Defined$ Targeted` /
    ///    `Defined$ Targeted.<Type>` (*Homesickness*, *Ms. Bumbleflower*,
    ///    *Jon Irenicus*), run that object instruction inside `WithSecondTarget`.
    /// 2. When Slot 1 is a Battlefield object and Slot 2 is a Graveyard card
    ///    (`TargetOnly` pair) followed by `ChangeZone | Origin$ Battlefield,Graveyard | Destination$ Exile | Defined$ Targeted`
    ///    (*The Spot, Living Portal*), exile Slot 1 and `WithSecondTarget` exile Slot 2.
    /// 3. When Slot 1 is `TargetOnly` (`Object`) and Slot 2 is `ChangeZone | ThisDefinedAndTgts$ ParentTarget | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True`
    ///    followed by `ChangeZone | Defined$ Remembered | Destination$ Battlefield`
    ///    (*Don & Leo, Problem Solvers*), emit `CaptureAffected` around `WithAllTargets` exile.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(super) fn wave29b_multi_target_step(
        &mut self,
        api: &str,
        p: &mut Params,
        _sub: &mut Option<String>,
        chain: &mut Chain,
        third_target_here: bool,
        had_second_target: bool,
        targets_here: bool,
    ) -> Option<(Vec<String>, Option<MultiTargetAction>)> {
        if third_target_here {
            return None;
        }

        // Case A: Slot 1 is Player + Slot 2 is Object + non-targeting object follow-up
        // (*Homesickness*: `PutCounter | Defined$ TargetedCard`;
        //  *Ms. Bumbleflower*: `Pump | Defined$ Targeted.Creature`).
        if had_second_target
            && !targets_here
            && chain.third_target.is_none()
            && matches!(
                chain.target.as_deref(),
                Some("TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent")
            )
            && chain
                .second_target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && matches!(api, "PutCounter" | "Pump" | "Tap" | "Untap" | "Goad")
            && p.peek("Defined").is_some_and(|d| {
                matches!(d, "TargetedCard" | "Targeted") || d.starts_with("Targeted.")
            })
        {
            let second_spec = chain.second_target.as_deref()?;
            let second_single = chain.second_target_up_to.is_none_or(|n| n <= 1)
                && chain.second_target_range.is_none();
            let mut probe = p.clone();
            if probe.peek("Defined") == Some("TargetedCard") {
                probe.take("Defined");
                probe
                    .entries
                    .push(("Defined".to_string(), "Targeted".to_string()));
            }
            let inner = self.effect_of_inner(
                api,
                &mut probe,
                Some(second_spec),
                LineFlags {
                    targets_here: false,
                    targets_a_player: false,
                    single_target: second_single,
                    prior_target_used: true,
                },
            )?;
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition"))
                || inner.is_empty()
            {
                return None;
            }
            *p = probe;
            return Some((
                vec![format!("Effect::WithSecondTarget(&[{}])", inner.join(", "))],
                None,
            ));
        }

        // Case B: *The Spot, Living Portal*:
        // Slot 1 `Object(..)` + Slot 2 `CardInGraveyard(..)` ->
        // `ChangeZone | Origin$ Battlefield,Graveyard | Destination$ Exile | Defined$ Targeted`.
        if had_second_target
            && !targets_here
            && chain.third_target.is_none()
            && api == "ChangeZone"
            && p.peek("Defined") == Some("Targeted")
            && matches!(
                p.peek("Origin"),
                Some("Battlefield,Graveyard" | "Graveyard,Battlefield")
            )
            && p.peek("Destination") == Some("Exile")
            && chain
                .target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && chain
                .second_target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::CardInGraveyard("))
        {
            let s1 = chain.target.as_deref()?;
            let s2 = chain.second_target.as_deref()?;
            let mut probe = p.clone();
            probe.take("Defined");
            probe.take("Origin");
            probe.take("Destination");
            probe.take("IsCurse");
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition"))
            {
                return None;
            }
            let ctor = if self.has_linked_exile_reader() {
                "Effect::exile_linked"
            } else {
                "Effect::exile"
            };
            *p = probe;
            return Some((
                vec![
                    format!("{ctor}({s1})"),
                    format!("Effect::WithSecondTarget(&[{ctor}({s2})])"),
                ],
                None,
            ));
        }

        // Case C: *Teferi, Who Slows the Sunset* `[+1]`:
        // After 2 or 3 single-object slots (`max <= 1` on each), lower
        // `Tap | Defined$ Targeted.<qual>` / `Untap | Defined$ Targeted.<qual>`
        // across each slot via `Effect::IfTargetMatches`.
        if had_second_target
            && !targets_here
            && matches!(api, "Tap" | "Untap")
            && let Some(qual) = p
                .peek("Defined")
                .and_then(|d| d.strip_prefix("Targeted."))
                .map(str::to_string)
            && Self::wave29b_all_object_slots_at_most_one(chain)
        {
            let mut probe = p.clone();
            probe.take("Defined");
            probe.take("IsCurse");
            if !probe
                .entries
                .iter()
                .all(|(k, _)| k.starts_with("Condition"))
            {
                return None;
            }
            let qual_expr = self.filter_expr(qual.trim())?;
            let f_ref = self.wave29b_filter_ref(&qual_expr);
            let op = if api == "Tap" {
                "Effect::TapTarget"
            } else {
                "Effect::UntapTarget"
            };
            let guarded = format!("Effect::IfTargetMatches {{ filter: {f_ref}, then: &[{op}] }}");
            let mut out = vec![
                guarded.clone(),
                format!("Effect::WithSecondTarget(&[{guarded}])"),
            ];
            if chain.third_target.is_some() {
                out.push(format!("Effect::WithThirdTarget(&[{guarded}])"));
            }
            *p = probe;
            return Some((out, None));
        }

        None
    }

    fn wave29b_all_object_slots_at_most_one(chain: &Chain) -> bool {
        let s1_ok = chain
            .target
            .as_deref()
            .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && chain.target_exact.is_none()
            && chain.target_range.is_none()
            && !chain.target_is_x
            && chain.target_up_to.is_none_or(|n| n <= 1);
        let s2_ok = chain
            .second_target
            .as_deref()
            .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && chain.second_target_range.is_none()
            && chain.second_target_up_to.is_none_or(|n| n <= 1);
        let s3_ok = chain.third_target.as_deref().is_none_or(|t| {
            t.starts_with("TargetSpec::Object(")
                && chain.third_target_range.is_none()
                && chain.third_target_up_to.is_none_or(|n| n <= 1)
        });
        s1_ok && s2_ok && s3_ok
    }

    /// Extra Slot-2 admissions for `wave27c_chain_extra_target`:
    /// - *Betrayal at the Vault*: Slot-2 multi-target bite (`DealDamage | DamageSource$ ParentTarget`).
    /// - *Don & Leo, Problem Solvers*: Slot-1 `TargetOnly` + Slot-2 `ChangeZone | ThisDefinedAndTgts$ ParentTarget` blink.
    pub(super) fn wave29b_extra_target_slot2_allowed(
        &self,
        api: &str,
        p: &Params,
        chain: &Chain,
        second_spec: &str,
    ) -> bool {
        let single_object_first = chain
            .target
            .as_deref()
            .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && chain.target_exact.is_none()
            && chain.target_range.is_none()
            && !chain.target_is_x
            && chain.target_up_to.is_none_or(|n| n <= 1);
        if api == "DealDamage"
            && Self::wave29b_second_target_bite_allowed(p, single_object_first)
            && !self.later_line_targets(p.peek("SubAbility"))
        {
            return true;
        }
        if api == "ChangeZone"
            && chain.effects.is_empty()
            && second_spec.starts_with("TargetSpec::Object(")
            && chain
                .target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && p.peek("ThisDefinedAndTgts") == Some("ParentTarget")
            && p.peek("Origin") == Some("Battlefield")
            && p.peek("Destination") == Some("Exile")
            && p.peek("RememberChanged") == Some("True")
            && !self.later_line_targets(p.peek("SubAbility"))
            && let Some(ret_name) = p.peek("SubAbility")
            && let Some(ret_body) = self.svars.get(ret_name.trim())
            && let Some((ret_api, rp)) = Params::parse(ret_body)
        {
            return ret_api == "ChangeZone"
                && matches!(rp.peek("Defined"), Some("Remembered" | "RememberedLKI"))
                && matches!(rp.peek("Origin"), Some("Exile" | "All"))
                && rp.peek("Destination") == Some("Battlefield");
        }
        false
    }

    /// Lowers *Don & Leo, Problem Solvers*: Slot-2 `ChangeZone | ThisDefinedAndTgts$ ParentTarget |
    /// Origin$ Battlefield | Destination$ Exile | RememberChanged$ True -> DBReturn -> [Cleanup]`
    /// into `Effect::WithAllTargets(&[Effect::Blink { target: TargetSpec::Object(&Filter::Any), owner_control }])`.
    pub(super) fn wave29b_two_slot_blink_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if !chain.effects.is_empty()
            || !chain
                .target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            || !chain
                .second_target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
        {
            return None;
        }
        let mut probe = p.clone();
        if probe.take("ThisDefinedAndTgts").as_deref() != Some("ParentTarget")
            || probe.take("Origin").as_deref() != Some("Battlefield")
            || probe.take("Destination").as_deref() != Some("Exile")
            || probe.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        probe.take("IsCurse");
        if !probe.exhausted() {
            return None;
        }
        let ret_name = sub.as_deref()?.trim().to_string();
        let ret_body = self.svars.get(&ret_name)?.clone();
        let (ret_api, mut rp) = Params::parse(&ret_body)?;
        rp.drop_prose();
        if ret_api != "ChangeZone"
            || !matches!(
                rp.take("Defined").as_deref(),
                Some("Remembered" | "RememberedLKI")
            )
            || !matches!(rp.take("Origin").as_deref(), Some("Exile" | "All"))
            || rp.take("Destination").as_deref() != Some("Battlefield")
        {
            return None;
        }
        let owner_control = match rp.take("GainControl").as_deref() {
            None | Some("False") => true,
            Some("True") => false,
            Some(_) => return None,
        };
        let mut after_ret = rp.take("SubAbility");
        if !rp.exhausted() {
            return None;
        }
        if let Some(cname) = after_ret.clone() {
            self.consume_optional_remember_cleanup(cname, &mut after_ret)?;
        }
        *p = probe;
        *sub = after_ret;
        Some(vec![format!(
            "Effect::WithAllTargets(&[Effect::Blink {{ target: TargetSpec::Object(&Filter::Any), owner_control: {owner_control} }}])"
        )])
    }

    /// Admits 3-slot `TargetOnly` chains (each slot `0..=1` / `1` `Object`) whose
    /// follow-up is `Tap`/`Untap` with `Defined$ Targeted.<qual>` (*Teferi, Who Slows the Sunset* `[+1]`).
    pub(super) fn wave29b_can_three_slot_qualified_tap_untap(
        &self,
        sub: Option<&str>,
        chain: &Chain,
        second_spec: &str,
        slot3_params: &Params,
    ) -> bool {
        let first_ok = chain
            .target
            .as_deref()
            .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && chain.target_exact.is_none()
            && chain.target_range.is_none()
            && !chain.target_is_x
            && chain.target_up_to.is_none_or(|n| n <= 1);
        if !first_ok || !second_spec.starts_with("TargetSpec::Object(") {
            return false;
        }
        let s3_max_ok = slot3_params
            .peek("TargetMax")
            .is_none_or(|v| matches!(v.trim(), "1"));
        if !s3_max_ok {
            return false;
        }
        let Some(sub_name) = sub else {
            return false;
        };
        let Some(body) = self.svars.get(sub_name.trim()) else {
            return false;
        };
        let Some((api, sp)) = Params::parse(body) else {
            return false;
        };
        matches!(api.as_str(), "Tap" | "Untap")
            && !sp.has("ValidTgts")
            && sp
                .peek("Defined")
                .is_some_and(|d| d.starts_with("Targeted."))
            && !self.later_line_targets(sp.peek("SubAbility"))
    }

    // =========================================================================
    // Part 4: `PutCounter`, `Pump`, & `AttackersDeclaredOneTarget` extensions
    // =========================================================================

    /// Additional `PutCounter | Defined$` values:
    /// - `AttachedBy Returned` when `self.cost_returns_self` (*Krasis Incubation*)
    ///   -> `Effect::ChangeAttachedCounters` (uses `source_attachment_lki`).
    /// - `TriggeredAttackers` on `Mode$ AttackersDeclaredOneTarget` when the
    ///   trigger requires `ValidAttackersAmount$ EQ1` (*Jiang Yanggu, Alone*).
    pub(super) fn wave29b_put_counter_defined(
        &self,
        def: &str,
        target: Option<&str>,
        specs: &[(String, String)],
    ) -> Option<Vec<String>> {
        if def == "AttachedBy Returned"
            && self.cost_returns_self
            && !self.on_a_spell
            && target.is_none()
        {
            return Some(
                specs
                    .iter()
                    .map(|(kind, n)| {
                        format!(
                            "Effect::ChangeAttachedCounters {{ kind: {kind}, amount: {n}, remove: false }}"
                        )
                    })
                    .collect(),
            );
        }
        None
    }

    /// Multi-step `AttackersDeclaredOneTarget` chain with `ValidAttackersAmount$ EQ1`
    /// where a later subability puts counters on `Defined$ TriggeredAttackers`
    /// (*Jiang Yanggu, Alone*: `Discard -> Draw -> PutCounter | Defined$ TriggeredAttackers`).
    pub(super) fn wave29b_attackers_at_target_chain(
        &mut self,
        body: &str,
        exact_count_is_one: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        if !exact_count_is_one {
            return None;
        }
        let (api, mut p) = Params::parse(body)?;
        p.drop_prose();
        if api != "Discard"
            || p.take("Defined").as_deref() != Some("You")
            || p.take("Mode").as_deref() != Some("TgtChoose")
            || p.take("NumCards").as_deref() != Some("1")
        {
            return None;
        }
        let sub1 = p.take("SubAbility")?;
        if !p.exhausted() {
            return None;
        }
        let draw_body = self.svars.get(sub1.trim())?.clone();
        let (draw_api, mut dp) = Params::parse(&draw_body)?;
        dp.drop_prose();
        if draw_api != "Draw"
            || !matches!(dp.take("Defined").as_deref(), None | Some("You"))
            || !matches!(dp.take("NumCards").as_deref(), None | Some("1"))
        {
            return None;
        }
        let sub2 = dp.take("SubAbility")?;
        if !dp.exhausted() {
            return None;
        }
        let put_body = self.svars.get(sub2.trim())?.clone();
        let (put_api, mut pp) = Params::parse(&put_body)?;
        pp.drop_prose();
        if put_api != "PutCounter"
            || pp.take("Defined").as_deref() != Some("TriggeredAttackers")
            || pp.has("ValidTgts")
        {
            return None;
        }
        let kind = counter_kind(pp.take("CounterType")?.trim())?;
        let amount = self.amount_or_count_ext(pp.take("CounterNum").as_deref().unwrap_or("1"))?;
        if !pp.exhausted() {
            return None;
        }
        chain
            .effects
            .push("Effect::DiscardForPlayers { who: PlayerRel::You, count: 1 }".to_string());
        chain.effects.push("Effect::draw(1)".to_string());
        chain.effects.push(format!(
            "Effect::AddCounterFilter {{ filter: &Filter::Reference, kind: {kind}, amount: {amount} }}"
        ));
        Some(())
    }

    /// Special `Pump` protection forms:
    /// - `KW$ Protection:Spell.Instant,Spell.Sorcery:...` (*Devoted Caretaker*) ->
    ///   `Modifier::ProtectionFrom(&Filter::And(&[Filter::INSTANT_OR_SORCERY, Filter::InZone(ZoneRef::Stack)]))`
    ///   (abilities of instant/sorcery cards in graveyards have empty `TypeSet` on the stack,
    ///   so `INSTANT_OR_SORCERY` + `InZone(Stack)` matches instant and sorcery spells only).
    /// - `KW$ Protection:Player.OpponentOf PlayerUID_ChosenPlayerUID:... | DefinedKW$ You` (*Cliffside Rescuer*)
    ///   and `KW$ Protection:Player.PlayerUID_ChosenPlayerUID:... | DefinedKW$ Opponent` (*Courageous Resolve*)
    ///   -> `Modifier::ProtectionFrom(&Filter::ControlledByOpponent)`.
    /// - `KW$ Protection:Player.PlayerUID_ChosenPlayerUID:... | DefinedKW$ TargetedPlayer`
    ///   on `Defined$ You` or `Defined$ Valid <F>` (*Eon Frolicker*) ->
    ///   `Modifier::PlayerProtectionFrom` / `Modifier::ProtectionFrom(&Filter::ControlledBy(PlayerRel::Chosen))`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29b_pump_effect(
        &mut self,
        p: &mut Params,
        aimed: &str,
        target: Option<&str>,
        second: bool,
    ) -> Option<Vec<String>> {
        if second || p.has("NumAtt") || p.has("NumDef") {
            return None;
        }
        let kw = p.peek("KW")?.trim();
        if !kw.starts_with("Protection:") || kw.contains('&') {
            return None;
        }
        let mut probe = p.clone();
        let kw = probe.take("KW")?;
        let defined_kw = probe.take("DefinedKW");
        let duration = match probe.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(_) => return None,
        };
        probe.take("IsCurse");
        let defined = probe.take("Defined");
        if !probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }

        let spec_head = kw.strip_prefix("Protection:")?.split(':').next()?.trim();

        // 1. Instant and sorcery spells (*Devoted Caretaker*):
        if matches!(
            spec_head,
            "Spell.Instant,Spell.Sorcery" | "Spell.Sorcery,Spell.Instant"
        ) && defined_kw.is_none()
        {
            let ctor = match defined.as_deref() {
                None | Some("Targeted" | "ParentTarget")
                    if aimed.starts_with("TargetSpec::Object(") =>
                {
                    "ModifyTargets"
                }
                Some("Self") if !self.on_a_spell => "ModifySource",
                _ => return None,
            };
            let from = self.body.filter_static(
                "PROT",
                "Filter::And(&[Filter::INSTANT_OR_SORCERY, Filter::InZone(ZoneRef::Stack)])",
            );
            *p = probe;
            return Some(vec![format!(
                "Effect::{ctor} {{ modifier: Modifier::ProtectionFrom(&{from}), duration: {duration} }}"
            )]);
        }

        // 2. Protection from each of your opponents (*Cliffside Rescuer*, *Courageous Resolve*):
        let is_from_opponents = (spec_head == "Player.OpponentOf PlayerUID_ChosenPlayerUID"
            && defined_kw.as_deref() == Some("You"))
            || (spec_head == "Player.PlayerUID_ChosenPlayerUID"
                && matches!(defined_kw.as_deref(), Some("Opponent" | "Player.Opponent")));
        if is_from_opponents {
            let ctor = match defined.as_deref() {
                None | Some("Targeted" | "ParentTarget")
                    if aimed.starts_with("TargetSpec::Object(") =>
                {
                    "ModifyTargets"
                }
                Some("Self") if !self.on_a_spell => "ModifySource",
                _ => return None,
            };
            *p = probe;
            return Some(vec![format!(
                "Effect::{ctor} {{ modifier: Modifier::ProtectionFrom(&Filter::ControlledByOpponent), duration: {duration} }}"
            )]);
        }

        // 3. Protection from targeted player (*Eon Frolicker*):
        if spec_head == "Player.PlayerUID_ChosenPlayerUID"
            && matches!(defined_kw.as_deref(), Some("TargetedPlayer" | "Targeted"))
            && matches!(
                target,
                Some(
                    "TargetSpec::AnyPlayer"
                        | "TargetSpec::AnyOpponent"
                        | "TargetSpec::Player(PlayerRel::Chosen)"
                )
            )
        {
            let chosen_ctrl = self
                .body
                .filter_static("PROT", "Filter::ControlledBy(PlayerRel::Chosen)");
            match defined.as_deref() {
                Some("You") => {
                    *p = probe;
                    return Some(vec![format!(
                        "Effect::continuous(&Filter::Any, Modifier::PlayerProtectionFrom {{ \
                         who: PlayerRel::You, from: &{chosen_ctrl} }}, {duration})"
                    )]);
                }
                Some(raw) if raw.starts_with("Valid ") => {
                    let expr = self.filter_expr(raw["Valid ".len()..].trim())?;
                    let affected = self.body.filter_static("PUMP", &expr);
                    *p = probe;
                    return Some(vec![format!(
                        "Effect::continuous(&{affected}, Modifier::ProtectionFrom(&{chosen_ctrl}), {duration})"
                    )]);
                }
                _ => return None,
            }
        }

        None
    }

    // =========================================================================
    // Part 5: Pass 2 additions (`Cyclops Gladiator`, `Steer Clear`, `Flux`,
    //         `Molten Psyche`, qualified multi-player `Draw` / `LoseLife`,
    //         `Rakdos Augermage`, `Essence Flux`, `Justiciar's Portal`)
    // =========================================================================

    pub(super) fn wave29b_oracle_deals_damage_to_self(&self) -> bool {
        if self.script_name.is_empty() {
            return false;
        }
        let o = self.oracle.to_ascii_lowercase();
        let full = format!(
            "damage equal to its power to {}",
            self.script_name.to_ascii_lowercase()
        );
        if o.contains(&full) || o.contains("damage equal to its power to this ") {
            return true;
        }
        if let Some((short, _)) = self.script_name.split_once(',') {
            let short_pat = format!(
                "damage equal to its power to {}",
                short.trim().to_ascii_lowercase()
            );
            if o.contains(&short_pat) {
                return true;
            }
        }
        false
    }

    fn wave29b_cast_time_subtype_condition(&self, var: &str, cmp: &str) -> Option<(String, bool)> {
        let swap_arms = match cmp {
            "GE1" | "GT0" => false,
            "EQ0" | "LE0" | "LT1" => true,
            _ => return None,
        };
        let def = self.svars.get(var)?.trim();
        let raw_valid = def.strip_prefix("Count$LastStateBattlefieldWithFallback ")?;
        let stripped = Self::wave29b_strip_ctrl_atom(raw_valid.trim(), "YouCtrl")?;
        let sub_word = if let Some((base, rest)) = stripped.split_once('.') {
            if rest.contains('+') || !matches!(base, "Permanent" | "Card" | "Creature") {
                return None;
            }
            rest.trim()
        } else {
            if stripped.contains('+') {
                return None;
            }
            stripped.trim()
        };
        let sub_path = self.cats.const_path(sub_word)?;
        if !sub_path.starts_with("subtypes::creature::") {
            return None;
        }
        Some((
            format!("Condition::ControlledSubtypeOnCast({sub_path})"),
            swap_arms,
        ))
    }

    /// Simultaneous APNAP `RepeatEach` steps for *Flux* and *Molten Psyche*.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave29b_repeat_each_step(
        &mut self,
        api: &str,
        sp: &Params,
        next_sub: Option<&str>,
    ) -> Option<(Vec<String>, Option<String>)> {
        let is_rem_player = |d: Option<&str>| {
            matches!(
                d,
                Some("Remembered" | "Player.IsRemembered" | "Player.Remembered")
            )
        };
        // 1. *Flux*: `Discard (AnyNumber, RememberDiscarded) -> Draw (Remembered$Amount)`.
        if api == "Discard"
            && is_rem_player(sp.peek("Defined"))
            && sp.peek("AnyNumber") == Some("True")
            && sp.peek("Mode") == Some("TgtChoose")
            && sp.peek("RememberDiscarded") == Some("True")
            && let Some(draw_name) = next_sub
            && let Some(draw_body) = self.svars.get(draw_name.trim()).cloned()
            && let Some((draw_api, mut dp)) = Params::parse(&draw_body)
        {
            dp.drop_prose();
            let mut probe = sp.clone();
            probe.take("Defined");
            probe.take("AnyNumber");
            if let Some(opt) = probe.take("Optional")
                && opt != "True"
            {
                return None;
            }
            probe.take("Mode");
            probe.take("RememberDiscarded");
            probe.take("IsCurse");
            let after_draw = dp.take("SubAbility");
            let draw_var = dp.take("NumCards");
            let draw_def = dp.take("Defined");
            if probe.exhausted()
                && draw_api == "Draw"
                && is_rem_player(draw_def.as_deref())
                && dp.exhausted()
                && draw_var
                    .as_deref()
                    .and_then(|v| self.svars.get(v.trim()))
                    .map(|s| s.trim())
                    == Some("Remembered$Amount")
            {
                return Some((
                    vec![
                        "Effect::DiscardUpToForPlayers { who: PlayerRel::EventPlayer, max: u8::MAX }"
                            .to_string(),
                        "Effect::DrawCardsFor { amount: Amount::ThisWayBy { what: ThisWay::Discarded, who: PlayerRel::EventPlayer }, who: PlayerRel::EventPlayer }"
                            .to_string(),
                    ],
                    after_draw,
                ));
            }
        }

        // 2. *Molten Psyche*: `ChangeZoneAll (Hand -> Library, Shuffle, RememberChanged) ->
        //    Draw (Remembered$Amount) -> DealDamage (Defined$ Player.Opponent+IsRemembered,
        //    NumDmg$ <PlayerCountRemembered$CardsDrawn>, ConditionPresent$ Artifact.YouCtrl, ConditionCompare$ GE3)`.
        if api == "ChangeZoneAll"
            && is_rem_player(sp.peek("Defined"))
            && sp.peek("Origin") == Some("Hand")
            && sp.peek("Destination") == Some("Library")
            && sp.peek("Shuffle") == Some("True")
            && sp.peek("RememberChanged") == Some("True")
            && let Some(draw_name) = next_sub
            && let Some(draw_body) = self.svars.get(draw_name.trim()).cloned()
            && let Some((draw_api, mut dp)) = Params::parse(&draw_body)
        {
            dp.drop_prose();
            let mut probe = sp.clone();
            probe.take("Defined");
            probe.take("Origin");
            probe.take("Destination");
            probe.take("Shuffle");
            probe.take("RememberChanged");
            probe.take("LibraryPosition");
            let dmg_name = dp.take("SubAbility")?;
            let draw_var = dp.take("NumCards");
            let draw_def = dp.take("Defined");
            if !probe.exhausted()
                || draw_api != "Draw"
                || !is_rem_player(draw_def.as_deref())
                || !dp.exhausted()
                || draw_var
                    .as_deref()
                    .and_then(|v| self.svars.get(v.trim()))
                    .map(|s| s.trim())
                    != Some("Remembered$Amount")
            {
                return None;
            }
            let dmg_body = self.svars.get(dmg_name.trim())?.clone();
            let (dmg_api, mut mp) = Params::parse(&dmg_body)?;
            mp.drop_prose();
            let after_dmg = mp.take("SubAbility");
            let dmg_var = mp.take("NumDmg")?;
            if dmg_api != "DealDamage"
                || !matches!(
                    mp.take("Defined").as_deref(),
                    Some("Player.Opponent+IsRemembered" | "Player.IsRemembered+Opponent")
                )
                || mp.take("ConditionPresent").as_deref() != Some("Artifact.YouCtrl")
                || mp.take("ConditionCompare").as_deref() != Some("GE3")
                || !mp.exhausted()
                || self.svars.get(dmg_var.trim()).map(|s| s.trim())
                    != Some("PlayerCountRemembered$CardsDrawn")
            {
                return None;
            }
            let mut effs = Self::shuffle_and_redraw_effects(
                "PlayerRel::EventPlayer",
                "PlayerRel::EventPlayer",
            );
            effs.push("__W29B_MOLTEN_PSYCHE_TAIL__".to_string());
            return Some((effs, after_dmg));
        }

        None
    }

    pub(super) fn wave29b_collapse_repeat_each(
        &mut self,
        domain: &str,
        body_effects: &[String],
        has_prior_effects: bool,
    ) -> Result<Option<Vec<String>>, ()> {
        let flux_pair = [
            "Effect::DiscardUpToForPlayers { who: PlayerRel::EventPlayer, max: u8::MAX }"
                .to_string(),
            "Effect::DrawCardsFor { amount: Amount::ThisWayBy { what: ThisWay::Discarded, who: PlayerRel::EventPlayer }, who: PlayerRel::EventPlayer }"
                .to_string(),
        ];
        if domain == "RepeatDomain::EachPlayer" && body_effects == flux_pair {
            if has_prior_effects {
                let _: Option<()> =
                    self.deny("a shared discard and redraw after earlier effects".to_string());
                return Err(());
            }
            return Ok(Some(vec![
                "Effect::DiscardUpToForPlayers { who: PlayerRel::EachPlayer, max: u8::MAX }"
                    .to_string(),
                "Effect::DrawCardsFor { amount: Amount::ThisWayBy { what: ThisWay::Discarded, who: PlayerRel::Recipient }, who: PlayerRel::EachPlayer }"
                    .to_string(),
            ]));
        }

        let mut molten =
            Self::shuffle_and_redraw_effects("PlayerRel::EventPlayer", "PlayerRel::EventPlayer");
        molten.push("__W29B_MOLTEN_PSYCHE_TAIL__".to_string());
        if body_effects == molten {
            if domain != "RepeatDomain::EachPlayer" || has_prior_effects {
                let _: Option<()> = self.deny(
                    "a shared wheel and metalcraft damage in unsupported context".to_string(),
                );
                return Err(());
            }
            let mut out =
                Self::shuffle_and_redraw_effects("PlayerRel::EachPlayer", "PlayerRel::Recipient");
            out.push(
                "Effect::IfCondition { condition: Condition::ControlCount(&Filter::ARTIFACT, 3), \
                 then: &[Effect::RepeatEach { domain: RepeatDomain::EachOpponent, \
                 effects: &[Effect::DealDamage { amount: Amount::CardsDrawnThisTurn(PlayerRel::EventPlayer), \
                 target: TargetSpec::Player(PlayerRel::EventPlayer) }] }], otherwise: &[] }"
                    .to_string(),
            );
            return Ok(Some(out));
        }
        if body_effects
            .iter()
            .any(|e| e == "__W29B_MOLTEN_PSYCHE_TAIL__")
        {
            return Err(());
        }

        Ok(None)
    }

    fn wave29b_parse_qualified_multi_player(
        &mut self,
        defined: &str,
    ) -> Option<(&'static str, String)> {
        let rest = defined.trim().strip_prefix("Player.")?;
        let (domain, qual) = if let Some(q) = rest.strip_prefix("Opponent+") {
            ("RepeatDomain::EachOpponent", q)
        } else {
            ("RepeatDomain::EachPlayer", rest)
        };
        if qual == "HasCardsInHand_Card_EQ0" {
            return Some((
                domain,
                "Condition::AmountCompare { left: &Amount::HandSize(PlayerRel::EventPlayer), \
                 op: AmountCmp::Equal, right: &Amount::Fixed(0) }"
                    .to_string(),
            ));
        }
        let ctrl_raw = qual.strip_prefix("controls")?;
        let (valid, cmp_op, rhs_n) = if let Some(v) = ctrl_raw.strip_suffix("_GE1") {
            (v, "AmountCmp::AtLeast", 1u32)
        } else if let Some(v) = ctrl_raw.strip_suffix("_EQ0") {
            (v, "AmountCmp::Equal", 0u32)
        } else if ["_EQ", "_GE", "_LE", "_GT", "_LT", "_NE"]
            .iter()
            .any(|p| ctrl_raw.contains(p))
        {
            return None;
        } else {
            (ctrl_raw, "AmountCmp::AtLeast", 1u32)
        };
        if valid.is_empty()
            || valid.contains("Ctrl")
            || valid.contains("Own")
            || valid.contains(['$', '/'])
        {
            return None;
        }
        let expr = self.filter_expr(valid.trim())?;
        let f_ref = self.wave29b_filter_ref(&expr);
        let cond = format!(
            "Condition::AmountCompare {{ left: &Amount::ControlledCount {{ \
             who: PlayerRel::EventPlayer, filter: {f_ref} }}, op: {cmp_op}, right: &Amount::Fixed({rhs_n}) }}"
        );
        Some((domain, cond))
    }

    pub(super) fn wave29b_qualified_multi_player_draw(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if target.is_some() || targets_a_player || p.has("Optional") || p.has("OptionalDecider") {
            return None;
        }
        let def = p.peek("Defined")?.to_string();
        let (domain, cond) = self.wave29b_parse_qualified_multi_player(&def)?;
        let mut probe = p.clone();
        probe.take("Defined");
        let n = self.amount_or_count_ext(probe.take("NumCards").as_deref().unwrap_or("1"))?;
        if !probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        *p = probe;
        Some(vec![format!(
            "Effect::RepeatEach {{ domain: {domain}, effects: &[Effect::IfCondition {{ \
             condition: {cond}, then: &[Effect::DrawCardsFor {{ amount: {n}, who: PlayerRel::EventPlayer }}], \
             otherwise: &[] }}] }}"
        )])
    }

    pub(super) fn wave29b_qualified_multi_player_lose_life(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if target.is_some() || targets_a_player {
            return None;
        }
        let def = p.peek("Defined")?.to_string();
        let (domain, cond) = self.wave29b_parse_qualified_multi_player(&def)?;
        let mut probe = p.clone();
        probe.take("Defined");
        probe.take("IsCurse");
        let n = self.amount_or_count_ext(&probe.take("LifeAmount")?)?;
        if !probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        *p = probe;
        Some(vec![format!(
            "Effect::RepeatEach {{ domain: {domain}, effects: &[Effect::IfCondition {{ \
             condition: {cond}, then: &[Effect::LoseLife {{ amount: {n}, target: PlayerRel::EventPlayer }}], \
             otherwise: &[] }}] }}"
        )])
    }

    /// `Discard | Mode$ RevealTgtChoose` (*Rakdos Augermage*): reveal your hand
    /// and target player/opponent chooses `n` cards from it that you discard.
    pub(super) fn wave29b_discard_reveal_tgt_choose(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("Mode") != Some("RevealTgtChoose") || p.has("RevealNumber") || p.has("Optional") {
            return None;
        }
        let has_chosen_player = targets_a_player
            || matches!(
                target,
                Some(
                    "TargetSpec::AnyPlayer"
                        | "TargetSpec::AnyOpponent"
                        | "TargetSpec::Player(PlayerRel::Chosen)"
                        | "TargetSpec::PlayerWhoAttackedThisTurn"
                )
            );
        if !has_chosen_player {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Mode");
        probe.take("IsCurse");
        if let Some(rem) = probe.take("RememberDiscarded")
            && rem != "True"
        {
            return None;
        }
        if probe.take("Defined").as_deref() != Some("You") {
            return None;
        }
        let raw_num = probe.take("NumCards").unwrap_or_else(|| "1".to_string());
        let n = self
            .plain_num(&raw_num)
            .and_then(|v| u8::try_from(v).ok())
            .filter(|&v| v >= 1)?;
        let valid = probe.take("DiscardValid");
        if probe.take("DiscardValidDesc").is_some() && valid.is_none() {
            return None;
        }
        if !probe
            .entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let filter_ref = match valid.as_deref() {
            None | Some("Card") => "&Filter::Any".to_string(),
            Some(v) => {
                let expr = self.filter_expr_in_zone(v, true)?;
                self.wave29b_filter_ref(&expr)
            }
        };
        *p = probe;
        Some(vec![
            "Effect::RevealHand { who: PlayerRel::You }".to_string(),
            format!(
                "Effect::ChooseCardEx {{ chooser: PlayerRel::Chosen, zone: ChooseZone::Hand(PlayerRel::You), \
                 filter: {filter_ref}, min: {n}, max: Amount::Fixed({n}), reveal_chosen: false, \
                 movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
                 then: &[Effect::DiscardDefined {{ cards: DefinedObjects::Found }}], \
                 for_unchosen: &[] }}"
            ),
        ])
    }

    /// Immediate single-target blink whose `DBReturn` has a non-`Cleanup`
    /// follow-up reading `Remembered` (*Essence Flux*, *Justiciar's Portal*).
    pub(super) fn wave29b_blink_with_followup(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if !single_target || !targets_here || p.peek("Defined").is_some() {
            return None;
        }
        let blink_target = target.filter(|t| t.starts_with("TargetSpec::Object("))?;
        if p.peek("Origin") != Some("Battlefield") || p.peek("Destination") != Some("Exile") {
            return None;
        }
        let ret_name = sub.as_deref()?.trim().to_string();
        let ret_body = self.svars.get(&ret_name)?.clone();
        let (ret_api, mut dp) = Params::parse(&ret_body)?;
        dp.drop_prose();
        if ret_api != "ChangeZone"
            || !matches!(
                dp.take("Defined").as_deref(),
                Some("Remembered" | "RememberedLKI")
            )
            || !matches!(dp.take("Origin").as_deref(), Some("Exile" | "All"))
            || dp.take("Destination").as_deref() != Some("Battlefield")
        {
            return None;
        }
        let after_return = dp.take("SubAbility")?;
        if self.sub_is_api(Some(&after_return), "Cleanup") {
            return None;
        }
        let owner_control = match dp.take("GainControl").as_deref() {
            None | Some("False") => true,
            Some("True") => false,
            Some(_) => return None,
        };
        if !dp.exhausted() {
            return None;
        }
        let mut cap_p = p.clone();
        cap_p.take("Origin");
        cap_p.take("Destination");
        cap_p.take("IsCurse");
        let mut next_sub = Some(after_return);
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        if let Some(effs) = self.wave27c_capture_affected_chain(
            "ChangeZone",
            &mut cap_p,
            &mut next_sub,
            Some(blink_target),
            LineFlags {
                targets_here: true,
                targets_a_player: false,
                single_target: true,
                prior_target_used: false,
            },
            chain,
            Some(vec![format!(
                "Effect::Blink {{ target: {blink_target}, owner_control: {owner_control} }}"
            )]),
        ) && cap_p.exhausted()
        {
            *p = cap_p;
            *sub = next_sub;
            Some(effs)
        } else {
            self.body.statics.truncate(saved_statics);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            None
        }
    }

    /// Constrain `Filter::IsRemembered` effects with `ConditionCheckSVar$` naming
    /// `Remembered$Valid <qual>` when `CaptureScope::single_object` holds
    /// (*Essence Flux*: `"If it's a Spirit, put a +1/+1 counter on it"`).
    pub(super) fn wave29b_check_svar_wrap(
        &mut self,
        _api: &str,
        p: &mut Params,
        effects: &[String],
    ) -> Option<Vec<String>> {
        if !self.capture_scope.as_ref().is_some_and(|s| s.single_object)
            || effects.is_empty()
            || !effects.iter().all(|e| e.contains("&Filter::IsRemembered"))
        {
            return None;
        }
        let var = p.peek("ConditionCheckSVar")?.trim().to_string();
        let cmp = p.peek("ConditionSVarCompare").unwrap_or("GE1").trim();
        if !matches!(cmp, "GE1" | "EQ1" | "GT0" | "NE0") {
            return None;
        }
        let def = self.svars.get(&var)?.trim().to_string();
        let valid = def.strip_prefix("Remembered$Valid ")?.trim();
        if valid.is_empty()
            || valid.contains(['$', '/'])
            || valid.contains("Ctrl")
            || valid.contains("Own")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("ConditionCheckSVar");
        probe.take("ConditionSVarCompare");
        probe.take("ConditionDescription");
        if !probe.exhausted() {
            return None;
        }
        let qual_expr = self.filter_expr(valid)?;
        let combined = format!("Filter::And(&[Filter::IsRemembered, {qual_expr}])");
        let f_ref = self.wave29b_filter_ref(&combined);
        *p = probe;
        Some(
            effects
                .iter()
                .map(|e| e.replace("&Filter::IsRemembered", &f_ref))
                .collect(),
        )
    }

    /// True if `start` or any transitive `SubAbility$` still reads `Remembered`
    /// or runs `Cleanup | ClearRemembered$ True` (so `remembered_this_way_chain`
    /// yields to `wave27c_try_capture_affected_chain`, e.g. *Balin, Loremaster*).
    pub(super) fn wave29b_sub_chain_reads_remembered(&self, start: &str) -> bool {
        let mut cur = Some(start.trim().to_string());
        for _ in 0..8 {
            let Some(name) = cur.take() else {
                break;
            };
            let Some(body) = self.svars.get(name.trim()) else {
                break;
            };
            let Some((api, sp)) = Params::parse(body) else {
                break;
            };
            if api == "Cleanup" && sp.peek("ClearRemembered") == Some("True") {
                return true;
            }
            for (_, v) in &sp.entries {
                if v.contains("Remembered") {
                    return true;
                }
                if let Some(sv_body) = self.svars.get(v.trim())
                    && sv_body.contains("Remembered")
                {
                    return true;
                }
            }
            cur = sp.peek("SubAbility").map(|s| s.trim().to_string());
        }
        false
    }
}
