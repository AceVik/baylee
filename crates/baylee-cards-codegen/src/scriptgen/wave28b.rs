//! Wave 28b pure-`scriptgen` lowerings for `DealDamage`, `Draw`, `PutCounter`,
//! `LoseLife`, `GainLife`, `DamageAll`, and optional trigger/Saga costs.
//!
//! Every rule here targets an existing, engine-verified `baylee-cards-dsl`
//! `Amount`, `Condition`, `Effect`, or `PlayerRel` variant with zero silent
//! parameter drops and zero Oracle approximations.

use super::{
    BlockRole, Chain, Params, Tx, activated::OptionalTriggerPrice, cost_parts, counter_kind,
    keyword_const,
};

impl Tx<'_> {
    /// Whether `trigger` is one of the six `baylee-engine` damage triggers that
    /// bind `event_object = damage_source` in `trigger/events.rs`.
    pub(super) fn wave28b_trigger_binds_damage_source(trigger: &str) -> bool {
        trigger.starts_with("Trigger::DealsDamageToPlayer {")
            || trigger.starts_with("Trigger::DealsCombatDamageToPlayer(")
            || trigger.starts_with("Trigger::OneOrMoreDealCombatDamageToPlayer(")
            || trigger.starts_with("Trigger::DealsCombatDamageToYou(")
            || trigger.starts_with("Trigger::DealsCombatDamage(")
            || trigger.starts_with("Trigger::DealsCombatDamageToPlayerOrPlaneswalker(")
    }

    /// Whether a `DamageDone`/`DamageDealtOnce` `ValidSource$` names the
    /// attached permanent (`EnchantedBy`/`EquippedBy`/`AttachedBy`).
    pub(super) fn wave28b_is_attached_damage_source(source: &str) -> bool {
        matches!(
            source.trim(),
            "Card.AttachedBy"
                | "Creature.AttachedBy"
                | "Permanent.AttachedBy"
                | "Card.EnchantedBy"
                | "Creature.EnchantedBy"
                | "Card.EquippedBy"
                | "Creature.EquippedBy"
        )
    }

    /// Whether an activation/spell cost contains a single-card `Discard<1/...>`
    /// token (so `Discarded$Valid` and `Discarded$CardMulticolor` refer to a
    /// single discarded card).
    pub(super) fn wave28b_cost_discards_single(cost: &str) -> bool {
        cost_parts(cost)
            .iter()
            .any(|tok| tok.starts_with("Discard<1/") && !tok.contains("/Hand"))
    }

    /// Additional `Count$` / `SVar` expressions lowered onto existing `Amount`
    /// variants. Called from `mana::count_expr_depth`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave28b_count_expr(
        &mut self,
        def: &str,
        depth: u8,
        static_only: bool,
    ) -> Option<String> {
        if depth >= 6 {
            return None;
        }
        let def = def.trim();
        if !static_only {
            if let Some(valid) = def
                .strip_prefix("Count$Valid ")
                .and_then(|s| s.strip_suffix("$GreatestCardPower"))
                && !valid.contains('/')
                && !valid.contains('$')
            {
                let expr = self.filter_expr(valid.trim())?;
                let f_ref = self.wave28b_filter_ref(&expr);
                return Some(format!("Amount::GreatestPowerAmong({f_ref})"));
            }
            if def == "Count$xPaid" && self.in_resolution_chosen_x {
                return Some("Amount::ResolutionChosenNumber".to_string());
            }
            if def == "Discarded$Amount" && self.in_discard_hand_may {
                return Some("Amount::ThisWay(ThisWay::Discarded)".to_string());
            }
            if def == "TargetedController$LandsInGraveyard" {
                return Some(
                    "Amount::CountOf { filter: &Filter::LAND, \
                     zone: ZoneSel::GraveyardOf(PlayerRel::ControllerOfTarget) }"
                        .to_string(),
                );
            }
            if def == "TriggeredActivator$SpellsCastThisTurn"
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("SpellCast" | "SpellCastOrCopy" | "AbilityCast" | "TapsForMana")
                )
            {
                return Some("Amount::SpellsCastThisTurn(PlayerRel::EventPlayer)".to_string());
            }
            if let Some(valid) = def
                .strip_prefix("Count$Valid ")
                .and_then(|s| s.strip_suffix("$Colors"))
                && !valid.contains(['$', '/'])
            {
                let expr = self.filter_expr(valid.trim())?;
                if !Self::wave28b_filter_uses_resolution_state(&expr) {
                    let f_ref = self.wave28b_filter_ref(&expr);
                    return Some(format!("Amount::DistinctColorsAmong({f_ref})"));
                }
            }
            if def == "Count$DifferentCounterKinds_Card.Self" && !self.on_a_spell {
                return Some("Amount::DistinctCounterKindsOnSource".to_string());
            }
            if def == "PlayerCountOpponents$HighestCardsInHand" {
                return Some("Amount::HighestOpponentHandSize".to_string());
            }
            if let Some(rest) = def.strip_prefix("Count$ThisTurnCast_")
                && rest.contains("!CastSaSource")
                && let Some(expr) = self.wave28b_other_cast_this_turn_amount(rest, false)
            {
                return Some(expr);
            }
            if let Some(rest) = def.strip_prefix("Count$ThisTurnCast_")
                && matches!(
                    self.trigger_mode.as_deref(),
                    Some("SpellCast" | "SpellCastOrCopy")
                )
                && let Some(valid) = rest
                    .strip_suffix("+ControlledBy TriggeredActivator")
                    .or_else(|| rest.strip_suffix(".ControlledBy TriggeredActivator"))
                && !valid.contains(['$', '/'])
            {
                let expr = self.filter_expr_in_zone(valid.trim(), true)?;
                let name = self.body.filter_static("CAST", &expr);
                return Some(format!(
                    "Amount::SpellsCastMatchingThisTurn {{ filter: &{name}, \
                     caster: PlayerRel::EventPlayer }}"
                ));
            }
            if def == "Count$ThisTurnEntered_Graveyard_from_Library_Card.OwnedBy Targeted" {
                return Some(
                    "Amount::EnteredThisTurn { filter: &Filter::Any, \
                     from: Some(ZoneRef::Library), to: ZoneRef::Graveyard, \
                     owner_or_ctrl: PlayerRel::Chosen, exclude_source: false }"
                        .to_string(),
                );
            }
            if matches!(
                def,
                "TriggeredSource$CardToughness" | "TriggeredSourceLKICopy$CardToughness"
            ) && self.damage_trigger_binds_source
            {
                return Some("Amount::EventLastToughness".to_string());
            }
            if matches!(
                def,
                "TriggeredSource$CardPower" | "TriggeredSourceLKICopy$CardPower"
            ) && self.damage_trigger_binds_source
            {
                return Some("Amount::EventPower".to_string());
            }
            if let Some(expr) = self.wave28b_two_arm_conditional_amount(def, depth) {
                return Some(expr);
            }
            if let Some(expr) = self.wave28b_scaled_conditional_amount(def, depth) {
                return Some(expr);
            }
            if let Some(expr) = self.wave28b_predicate_valid_amount(def) {
                return Some(expr);
            }
            if let Some(rest) = def.strip_prefix("Discarded$CardMulticolor.")
                && self.cost_discards_single_for_cost
                && let Some((then_raw, else_raw)) = rest.split_once('.')
                && !else_raw.contains('.')
            {
                let then_amt = self.wave28b_arm_amount(then_raw.trim(), depth + 1)?;
                let else_amt = self.wave28b_arm_amount(else_raw.trim(), depth + 1)?;
                return Some(format!(
                    "Amount::IfCondition {{ condition: &Condition::DiscardedForCost(&Filter::Multicolored), \
                     then: &{then_amt}, otherwise: &{else_amt} }}"
                ));
            }
        }
        None
    }

    /// Fallback for `effects::amount_or_count_ext` covering `AFLifeLost`,
    /// resolution-chosen X, discard-hand `Discarded$Amount`, and wave 28b
    /// expressions wrapped in arithmetic operators.
    pub(super) fn wave28b_amount_ext(&mut self, raw: &str, resolved: &str) -> Option<String> {
        let raw = raw.trim();
        let resolved = resolved.trim();
        if raw == "AFLifeLost"
            && self.chain_prev_lose_life
            && self.svars.get("AFLifeLost").map(|s| s.trim()) == Some("Number$0")
        {
            return Some("Amount::ThisWay(ThisWay::LifeLost)".to_string());
        }
        if resolved == "Count$xPaid" && self.in_resolution_chosen_x {
            return Some("Amount::ResolutionChosenNumber".to_string());
        }
        if resolved == "Discarded$Amount" && self.in_discard_hand_may {
            return Some("Amount::ThisWay(ThisWay::Discarded)".to_string());
        }
        if let Some(expr) = Self::wave28b_devotion_or_chroma(resolved) {
            return Some(expr);
        }
        if let Some(valid) = resolved.strip_prefix("Count$ValidExile ")
            && !valid.contains('/')
            && !valid.contains('$')
            && let Some(stripped) = Self::strip_your_graveyard_owner(valid.trim())
        {
            let inner = self.filter_expr_in_zone(&stripped, true)?;
            if inner != "Filter::Any" {
                let combined = format!("Filter::And(&[{inner}, Filter::OwnedByYou])");
                let name = self.body.filter_static("COUNT", &combined);
                return Some(format!(
                    "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::ExileAll }}"
                ));
            }
        }
        if let Some((base_raw, op_raw)) = resolved.split_once('/')
            && let Some(base) = Self::wave28b_devotion_or_chroma(base_raw.trim())
            && let Some(scaled) = Self::wave28b_apply_simple_op(&base, op_raw.trim())
        {
            return Some(scaled);
        }
        self.wave28b_count_expr(resolved, 0, false)
    }

    fn wave28b_apply_simple_op(base: &str, op: &str) -> Option<String> {
        if op == "Twice" {
            return Some(format!("Amount::Times {{ base: &{base}, factor: 2 }}"));
        }
        if op == "Thrice" {
            return Some(format!("Amount::Times {{ base: &{base}, factor: 3 }}"));
        }
        if op == "HalfDown" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: false }}"));
        }
        if op == "HalfUp" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: true }}"));
        }
        if let Some(n) = op.strip_prefix("Plus.").and_then(|s| s.parse::<u32>().ok()) {
            return Some(format!("Amount::Plus {{ base: &{base}, offset: {n} }}"));
        }
        if let Some(n) = op
            .strip_prefix("Minus.")
            .and_then(|s| s.parse::<u32>().ok())
        {
            return Some(format!(
                "Amount::SaturatingSub {{ base: &{base}, subtract: {n} }}"
            ));
        }
        if let Some(n) = op
            .strip_prefix("Times.")
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&v| v >= 1)
        {
            return Some(format!("Amount::Times {{ base: &{base}, factor: {n} }}"));
        }
        None
    }

    fn wave28b_devotion_or_chroma(def: &str) -> Option<String> {
        if let Some(rest) = def
            .strip_prefix("Count$Devotion.")
            .or_else(|| def.strip_prefix("Count$DevotionDual."))
        {
            let mut colors = Vec::new();
            for part in rest.split('.') {
                let color = Self::wave28b_color_variant(part.trim())?;
                if !colors.contains(&color) {
                    colors.push(color);
                }
            }
            if colors.is_empty() {
                return None;
            }
            return Some(format!(
                "Amount::Devotion {{ who: PlayerRel::You, colors: ColorSet::from_slice(&[{}]) }}",
                colors.join(", ")
            ));
        }
        if let Some(rest) = def.strip_prefix("Count$Chroma.") {
            let color = Self::wave28b_color_variant(rest.trim())?;
            return Some(format!(
                "Amount::Devotion {{ who: PlayerRel::You, colors: ColorSet::from_slice(&[{color}]) }}"
            ));
        }
        None
    }

    fn wave28b_color_variant(word: &str) -> Option<&'static str> {
        Some(match word {
            "White" => "Color::White",
            "Blue" => "Color::Blue",
            "Black" => "Color::Black",
            "Red" => "Color::Red",
            "Green" => "Color::Green",
            _ => return None,
        })
    }

    fn wave28b_two_arm_condition(&self, word: &str) -> Option<&'static str> {
        let spell_or_etb = !self.in_delayed && (self.on_a_spell || self.is_etb_self_trigger);
        Some(match word {
            "Morbid" => "Condition::CreaturesDiedThisTurnAtLeast(1)",
            "Hellbent" => "Condition::HandSizeAtMost(0)",
            "Threshold" => "Condition::GraveyardCountAtLeast(7)",
            "Delirium" => "Condition::GraveyardTypesAtLeast(4)",
            "Metalcraft" => "Condition::ControlCount(&Filter::ARTIFACT, 3)",
            "Blessing" => "Condition::CitysBlessing",
            "Landfall" => {
                "Condition::AmountCompare { left: &Amount::EnteredThisTurn { \
                 filter: &Filter::LAND, from: None, to: ZoneRef::Battlefield, \
                 owner_or_ctrl: PlayerRel::You, exclude_source: false }, \
                 op: AmountCmp::AtLeast, right: &Amount::Fixed(1) }"
            }
            "Bargained" if spell_or_etb && self.body.bargain => "Condition::Bargained",
            "Prototyped" if spell_or_etb => "Condition::Prototyped",
            "Escaped" if spell_or_etb => "Condition::Escaped",
            "Monarch" => "Condition::IsMonarch(PlayerRel::You)",
            "Initiative" => "Condition::HasInitiative(PlayerRel::You)",
            "CompletedDungeon" => "Condition::CompletedADungeon(PlayerRel::You)",
            _ => return None,
        })
    }

    fn wave28b_two_arm_conditional_amount(&mut self, def: &str, depth: u8) -> Option<String> {
        let rest = def.strip_prefix("Count$")?;
        let (word, arms) = rest.split_once('.')?;
        let cond = self.wave28b_two_arm_condition(word)?;
        let (then_raw, else_raw) = arms.split_once('.')?;
        if else_raw.contains('.') {
            return None;
        }
        let then_amt = self.wave28b_arm_amount(then_raw.trim(), depth + 1)?;
        let else_amt = self.wave28b_arm_amount(else_raw.trim(), depth + 1)?;
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond}, then: &{then_amt}, otherwise: &{else_amt} }}"
        ))
    }

    fn wave28b_scaled_conditional_amount(&mut self, def: &str, depth: u8) -> Option<String> {
        let (lhs, rhs) = def.split_once("/Times.")?;
        let rhs_def = self.svars.get(rhs.trim())?.trim().to_string();
        let rest = rhs_def.strip_prefix("Count$")?;
        let (word, arms) = rest.split_once('.')?;
        let cond = self.wave28b_two_arm_condition(word)?;
        let (then_raw, else_raw) = arms.split_once('.')?;
        if else_raw.contains('.') {
            return None;
        }
        let then_n: u32 = then_raw.trim().parse().ok()?;
        let else_n: u32 = else_raw.trim().parse().ok()?;
        let base_lhs = if let Some(var) = lhs.trim().strip_prefix("SVar$") {
            let inner = self.svars.get(var.trim())?.trim().to_string();
            self.wave28b_resolve_base(&inner, depth + 1)?
        } else {
            self.wave28b_resolve_base(lhs.trim(), depth + 1)?
        };
        let then_amt = Self::wave28b_scale_amount(&base_lhs, then_n);
        let else_amt = Self::wave28b_scale_amount(&base_lhs, else_n);
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond}, then: &{then_amt}, otherwise: &{else_amt} }}"
        ))
    }

    fn wave28b_resolve_base(&mut self, expr: &str, depth: u8) -> Option<String> {
        if expr == "Count$xPaid" {
            if self.in_resolution_chosen_x {
                return Some("Amount::ResolutionChosenNumber".to_string());
            }
            if self.has_x || (self.is_etb_self_trigger && self.has_x_mana_cost && !self.in_delayed)
            {
                return Some("Amount::X".to_string());
            }
            return None;
        }
        self.count_expr_depth(expr, depth, false)
    }

    fn wave28b_scale_amount(base: &str, factor: u32) -> String {
        match factor {
            0 => "Amount::Fixed(0)".to_string(),
            1 => base.to_string(),
            2 if base == "Amount::X" => "Amount::DoubleX".to_string(),
            n => format!("Amount::Times {{ base: &{base}, factor: {n} }}"),
        }
    }

    fn wave28b_arm_amount(&mut self, raw: &str, depth: u8) -> Option<String> {
        if let Ok(n) = raw.parse::<u32>() {
            return Some(format!("Amount::Fixed({n})"));
        }
        let def = self.svars.get(raw)?.trim().to_string();
        let amt = self.wave28b_resolve_base(&def, depth)?;
        (!amt.contains("Neg")).then_some(amt)
    }

    /// Lowers single-object 0/1 predicate counts (`Targeted$Valid <F>`,
    /// `ParentTargeted$Valid <F>`, `TriggeredCard$Valid <F>`, `Discarded$Valid <F>`)
    /// with optional `/Plus.<N>` or `/Times.<N>` into `Amount::IfCondition`.
    fn wave28b_predicate_valid_amount(&mut self, def: &str) -> Option<String> {
        let (base_raw, op_raw) = def
            .split_once('/')
            .map_or((def, None), |(l, r)| (l.trim(), Some(r.trim())));
        let cond = self.wave28b_predicate_valid_condition(base_raw)?;
        let (then_n, else_n) = match op_raw {
            None => (1u32, 0u32),
            Some(op) => {
                if let Some(n) = op.strip_prefix("Plus.").and_then(|s| s.parse::<u32>().ok()) {
                    (n.checked_add(1)?, n)
                } else if let Some(n) = op
                    .strip_prefix("Times.")
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    (n, 0)
                } else if op == "Twice" {
                    (2, 0)
                } else {
                    return None;
                }
            }
        };
        Some(format!(
            "Amount::IfCondition {{ condition: &{cond}, \
             then: &Amount::Fixed({then_n}), otherwise: &Amount::Fixed({else_n}) }}"
        ))
    }

    fn wave28b_predicate_valid_condition(&mut self, base_raw: &str) -> Option<String> {
        if let Some(valid) = base_raw
            .strip_prefix("Targeted$Valid ")
            .or_else(|| base_raw.strip_prefix("ParentTargeted$Valid "))
            && self.single_object_target
            && !valid.contains(['$', '/'])
        {
            let expr = self.filter_expr(valid.trim())?;
            let f_ref = self.wave28b_filter_ref(&expr);
            return Some(format!(
                "Condition::ObjectMatches {{ what: &TargetSpec::Object(&Filter::Any), filter: {f_ref} }}"
            ));
        }
        if let Some(valid) = base_raw
            .strip_prefix("TriggeredCard$Valid ")
            .or_else(|| base_raw.strip_prefix("TriggeredCardLKICopy$Valid "))
            && self.trigger_mode.as_deref() == Some("ChangesZone")
            && !valid.contains(['$', '/'])
        {
            let expr = self.filter_expr(valid.trim())?;
            let f_ref = self.wave28b_filter_ref(&expr);
            return Some(format!(
                "Condition::ObjectMatches {{ what: &TargetSpec::EventObject, filter: {f_ref} }}"
            ));
        }
        if let Some(valid) = base_raw.strip_prefix("Discarded$Valid ")
            && self.cost_discards_single_for_cost
            && !valid.contains(['$', '/'])
        {
            let expr = self.filter_expr_in_zone(valid.trim(), true)?;
            let f_ref = self.wave28b_filter_ref(&expr);
            return Some(format!("Condition::DiscardedForCost({f_ref})"));
        }
        if let Some(valid) = base_raw.strip_prefix("Sacrificed$Valid ")
            && self.cost_sacrifices_other
            && !valid.contains(['$', '/'])
        {
            let expr = self.filter_expr(valid.trim())?;
            let f_ref = self.wave28b_filter_ref(&expr);
            return Some(format!("Condition::SacrificedForCost({f_ref})"));
        }
        None
    }

    fn wave28b_filter_uses_resolution_state(expr: &str) -> bool {
        expr.contains("PlayerRel::Chosen")
            || expr.contains("PlayerRel::EventPlayer")
            || expr.contains("PlayerRel::ControllerOfTarget")
            || expr.contains("PlayerRel::OwnerOfTarget")
            || expr.contains("PlayerRel::ControllerOfEvent")
            || expr.contains("PlayerRel::DefendingPlayer")
            || expr.contains("Filter::Reference")
            || expr.contains("Filter::IsRemembered")
    }

    fn wave28b_is_single_player_rel(rel: &str) -> bool {
        matches!(
            rel,
            "PlayerRel::You"
                | "PlayerRel::Chosen"
                | "PlayerRel::EventPlayer"
                | "PlayerRel::ActivePlayer"
                | "PlayerRel::ControllerOfTarget"
                | "PlayerRel::OwnerOfTarget"
                | "PlayerRel::ControllerOfEvent"
                | "PlayerRel::OwnerOfEvent"
                | "PlayerRel::DefendingPlayer"
                | "PlayerRel::ControllerOfAttached"
                | "PlayerRel::EnchantedPlayer"
                | "PlayerRel::OwnerOfSource"
                | "PlayerRel::GiftRecipient"
        )
    }

    /// Lowers unfiltered `Count$ThisTurnCast_Card.YouCtrl+!CastSaSource` (or
    /// `Card.!CastSaSource+YouCtrl`) on a resolving spell or ETB into
    /// `SpellsCastMatchingThisTurn(Any, You)` minus 1 iff `SourceWasCastByYou`.
    /// Filtered `!CastSaSource` expressions (e.g. `Card.Red+!CastSaSource+YouCtrl`)
    /// stay refused because cast-time characteristics can differ from resolution-time
    /// characteristics (and `wave27a` asserts that refusal boundary).
    fn wave28b_other_cast_this_turn_amount(
        &self,
        rest: &str,
        in_etb_modifier: bool,
    ) -> Option<String> {
        if self.in_delayed || (!self.on_a_spell && !self.is_etb_self_trigger && !in_etb_modifier) {
            return None;
        }
        if !matches!(
            rest.trim(),
            "Card.YouCtrl+!CastSaSource" | "Card.!CastSaSource+YouCtrl"
        ) {
            return None;
        }
        let raw =
            "Amount::SpellsCastMatchingThisTurn { filter: &Filter::Any, caster: PlayerRel::You }";
        Some(format!(
            "Amount::IfCondition {{ condition: &Condition::SourceWasCastByYou, \
             then: &Amount::SaturatingSub {{ base: &{raw}, subtract: 1 }}, \
             otherwise: &{raw} }}"
        ))
    }

    /// Additional `etbCounter` `CheckSVar$` conditions backed by existing DSL
    /// `Condition` variants (e.g. *Hotheaded Giant* `!CastSaSource`, opponent
    /// control count comparisons).
    pub(super) fn wave28b_etb_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let def = self
            .svars
            .get(var.trim())
            .map_or(var, String::as_str)
            .trim()
            .to_string();
        let cmp = cmp.trim();
        if let Some(rest) = def.strip_prefix("Count$ThisTurnCast_")
            && rest.contains("!CastSaSource")
            && let Some(amt) = self.wave28b_other_cast_this_turn_amount(rest, true)
        {
            return Self::wave28b_amount_cmp_num(&amt, cmp);
        }
        if def == "PlayerCountPlayers$HighestValidGraveyard Card.YouOwn" {
            return Self::wave28b_any_graveyard_count_cond(cmp);
        }
        self.wave28b_opponent_controls_more_cond(&def, cmp)
    }

    fn wave28b_amount_cmp_num(amt: &str, cmp: &str) -> Option<String> {
        let (op_code, rhs_raw) = cmp.split_at_checked(2)?;
        let op = match op_code {
            "GE" => "AmountCmp::AtLeast",
            "GT" => "AmountCmp::GreaterThan",
            "LE" => "AmountCmp::AtMost",
            "LT" => "AmountCmp::LessThan",
            "EQ" => "AmountCmp::Equal",
            "NE" => "AmountCmp::NotEqual",
            _ => return None,
        };
        let n: u32 = rhs_raw.trim().parse().ok()?;
        Some(format!(
            "Condition::AmountCompare {{ left: &{amt}, op: {op}, right: &Amount::Fixed({n}) }}"
        ))
    }

    fn wave28b_opponent_controls_more_cond(&mut self, raw: &str, cmp: &str) -> Option<String> {
        let opp_valid = raw
            .strip_prefix("PlayerCountOpponents$HighestValid ")?
            .trim();
        let rhs_var = cmp.strip_prefix("GT")?.trim();
        let rhs_def = self.svars.get(rhs_var)?.trim().to_string();
        let you_valid = rhs_def.strip_prefix("Count$Valid ")?.trim();
        if opp_valid != you_valid || opp_valid.contains(['$', '/', ',']) {
            return None;
        }
        let (base, quals) = opp_valid.split_once('.')?;
        let mut saw_you_ctrl = false;
        let mut kept = Vec::new();
        for atom in quals.split('+').map(str::trim) {
            if atom == "YouCtrl" {
                saw_you_ctrl = true;
            } else if matches!(atom, "OppCtrl" | "YouDontCtrl") || atom.starts_with("ControlledBy")
            {
                return None;
            } else if !atom.is_empty() {
                kept.push(atom);
            }
        }
        if !saw_you_ctrl {
            return None;
        }
        let stripped = if kept.is_empty() {
            base.trim().to_string()
        } else {
            format!("{}.{}", base.trim(), kept.join("+"))
        };
        let expr = self.filter_expr(&stripped)?;
        if Self::wave28b_filter_uses_resolution_state(&expr) {
            return None;
        }
        let f_ref = self.wave28b_filter_ref(&expr);
        Some(format!("Condition::OpponentControlsMore({f_ref})"))
    }

    fn wave28b_filter_ref(&mut self, expr: &str) -> String {
        match expr {
            "Filter::Any"
            | "Filter::CREATURE"
            | "Filter::ARTIFACT"
            | "Filter::ENCHANTMENT"
            | "Filter::LAND"
            | "Filter::NONLAND"
            | "Filter::NONCREATURE"
            | "Filter::INSTANT"
            | "Filter::SORCERY"
            | "Filter::PLANESWALKER"
            | "Filter::YOUR_CREATURE"
            | "Filter::YOUR_ARTIFACT"
            | "Filter::YOUR_LAND"
            | "Filter::OPPONENT_CREATURE"
            | "Filter::Multicolored" => format!("&{expr}"),
            _ => format!("&{}", self.body.filter_static("COND", expr)),
        }
    }

    /// Additional `Count$Compare` / `ConditionCheckSVar$` conditions.
    pub(super) fn wave28b_compare_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let raw = self.resolve_svar_or_inline(var)?;
        let raw = raw.trim();
        let cmp = cmp.trim();
        if raw == "PlayerCountPlayers$HighestValidGraveyard Card.YouOwn" {
            return Self::wave28b_any_graveyard_count_cond(cmp);
        }
        if let Some(cond) = self.wave28b_opponent_controls_more_cond(raw, cmp) {
            return Some(cond);
        }
        if let Some(base) = self.wave28b_predicate_valid_condition(raw) {
            return match cmp {
                "GE1" | "EQ1" | "GT0" | "NE0" => Some(base),
                "EQ0" | "LE0" | "LT1" => Some(format!("Condition::Not(&{base})")),
                _ => None,
            };
        }
        if let Some(cond) = self.wave28b_product_nonzero_condition(raw, cmp, 0) {
            return Some(cond);
        }
        None
    }

    fn wave28b_any_graveyard_count_cond(cmp: &str) -> Option<String> {
        let (op, rhs) = cmp.split_at_checked(2)?;
        let n: u16 = rhs.trim().parse().ok()?;
        match op {
            "GE" if n >= 1 => Some(format!("Condition::AnyGraveyardCountAtLeast({n})")),
            "GT" => n
                .checked_add(1)
                .map(|k| format!("Condition::AnyGraveyardCountAtLeast({k})")),
            "LT" if n >= 1 => Some(format!(
                "Condition::Not(&Condition::AnyGraveyardCountAtLeast({n}))"
            )),
            "LE" => n
                .checked_add(1)
                .map(|k| format!("Condition::Not(&Condition::AnyGraveyardCountAtLeast({k}))")),
            _ => None,
        }
    }

    /// Lowers `SVar$<A>/Times.<B>` compared against 0 (positive vs zero),
    /// supporting nested products (e.g. *Scepter of Empires*, *Mine Worker*).
    fn wave28b_product_nonzero_condition(
        &mut self,
        raw: &str,
        cmp: &str,
        depth: u8,
    ) -> Option<String> {
        if depth >= 4 || !raw.contains("/Times.") {
            return None;
        }
        let positive = match cmp {
            "GE1" | "GT0" | "NE0" => true,
            "EQ0" | "LE0" | "LT1" => false,
            _ => return None,
        };
        let mut factors = Vec::new();
        self.wave28b_collect_product_factors(raw, depth, &mut factors)?;
        if factors.len() < 2 {
            return None;
        }
        if positive {
            let clauses: Vec<String> = factors
                .into_iter()
                .map(|amt| {
                    format!(
                        "Condition::AmountCompare {{ left: &{amt}, op: AmountCmp::AtLeast, \
                         right: &Amount::Fixed(1) }}"
                    )
                })
                .collect();
            Some(format!("Condition::All(&[{}])", clauses.join(", ")))
        } else {
            let clauses: Vec<String> = factors
                .into_iter()
                .map(|amt| {
                    format!(
                        "Condition::AmountCompare {{ left: &{amt}, op: AmountCmp::Equal, \
                         right: &Amount::Fixed(0) }}"
                    )
                })
                .collect();
            Some(format!("Condition::Any(&[{}])", clauses.join(", ")))
        }
    }

    fn wave28b_collect_product_factors(
        &mut self,
        raw: &str,
        depth: u8,
        out: &mut Vec<String>,
    ) -> Option<()> {
        if depth >= 4 {
            return None;
        }
        let trimmed = raw.trim();
        if let Some((lhs, rhs)) = trimmed.split_once("/Times.") {
            let lhs_resolved = if let Some(var) = lhs.trim().strip_prefix("SVar$") {
                self.svars.get(var.trim())?.trim().to_string()
            } else {
                lhs.trim().to_string()
            };
            self.wave28b_collect_product_factors(&lhs_resolved, depth + 1, out)?;
            let rhs_resolved = self
                .svars
                .get(rhs.trim())
                .map_or_else(|| rhs.trim().to_string(), |s| s.trim().to_string());
            self.wave28b_collect_product_factors(&rhs_resolved, depth + 1, out)?;
            return Some(());
        }
        let amt = self.static_count_expr(trimmed)?;
        if amt.contains("Neg") || amt.starts_with("Amount::Fixed(") {
            return None;
        }
        out.push(amt);
        Some(())
    }

    /// Resolution-line `ConditionCheckSVar$` wrapper for `Count$ResolvedThisTurn`
    /// (`Effect::IfResolvedTimesThisTurn`).
    pub(super) fn wave28b_check_svar_wrap(
        &self,
        api: &str,
        p: &mut Params,
        effects: &[String],
    ) -> Option<Vec<String>> {
        let var = p.peek("ConditionCheckSVar")?.trim();
        let cmp = p.peek("ConditionSVarCompare")?.trim();
        let def = self.svars.get(var).map_or(var, String::as_str).trim();
        if def != "Count$ResolvedThisTurn" {
            return None;
        }
        let times: u32 = cmp.strip_prefix("EQ")?.parse().ok().filter(|&n| n >= 1)?;
        let saved = p.clone();
        p.take("ConditionCheckSVar");
        p.take("ConditionSVarCompare");
        p.take("ConditionDescription");
        if !p.exhausted() {
            *p = saved;
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{api}.{key}`"));
            }
            return None;
        }
        Some(vec![format!(
            "Effect::IfResolvedTimesThisTurn {{ times: {times}, then: &[{}] }}",
            effects.join(", ")
        )])
    }

    /// Inline `ConditionCheckSVar$` expressions that are not `SVar` names (e.g.
    /// `TriggeredSpellAbility$CardPower` on `SpellCast`, `TriggeredSpellAbility$CardManaCostLKI`,
    /// `TriggeredAttacker$CardPower`), plus Wave 28b compare conditions.
    pub(super) fn wave28b_inline_svar_condition(&mut self, var: &str, cmp: &str) -> Option<String> {
        let raw = self
            .svars
            .get(var.trim())
            .map_or(var, String::as_str)
            .trim()
            .to_string();
        let raw = raw.as_str();
        let cmp = cmp.trim();
        if matches!(
            raw,
            "TriggeredSpellAbility$CardPower" | "TriggeredCard$CardPower"
        ) && matches!(
            self.trigger_mode.as_deref(),
            Some("SpellCast" | "SpellCastOrCopy")
        ) {
            return self.wave28b_event_power_condition(cmp);
        }
        if matches!(
            raw,
            "TriggeredAttacker$CardPower" | "TriggeredAttackerLKICopy$CardPower"
        ) && matches!(
            self.trigger_mode.as_deref(),
            Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
        ) {
            return self.wave28b_event_power_condition(cmp);
        }
        if matches!(
            raw,
            "TriggeredSpellAbility$CardManaCostLKI"
                | "TriggeredSpellAbility$CardManaCost"
                | "TriggeredCardLKI$CardManaCost"
                | "TriggeredCard$CardManaCost"
        ) && matches!(
            self.trigger_mode.as_deref(),
            Some("SpellCast" | "SpellCastOrCopy")
        ) {
            let (op_code, rhs_raw) = cmp.split_at_checked(2)?;
            let op = match op_code {
                "GE" => "AmountCmp::AtLeast",
                "GT" => "AmountCmp::GreaterThan",
                "LE" => "AmountCmp::AtMost",
                "LT" => "AmountCmp::LessThan",
                "EQ" => "AmountCmp::Equal",
                "NE" => "AmountCmp::NotEqual",
                _ => return None,
            };
            let n: u32 = rhs_raw.trim().parse().ok()?;
            return Some(format!(
                "Condition::AmountCompare {{ left: &Amount::EventManaValue, op: {op}, \
                 right: &Amount::Fixed({n}) }}"
            ));
        }
        self.wave28b_compare_condition(var, cmp)
    }

    fn wave28b_event_power_condition(&mut self, cmp: &str) -> Option<String> {
        let (op_code, rhs_raw) = cmp.split_at_checked(2)?;
        let n: i32 = rhs_raw.trim().parse().ok()?;
        let filter_expr = match op_code {
            "GE" => format!("Filter::PowerAtLeast({n})"),
            "GT" => format!("Filter::PowerAtLeast({})", n.checked_add(1)?),
            "LE" => format!("Filter::PowerAtMost({n})"),
            "LT" => format!("Filter::PowerAtMost({})", n.checked_sub(1)?),
            "EQ" => format!("Filter::And(&[Filter::PowerAtLeast({n}), Filter::PowerAtMost({n})])"),
            _ => return None,
        };
        let f_ref = format!("&{}", self.body.filter_static("COND", &filter_expr));
        Some(format!(
            "Condition::ObjectMatches {{ what: &TargetSpec::EventObject, filter: {f_ref} }}"
        ))
    }

    /// Combines an already-parsed `ConditionPresent$` clause with a co-occurring
    /// `ConditionCheckSVar$` on the same resolution line (*Thickest in the Thicket*,
    /// *Triumph of Ferocity*).
    pub(super) fn wave28b_combine_present_and_svar_cond(
        &mut self,
        p: &mut Params,
        present_cond: String,
    ) -> Option<String> {
        let Some(var) = p.peek("ConditionCheckSVar").map(str::to_string) else {
            return Some(present_cond);
        };
        let cmp = p
            .peek("ConditionSVarCompare")
            .map_or_else(|| "GE1".to_string(), str::to_string);
        let svar_cond = self
            .svar_condition(&var, &cmp)
            .or_else(|| self.extra_svar_condition_clause(&var, &cmp))
            .or_else(|| self.wave7_svar_condition(&var, &cmp))
            .or_else(|| self.wave11_amount_compare_condition(&var, &cmp, true))
            .or_else(|| self.wave28b_inline_svar_condition(&var, &cmp))?;
        p.take("ConditionCheckSVar");
        p.take("ConditionSVarCompare");
        Some(format!("Condition::All(&[{present_cond}, {svar_cond}])"))
    }

    /// Additional `Defined$` player references backed by existing `PlayerRel` variants.
    pub(super) fn wave28b_player_of_line(
        &self,
        defined: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<&'static str> {
        let is_non_player_target = target.is_some_and(|t| {
            t.starts_with("TargetSpec::Object(")
                || t.starts_with("TargetSpec::AbilityOnStack(")
                || t.starts_with("TargetSpec::SpellOrAbility(")
                || matches!(
                    t,
                    "TargetSpec::CreatureOrPlaneswalker" | "TargetSpec::SpellOrAbility"
                )
        });
        match defined? {
            "TargetedOwner" | "ParentTargetOwner" | "ParentTargetedOwner"
                if is_non_player_target =>
            {
                Some("PlayerRel::OwnerOfTarget")
            }
            "ParentTargetedController" if is_non_player_target => {
                Some("PlayerRel::ControllerOfTarget")
            }
            "TargetedController"
                if target.is_some_and(|t| {
                    t == "TargetSpec::SpellOrAbility"
                        || t.starts_with("TargetSpec::SpellOrAbility(")
                        || t.starts_with("TargetSpec::AbilityOnStack(")
                }) =>
            {
                Some("PlayerRel::ControllerOfTarget")
            }
            "TriggeredActivator" | "TriggeredActivatingPlayer"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("SpellCast" | "SpellCastOrCopy" | "AbilityCast" | "TapsForMana")
                ) =>
            {
                Some("PlayerRel::EventPlayer")
            }
            "TriggeredBlockerController"
                if self.trigger_mode.as_deref() == Some("Blocks")
                    || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                        && self
                            .block_line
                            .as_ref()
                            .is_some_and(|(r, _, _)| *r == BlockRole::Blocked)) =>
            {
                Some("PlayerRel::ControllerOfEvent")
            }
            "TriggeredCardOwner" if self.trigger_mode.as_deref() == Some("Drawn") => {
                Some("PlayerRel::EventPlayer")
            }
            "AttackingPlayer"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce" | "DamageDealtOnce")
                ) && self.damage_trigger_is_combat =>
            {
                Some("PlayerRel::ActivePlayer")
            }
            "ThisTargetedPlayer"
                if targets_a_player
                    || target == Some("TargetSpec::Player(PlayerRel::Chosen)")
                    || matches!(
                        target,
                        Some(
                            "TargetSpec::AnyPlayer"
                                | "TargetSpec::AnyOpponent"
                                | "TargetSpec::PlayerWhoAttackedThisTurn"
                        )
                    ) =>
            {
                Some("PlayerRel::Chosen")
            }
            _ => None,
        }
    }

    /// `DealDamage | Defined$ TriggeredSourceLKICopy` on an attached-permanent
    /// damage trigger (*Guilty Conscience*) -> `Effect::DealDamageToAttached`
    /// (which resolves `source_attachment_lki` in `resolve/life.rs`).
    pub(super) fn wave28b_deal_damage_special(
        &mut self,
        p: &mut Params,
        num_dmg: &str,
        damage_source: Option<&str>,
        no_prevention: bool,
        may: bool,
    ) -> Option<Vec<String>> {
        if damage_source.is_some()
            || no_prevention
            || self.on_a_spell
            || !self.damage_trigger_source_is_attached
            || !matches!(
                p.peek("Defined"),
                Some("TriggeredSourceLKICopy" | "TriggeredSource")
            )
        {
            return None;
        }
        p.take("Defined");
        let n = self.amount_or_count_ext(num_dmg)?;
        let dmg = format!("Effect::DealDamageToAttached {{ amount: {n} }}");
        Some(if may {
            vec![format!("Effect::MayDo {{ effects: &[{dmg}] }}")]
        } else {
            vec![dmg]
        })
    }

    /// Additional `PutCounter | Defined$` values backed by existing DSL effects.
    pub(super) fn wave28b_put_counter_defined(
        &mut self,
        def: &str,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        single_target: bool,
        specs: &[(String, String)],
    ) -> Option<Vec<String>> {
        let emit_filter = |filter: &str| -> Vec<String> {
            specs
                .iter()
                .map(|(kind, n)| {
                    format!(
                        "Effect::AddCounterFilter {{ filter: {filter}, kind: {kind}, amount: {n} }}"
                    )
                })
                .collect()
        };
        let emit_target = || -> Vec<String> {
            specs
                .iter()
                .map(|(kind, n)| format!("Effect::AddCounter {{ kind: {kind}, amount: {n} }}"))
                .collect()
        };
        match def {
            "TriggeredSourceLKICopy" | "TriggeredSource"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce" | "DamageDealtOnce")
                ) && target.is_none() =>
            {
                if self.trigger_is_self {
                    Some(emit_filter("&Filter::This"))
                } else if self.damage_trigger_binds_source {
                    Some(emit_filter("&Filter::Reference"))
                } else {
                    None
                }
            }
            "TriggeredCardLKICopy" | "TriggeredCard"
                if self.trigger_mode.as_deref() == Some("Mutates")
                    && self.trigger_is_self
                    && target.is_none() =>
            {
                Some(emit_filter("&Filter::This"))
            }
            "TargetedCard" if !targets_here && target.is_some() && !targets_a_player => {
                Some(emit_target())
            }
            other
                if !targets_here
                    && single_target
                    && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
                    && (other.starts_with("Targeted.") || other.starts_with("ParentTarget.")) =>
            {
                let qual = other
                    .strip_prefix("Targeted.")
                    .or_else(|| other.strip_prefix("ParentTarget."))?;
                let q = self.filter_expr(&format!("Card.{qual}"))?;
                let name = self.body.filter_static("COUNTER", &q);
                let inner = emit_target().join(", ");
                Some(vec![format!(
                    "Effect::IfTargetMatches {{ filter: &{name}, then: &[{inner}] }}"
                )])
            }
            _ => None,
        }
    }

    /// Additional `Draw` shapes: `Upto$ True` for you (`ChooseNumberEx`) and
    /// `OptionalDecider$ True / <single-player>` (`MayDo` / `MayDoFor`).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave28b_draw_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("Upto") == Some("True") {
            if p.has("RememberDrawn") || p.has("OptionalDecider") {
                return None;
            }
            let saved = p.clone();
            let saved_statics = self.body.statics.len();
            p.take("Upto");
            let defined = p.take("Defined");
            let Some(max) = self.amount_or_count_ext(p.take("NumCards").as_deref().unwrap_or("1"))
            else {
                *p = saved;
                self.body.statics.truncate(saved_statics);
                return None;
            };
            if !targets_a_player && matches!(defined.as_deref(), None | Some("You")) {
                return Some(vec![format!(
                    "Effect::ChooseNumberEx {{ chooser: PlayerRel::You, min: Amount::Fixed(0), \
                     max: {max}, prompt: NumberChoiceKind::Choice, pay_cost: None, \
                     then: &[Effect::DrawCards {{ amount: Amount::ResolutionChosenNumber }}] }}"
                )]);
            }
            if !targets_a_player
                && let Some(domain) = match defined.as_deref() {
                    Some("Player") => Some("RepeatDomain::EachPlayer"),
                    Some("Opponent" | "Player.Opponent") => Some("RepeatDomain::EachOpponent"),
                    _ => None,
                }
            {
                return Some(vec![format!(
                    "Effect::RepeatEach {{ domain: {domain}, effects: &[Effect::ChooseNumberEx {{ \
                     chooser: PlayerRel::EventPlayer, min: Amount::Fixed(0), max: {max}, \
                     prompt: NumberChoiceKind::Choice, pay_cost: None, \
                     then: &[Effect::DrawCardsFor {{ amount: Amount::ResolutionChosenNumber, \
                     who: PlayerRel::EventPlayer }}] }}] }}"
                )]);
            }
            if let Some(who) =
                self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)
                && Self::wave28b_is_single_player_rel(who)
            {
                let draw = if who == "PlayerRel::You" {
                    "Effect::DrawCards { amount: Amount::ResolutionChosenNumber }".to_string()
                } else {
                    format!(
                        "Effect::DrawCardsFor {{ amount: Amount::ResolutionChosenNumber, who: {who} }}"
                    )
                };
                return Some(vec![format!(
                    "Effect::ChooseNumberEx {{ chooser: {who}, min: Amount::Fixed(0), \
                     max: {max}, prompt: NumberChoiceKind::Choice, pay_cost: None, \
                     then: &[{draw}] }}"
                )]);
            }
            *p = saved;
            self.body.statics.truncate(saved_statics);
            return None;
        }
        let decider = p.peek("OptionalDecider")?;
        if decider == "You" {
            return None;
        }
        let saved = p.clone();
        let saved_statics = self.body.statics.len();
        let decider = p.take("OptionalDecider").expect("checked above");
        let defined = p.take("Defined");
        let Some(who) = self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)
        else {
            *p = saved;
            self.body.statics.truncate(saved_statics);
            return None;
        };
        let decider_rel = if decider == "True" {
            (who == "PlayerRel::You").then_some("PlayerRel::You")
        } else if targets_a_player
            && defined.is_none()
            && who == "PlayerRel::Chosen"
            && matches!(decider.as_str(), "Opponent" | "Player" | "TargetedPlayer")
        {
            Some("PlayerRel::Chosen")
        } else {
            self.wave11_player_of_line(Some(decider.as_str()), target, targets_a_player)
                .filter(|rel| *rel == who && Self::wave28b_is_single_player_rel(rel))
        };
        let Some(decider_rel) = decider_rel else {
            *p = saved;
            self.body.statics.truncate(saved_statics);
            return None;
        };
        let Some(n) = self.amount_or_count_ext(p.take("NumCards").as_deref().unwrap_or("1")) else {
            *p = saved;
            self.body.statics.truncate(saved_statics);
            return None;
        };
        let draw = match (
            who,
            n.strip_prefix("Amount::Fixed(")
                .and_then(|r| r.strip_suffix(')')),
        ) {
            ("PlayerRel::You", Some(fixed)) => format!("Effect::draw({fixed})"),
            ("PlayerRel::You", None) => format!("Effect::DrawCards {{ amount: {n} }}"),
            (player, _) => format!("Effect::DrawCardsFor {{ amount: {n}, who: {player} }}"),
        };
        let wrapped = if decider_rel == "PlayerRel::You" {
            format!("Effect::MayDo {{ effects: &[{draw}] }}")
        } else {
            format!("Effect::MayDoFor {{ player: {decider_rel}, effects: &[{draw}] }}")
        };
        Some(vec![wrapped])
    }

    /// Chain-level Wave 28b handlers:
    /// 1. `DealDamage | Defined$ You | OptionalDecider$ You | RememberDamaged$ True | SubAbility$ DBImmediateTrig`
    ///    (*Elektra, Femme Fatale*) -> `MayDo([DealDamage(You), ReflexiveTrigger(DamageDealtToYou)])`.
    /// 2. `Draw | RememberDrawn$ True | SubAbility$ DBDiscard(Card.IsRemembered)`
    ///    (*Casting of Bones*) -> `Effect::DrawThenDiscardDrawn`.
    /// 3. `PutCounter | Choices$ <F>` -> `Effect::ChooseCardEx` (wrapped in
    ///    `Effect::CaptureAffected` when an outer object target is active so
    ///    `Filter::IsRemembered` never collides with `res.targets`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn wave28b_chain_step(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        if api == "AnimateAll" && self.on_a_spell && p.has("Abilities") && !p.has("Defined") {
            let saved_p = p.clone();
            let saved_statics = self.body.statics.len();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            self.on_a_spell = false;
            let res = self.animate_all_effect(p, target);
            self.on_a_spell = true;
            if res.is_some() {
                return res;
            }
            *p = saved_p;
            self.body.statics.truncate(saved_statics);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return None;
        }
        if api == "Animate"
            && self.on_a_spell
            && single_target
            && p.has("Abilities")
            && target.is_some_and(|t| !t.starts_with("TargetSpec::Player("))
            && matches!(
                p.peek("Defined"),
                None | Some("Targeted" | "ParentTarget" | "ParentTargeted")
            )
        {
            let saved_p = p.clone();
            let saved_statics = self.body.statics.len();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            self.on_a_spell = false;
            let res = self.animate_effect(p, target);
            self.on_a_spell = true;
            if res.is_some() {
                return res;
            }
            *p = saved_p;
            self.body.statics.truncate(saved_statics);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            return None;
        }
        if api == "PutCounterAll"
            && (p.has("ValidCardsDesc")
                || p.has("ValidCards2")
                || p.has("CounterType2")
                || p.peek("ValidCards")
                    .is_some_and(|v| v.contains("targetedBy")))
            && let Some(fx) = self.wave28b_put_counter_all(p, target, targets_here)
        {
            return Some(fx);
        }
        if p.has("ConditionDefined")
            && (p.has("ConditionPresent") || p.has("ConditionNotPresent"))
            && !p.has("PresentPlayer")
            && matches!(p.peek("PresentZone"), None | Some("Battlefield"))
            && let Some(fx) = self.wave28b_condition_defined_step(
                api,
                p,
                target,
                targets_here,
                targets_a_player,
                single_target,
            )
        {
            return Some(fx);
        }
        if api == "DealDamage"
            && p.peek("RememberDamaged") == Some("True")
            && p.peek("Defined") == Some("You")
            && p.peek("OptionalDecider") == Some("You")
            && self.sub_is_api(sub.as_deref(), "ImmediateTrigger")
        {
            return self.wave28b_may_damage_self_reflexive(p, sub);
        }
        if api == "Draw" && matches!(p.peek("RememberDrawn"), Some("True" | "AllReplaced")) {
            if p.peek("RememberDrawn") == Some("True")
                && !targets_a_player
                && matches!(p.peek("Defined"), None | Some("You"))
                && self.sub_is_api(sub.as_deref(), "Discard")
                && let Some(fx) = self.wave28b_draw_then_discard_drawn(p, sub)
            {
                return Some(fx);
            }
            if let Some(fx) = self.wave28b_draw_remembered_capture(p, sub, target, targets_a_player)
            {
                return Some(fx);
            }
            return None;
        }
        if api == "PutCounter" && p.has("Choices") {
            return self.wave28b_put_counter_choices(
                p,
                sub,
                target,
                targets_here,
                targets_a_player,
                single_target,
            );
        }
        None
    }

    /// Extends `PutCounterAll` with `ValidCardsDesc$`, secondary `ValidCards2$`
    /// / `CounterType2$` / `CounterNum2$` pairs (*Ajani Steadfast*, *Brokers
    /// Ascendancy*, *Dueling Coach*, …), and Renew's `ValidCards$ *.targetedBy`
    /// targeting pattern (*Kheru Goldkeeper*, *Sagu Pummeler*).
    #[allow(clippy::too_many_lines)]
    fn wave28b_put_counter_all(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here {
            return None;
        }
        let saved_p = p.clone();
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let fail = |this: &mut Self, p: &mut Params| {
            *p = saved_p;
            this.body.statics.truncate(saved_statics);
            *this.unclaimed.borrow_mut() = saved_unclaimed;
            None
        };
        let Some(valid1) = p.take("ValidCards") else {
            return fail(self, p);
        };
        p.take("ValidCardsDesc");
        p.take("ValidCards2Desc");
        p.take("IsCurse");
        let Some(code1) = p.take("CounterType") else {
            return fail(self, p);
        };
        let Some(kind1) = counter_kind(&code1) else {
            return fail(self, p);
        };
        let num1_raw = p.take("CounterNum");
        let Some(n1) = self.amount_or_count_ext(num1_raw.as_deref().unwrap_or("1")) else {
            return fail(self, p);
        };
        let valid2 = p.take("ValidCards2");
        let code2 = p.take("CounterType2");
        let num2_raw = p.take("CounterNum2");
        if valid2.is_some() && code2.is_none() {
            return fail(self, p);
        }
        if num2_raw.is_some() && code2.is_none() {
            return fail(self, p);
        }
        let second_spec = if let Some(c2) = code2 {
            let Some(kind2) = counter_kind(&c2) else {
                return fail(self, p);
            };
            let n2_str = match num2_raw.as_deref() {
                Some(raw) => raw,
                None if matches!(num1_raw.as_deref(), None | Some("1")) => "1",
                None => return fail(self, p),
            };
            let Some(n2) = self.amount_or_count_ext(n2_str) else {
                return fail(self, p);
            };
            Some((kind2, n2))
        } else {
            None
        };
        if valid1.contains("targetedBy") {
            if valid2.is_some() || !target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) {
                return fail(self, p);
            }
            let guard_creature = match valid1.trim() {
                "Card.targetedBy" | "Permanent.targetedBy" => false,
                "Creature.targetedBy" => !matches!(
                    target,
                    Some(
                        "TargetSpec::Object(&Filter::CREATURE)"
                            | "TargetSpec::Object(&Filter::YOUR_CREATURE)"
                            | "TargetSpec::Object(&Filter::OPPONENT_CREATURE)"
                    )
                ),
                _ => return fail(self, p),
            };
            let mut ops = vec![format!(
                "Effect::AddCounter {{ kind: {kind1}, amount: {n1} }}"
            )];
            if let Some((kind2, n2)) = second_spec {
                ops.push(format!(
                    "Effect::AddCounter {{ kind: {kind2}, amount: {n2} }}"
                ));
            }
            return Some(if guard_creature {
                vec![format!(
                    "Effect::IfTargetMatches {{ filter: &Filter::CREATURE, then: &[{}] }}",
                    ops.join(", ")
                )]
            } else {
                ops
            });
        }
        if target.is_some() {
            return fail(self, p);
        }
        let Some(expr1) = self.filter_expr(&valid1) else {
            return fail(self, p);
        };
        let filter1 = self.body.filter_static("COUNTERS", &expr1);
        let mut ops = vec![format!(
            "Effect::AddCounterFilter {{ filter: &{filter1}, kind: {kind1}, amount: {n1} }}"
        )];
        if let Some((kind2, n2)) = second_spec {
            let filter2_ref = if let Some(v2) = valid2 {
                let Some(expr2) = self.filter_expr(&v2) else {
                    return fail(self, p);
                };
                format!("&{}", self.body.filter_static("COUNTERS", &expr2))
            } else {
                format!("&{filter1}")
            };
            ops.push(format!(
                "Effect::AddCounterFilter {{ filter: {filter2_ref}, kind: {kind2}, amount: {n2} }}"
            ));
        }
        Some(ops)
    }

    /// Wraps an inner resolution step guarded by `ConditionDefined$` values
    /// that `chains.rs::wave11_object_matches_condition` does not cover:
    /// - `Equipped` / `Enchanted` (positive `ConditionPresent$` -> `Condition::AttachedMatches`)
    /// - `ThisTargetedCard` (when `targets_here && single_target` on an object target)
    /// - `Sacrificed` (when `self.cost_sacrifices_other` -> `Condition::SacrificedForCost`)
    /// - `Targeted` / `ParentTarget` on graveyard targets (`TargetSpec::CardInGraveyard*`)
    #[allow(clippy::too_many_lines)]
    fn wave28b_condition_defined_step(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        enum CondKind {
            Attached,
            ThisTargeted,
            Sacrificed,
            GraveyardTarget,
        }
        let defined = p.peek("ConditionDefined")?;
        let negated_key = if p.has("ConditionNotPresent") && !p.has("ConditionPresent") {
            true
        } else if p.has("ConditionPresent") && !p.has("ConditionNotPresent") {
            false
        } else {
            return None;
        };
        let key = if negated_key {
            "ConditionNotPresent"
        } else {
            "ConditionPresent"
        };
        let cmp = p
            .peek("PresentCompare")
            .or_else(|| p.peek("ConditionCompare"))
            .map(str::to_string);
        let positive = match (negated_key, cmp.as_deref()) {
            (false, None | Some("GE1" | "EQ1" | "GT0")) => true,
            (true, None) | (false, Some("EQ0" | "LT1")) => false,
            _ => return None,
        };
        let is_gy_target = target.is_some_and(|t| {
            t.starts_with("TargetSpec::CardInGraveyard(")
                || t.starts_with("TargetSpec::CardInYourGraveyard(")
                || t.starts_with("TargetSpec::CardInOpponentGraveyard(")
        });
        let kind = match defined {
            "Equipped" | "Enchanted" if !self.on_a_spell && positive => CondKind::Attached,
            "ThisTargetedCard"
                if targets_here
                    && single_target
                    && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
            {
                CondKind::ThisTargeted
            }
            "ThisTargetedCard" if targets_here && single_target && is_gy_target => {
                CondKind::GraveyardTarget
            }
            "Sacrificed" if self.cost_sacrifices_other => CondKind::Sacrificed,
            "Targeted" | "ParentTarget" if single_target && is_gy_target => {
                CondKind::GraveyardTarget
            }
            _ => return None,
        };
        let saved_p = p.clone();
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let fail = |this: &mut Self, p: &mut Params| {
            *p = saved_p;
            this.body.statics.truncate(saved_statics);
            *this.unclaimed.borrow_mut() = saved_unclaimed;
            None
        };
        let valid = p.take(key)?;
        p.take("ConditionDefined");
        p.take("PresentCompare");
        p.take("ConditionCompare");
        p.take("PresentZone");
        p.take("ConditionDescription");
        let Some(expr) = self.filter_expr(&valid) else {
            return fail(self, p);
        };
        if Self::wave28b_filter_uses_resolution_state(&expr)
            || (!self.on_a_spell && expr.contains("Amount::Sacrificed"))
        {
            return fail(self, p);
        }
        let f_ref = self.wave28b_filter_ref(&expr);
        let Some(inner) = self.effect_of(api, p, target, targets_here, targets_a_player) else {
            return fail(self, p);
        };
        if inner.is_empty() {
            return fail(self, p);
        }
        let inner_str = inner.join(", ");
        let wrapped = match kind {
            CondKind::Attached => format!(
                "Effect::IfCondition {{ condition: Condition::AttachedMatches({f_ref}), \
                 then: &[{inner_str}], otherwise: &[] }}"
            ),
            CondKind::ThisTargeted if positive => {
                format!("Effect::IfTargetMatches {{ filter: {f_ref}, then: &[{inner_str}] }}")
            }
            CondKind::ThisTargeted | CondKind::GraveyardTarget => {
                let base = format!(
                    "Condition::ObjectMatches {{ what: &TargetSpec::Object(&Filter::Any), \
                     filter: {f_ref} }}"
                );
                let cond = if positive {
                    base
                } else {
                    format!("Condition::Not(&{base})")
                };
                format!(
                    "Effect::IfCondition {{ condition: {cond}, then: &[{inner_str}], otherwise: &[] }}"
                )
            }
            CondKind::Sacrificed => {
                let base = format!("Condition::SacrificedForCost({f_ref})");
                let cond = if positive {
                    base
                } else {
                    format!("Condition::Not(&{base})")
                };
                format!(
                    "Effect::IfCondition {{ condition: {cond}, then: &[{inner_str}], otherwise: &[] }}"
                )
            }
        };
        Some(vec![wrapped])
    }

    /// Lowers `Draw | RememberDrawn$ AllReplaced|True` chains via
    /// `Effect::CaptureAffected` (since `draw_now` emits `Cause::Effect`
    /// `ZoneChanged` events, `Amount::RememberedCount` equals the exact number
    /// of cards drawn in `first`):
    /// - `Discard | NumCards$ Remembered$Amount` (*Ancient Excavation*)
    /// - `DealDamage | Defined$ <single-player> | NumDmg$ Remembered$Amount` (*Malignant Growth*)
    /// - Zero-drawn fallback (`ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0`) (*Trade Route Envoy*, *Mr. Foxglove*)
    #[allow(clippy::too_many_lines)]
    fn wave28b_draw_remembered_capture(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if self.capture_scope.is_some() || p.has("OptionalDecider") || p.has("Upto") {
            return None;
        }
        let saved_p = p.clone();
        let saved_sub = sub.clone();
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let fail = |this: &mut Self, p: &mut Params, sub: &mut Option<String>| {
            *p = saved_p;
            *sub = saved_sub;
            this.body.statics.truncate(saved_statics);
            *this.unclaimed.borrow_mut() = saved_unclaimed;
            None
        };
        let Some(sub_name) = sub.clone() else {
            return fail(self, p, sub);
        };
        let Some(sub_body) = self.svars.get(sub_name.trim()).cloned() else {
            return fail(self, p, sub);
        };
        let Some((sub_api, mut sp)) = Params::parse(&sub_body) else {
            return fail(self, p, sub);
        };
        sp.drop_prose();
        let defined = p.take("Defined");
        let Some(who) = self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)
        else {
            return fail(self, p, sub);
        };
        if !Self::wave28b_is_single_player_rel(who) {
            return fail(self, p, sub);
        }
        let draw_raw = p.take("NumCards").unwrap_or_else(|| "1".to_string());
        let Some(draw_amt) = self.amount_or_count_ext(&draw_raw) else {
            return fail(self, p, sub);
        };
        p.take("RememberDrawn");
        let draw_cond = if p.has("ConditionPresent") {
            if p.has("ConditionDefined")
                || p.has("PresentPlayer")
                || !matches!(p.peek("PresentZone"), None | Some("Battlefield"))
            {
                return fail(self, p, sub);
            }
            let valid = p.take("ConditionPresent")?;
            let cmp = p
                .take("PresentCompare")
                .or_else(|| p.take("ConditionCompare"));
            p.take("PresentZone");
            p.take("ConditionDescription");
            let positive = match cmp.as_deref() {
                None | Some("GE1" | "GT0") => true,
                Some("EQ0" | "LT1") => false,
                _ => return fail(self, p, sub),
            };
            let Some(expr) = self.filter_expr(&valid) else {
                return fail(self, p, sub);
            };
            let f_ref = self.wave28b_filter_ref(&expr);
            let op = if positive {
                "AmountCmp::AtLeast"
            } else {
                "AmountCmp::Equal"
            };
            let rhs = u32::from(positive);
            Some(format!(
                "Condition::AmountCompare {{ left: &Amount::CountOf {{ filter: {f_ref}, \
                 zone: ZoneSel::Battlefield }}, op: {op}, right: &Amount::Fixed({rhs}) }}"
            ))
        } else {
            None
        };
        if !p.exhausted() {
            return fail(self, p, sub);
        }
        let base_draw = match (
            who,
            draw_amt
                .strip_prefix("Amount::Fixed(")
                .and_then(|r| r.strip_suffix(')')),
        ) {
            ("PlayerRel::You", Some(fixed)) => format!("Effect::draw({fixed})"),
            ("PlayerRel::You", None) => format!("Effect::DrawCards {{ amount: {draw_amt} }}"),
            (player, _) => format!("Effect::DrawCardsFor {{ amount: {draw_amt}, who: {player} }}"),
        };
        let first_eff = if let Some(cond) = draw_cond {
            format!(
                "Effect::IfCondition {{ condition: {cond}, then: &[{base_draw}], otherwise: &[] }}"
            )
        } else {
            base_draw
        };
        let is_remembered_amount = |raw: &str| -> bool {
            let trimmed = raw.trim();
            trimmed == "Remembered$Amount"
                || self
                    .svars
                    .get(trimmed)
                    .is_some_and(|v| v.trim() == "Remembered$Amount")
        };
        let (then_eff, next_sub) = if sub_api == "Discard"
            && sp.take("Mode").as_deref() == Some("TgtChoose")
            && sp
                .take("NumCards")
                .as_deref()
                .is_some_and(is_remembered_amount)
        {
            sp.take("IsCurse");
            let disc_def = sp.take("Defined");
            let Some(disc_who) =
                self.wave11_player_of_line(disc_def.as_deref(), target, targets_a_player)
            else {
                return fail(self, p, sub);
            };
            if disc_who != who {
                return fail(self, p, sub);
            }
            let next_sub = sp.take("SubAbility");
            if !sp.exhausted() {
                return fail(self, p, sub);
            }
            (
                format!(
                    "Effect::DiscardAmountForPlayers {{ who: {disc_who}, \
                     amount: Amount::RememberedCount }}"
                ),
                next_sub,
            )
        } else if sub_api == "DealDamage"
            && !sp.has("DamageSource")
            && !sp.has("NoPrevention")
            && !sp.has("OptionalDecider")
            && sp
                .take("NumDmg")
                .as_deref()
                .is_some_and(is_remembered_amount)
        {
            let dmg_def = sp.take("Defined");
            let Some(dmg_who) =
                self.wave11_player_of_line(dmg_def.as_deref(), target, targets_a_player)
            else {
                return fail(self, p, sub);
            };
            if !Self::wave28b_is_single_player_rel(dmg_who) {
                return fail(self, p, sub);
            }
            let next_sub = sp.take("SubAbility");
            if !sp.exhausted() {
                return fail(self, p, sub);
            }
            (
                format!(
                    "Effect::DealDamage {{ amount: Amount::RememberedCount, \
                     target: TargetSpec::Player({dmg_who}) }}"
                ),
                next_sub,
            )
        } else if sp.peek("ConditionDefined") == Some("Remembered")
            && sp.peek("ConditionPresent") == Some("Card")
            && matches!(
                sp.peek("ConditionCompare")
                    .or_else(|| sp.peek("PresentCompare")),
                Some("EQ0" | "LT1" | "LE0")
            )
        {
            sp.take("ConditionDefined");
            sp.take("ConditionPresent");
            sp.take("ConditionCompare");
            sp.take("PresentCompare");
            sp.take("ConditionDescription");
            let next_sub = sp.take("SubAbility");
            let Some(inner) = self.effect_of(&sub_api, &mut sp, target, false, false) else {
                return fail(self, p, sub);
            };
            if inner.is_empty() || !sp.exhausted() {
                return fail(self, p, sub);
            }
            (
                format!(
                    "Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                     left: &Amount::RememberedCount, op: AmountCmp::Equal, \
                     right: &Amount::Fixed(0) }}, then: &[{}], otherwise: &[] }}",
                    inner.join(", ")
                ),
                next_sub,
            )
        } else {
            return fail(self, p, sub);
        };
        let Ok(after_cleanup) = self.wave28b_consume_optional_cleanup(next_sub) else {
            return fail(self, p, sub);
        };
        *sub = after_cleanup;
        Some(vec![format!(
            "Effect::CaptureAffected {{ first: &[{first_eff}], then: &[{then_eff}] }}"
        )])
    }

    /// Lowers a Council's dilemma vote option that draws `VoteNum` cards and
    /// discards `Remembered$Amount` (*Messenger Jays*) via `CaptureAffected`.
    pub(super) fn wave28b_vote_draw_remembered_discard(
        &self,
        sp: &mut Params,
        option_idx: u8,
        sub_name: &str,
    ) -> Option<Vec<String>> {
        if sp.has("ValidTgts")
            || sp.has("UnlessCost")
            || sp.has("AtEOT")
            || sp.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let raw = sp.take("NumCards")?;
        let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
            return None;
        };
        if sp.take("RememberDrawn").as_deref() != Some("AllReplaced") {
            return None;
        }
        if !matches!(sp.take("Defined").as_deref(), None | Some("You")) || !sp.exhausted() {
            return None;
        }
        let sub_body = self.svars.get(sub_name.trim())?;
        let (sub_api, mut dp) = Params::parse(sub_body)?;
        dp.drop_prose();
        if sub_api != "Discard"
            || dp.take("Mode").as_deref() != Some("TgtChoose")
            || !matches!(dp.take("Defined").as_deref(), None | Some("You"))
        {
            return None;
        }
        let disc_num = dp.take("NumCards")?;
        let disc_def = self
            .svars
            .get(disc_num.trim())
            .map_or(disc_num.as_str(), String::as_str)
            .trim();
        if disc_def != "Remembered$Amount" {
            return None;
        }
        let next_sub = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        if self
            .wave28b_consume_optional_cleanup(next_sub)
            .ok()?
            .is_some()
        {
            return None;
        }
        Some(vec![format!(
            "Effect::CaptureAffected {{ first: &[Effect::DrawCards {{ amount: {amt} }}], \
             then: &[Effect::DiscardAmountForPlayers {{ who: PlayerRel::You, \
             amount: Amount::RememberedCount }}] }}"
        )])
    }

    fn wave28b_may_damage_self_reflexive(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let saved_p = p.clone();
        let saved_sub = sub.clone();
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let fail = |this: &mut Self, p: &mut Params, sub: &mut Option<String>| {
            *p = saved_p;
            *sub = saved_sub;
            this.body.statics.truncate(saved_statics);
            *this.unclaimed.borrow_mut() = saved_unclaimed;
            None
        };
        if p.has("DamageSource") || p.has("NoPrevention") {
            return fail(self, p, sub);
        }
        let Some(sub_name) = sub.clone() else {
            return fail(self, p, sub);
        };
        let Some(sub_body) = self.svars.get(sub_name.trim()).cloned() else {
            return fail(self, p, sub);
        };
        let Some((sub_api, mut ip)) = Params::parse(&sub_body) else {
            return fail(self, p, sub);
        };
        ip.drop_prose();
        if sub_api != "ImmediateTrigger"
            || ip.take("ConditionDefined").as_deref() != Some("Remembered")
            || ip.take("ConditionPresent").as_deref() != Some("You")
            || !matches!(ip.take("ConditionCompare").as_deref(), None | Some("GE1"))
        {
            return fail(self, p, sub);
        }
        let Some(exec_name) = ip.take("Execute") else {
            return fail(self, p, sub);
        };
        let next_sub = ip.take("SubAbility");
        if !ip.exhausted() {
            return fail(self, p, sub);
        }
        let Ok(after_cleanup) = self.wave28b_consume_optional_cleanup(next_sub) else {
            return fail(self, p, sub);
        };
        let Some(exec_body) = self.svars.get(exec_name.trim()).cloned() else {
            return fail(self, p, sub);
        };
        let mut refl_chain = Chain::default();
        if self.chain(&exec_body, &mut refl_chain).is_none()
            || refl_chain.effects.is_empty()
            || refl_chain.second_target.is_some()
            || refl_chain.target_is_x
        {
            return fail(self, p, sub);
        }
        let Some(num_dmg) = p.take("NumDmg") else {
            return fail(self, p, sub);
        };
        let Some(amount) = self.amount_or_count_ext(&num_dmg) else {
            return fail(self, p, sub);
        };
        p.take("Defined");
        p.take("OptionalDecider");
        p.take("RememberDamaged");
        let targets = Self::target_req_expr(&refl_chain)
            .map_or_else(|| "None".to_string(), |req| format!("Some({req})"));
        let refl_effects = refl_chain.effects.join(", ");
        *sub = after_cleanup;
        Some(vec![format!(
            "Effect::MayDo {{ effects: &[Effect::DealDamage {{ amount: {amount}, \
             target: TargetSpec::Player(PlayerRel::You) }}, \
             Effect::ReflexiveTrigger {{ when: ReflexiveWhen::ThisWay(ThisWay::DamageDealtToYou), \
             once: true, condition: None, targets: {targets}, remember: TriggerRemember::None, \
             effects: &[{refl_effects}] }}] }}"
        )])
    }

    fn wave28b_draw_then_discard_drawn(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let saved_p = p.clone();
        let saved_sub = sub.clone();
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let fail = |this: &mut Self, p: &mut Params, sub: &mut Option<String>| {
            *p = saved_p;
            *sub = saved_sub;
            this.body.statics.truncate(saved_statics);
            *this.unclaimed.borrow_mut() = saved_unclaimed;
            None
        };
        if p.has("OptionalDecider") || p.has("Upto") {
            return fail(self, p, sub);
        }
        let Some(sub_name) = sub.clone() else {
            return fail(self, p, sub);
        };
        let Some(sub_body) = self.svars.get(sub_name.trim()).cloned() else {
            return fail(self, p, sub);
        };
        let Some((sub_api, mut dp)) = Params::parse(&sub_body) else {
            return fail(self, p, sub);
        };
        dp.drop_prose();
        if sub_api != "Discard"
            || dp.take("Mode").as_deref() != Some("TgtChoose")
            || !matches!(dp.take("Defined").as_deref(), None | Some("You"))
            || dp.take("DiscardValid").as_deref() != Some("Card.IsRemembered")
        {
            return fail(self, p, sub);
        }
        let discard_raw = dp.take("NumCards").unwrap_or_else(|| "1".to_string());
        let Some(discard) = self
            .plain_num(&discard_raw)
            .and_then(|n| u8::try_from(n).ok())
            .filter(|&n| n >= 1)
        else {
            return fail(self, p, sub);
        };
        let next_sub = dp.take("SubAbility");
        if !dp.exhausted() {
            return fail(self, p, sub);
        }
        let Ok(after_cleanup) = self.wave28b_consume_optional_cleanup(next_sub) else {
            return fail(self, p, sub);
        };
        let draw_raw = p.take("NumCards").unwrap_or_else(|| "1".to_string());
        let Some(draw) = self.amount_or_count_ext(&draw_raw) else {
            return fail(self, p, sub);
        };
        p.take("Defined");
        p.take("RememberDrawn");
        *sub = after_cleanup;
        Some(vec![format!(
            "Effect::DrawThenDiscardDrawn {{ draw: {draw}, discard: {discard} }}"
        )])
    }

    #[allow(clippy::too_many_lines)]
    fn wave28b_put_counter_choices(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let saved_p = p.clone();
        let saved_sub = sub.clone();
        let saved_statics = self.body.statics.len();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let fail = |this: &mut Self, p: &mut Params, sub: &mut Option<String>| {
            *p = saved_p;
            *sub = saved_sub;
            this.body.statics.truncate(saved_statics);
            *this.unclaimed.borrow_mut() = saved_unclaimed;
            None
        };
        if targets_here
            || p.has("Defined")
            || p.has("ForColor")
            || p.has("SharedKeywords")
            || p.has("DividedAsYouChoose")
            || p.has("PutOnEachOther")
            || p.has("RememberPut")
            || p.has("ChoiceAmount")
            || p.has("MinChoiceAmount")
            || !matches!(p.peek("Chooser"), None | Some("You"))
            || !matches!(p.peek("Placer"), None | Some("You"))
        {
            return fail(self, p, sub);
        }
        let choices = p.take("Choices")?;
        p.take("ChoiceTitle");
        p.take("ChoicesDesc");
        p.take("Chooser");
        p.take("Placer");
        p.take("IsCurse");
        let min = match (
            p.take("Optional").as_deref(),
            p.take("Mandatory").as_deref(),
        ) {
            (Some("True"), None) => 0u8,
            (None | Some("False"), None | Some("True")) => 1u8,
            _ => return fail(self, p, sub),
        };
        let remember_cards = match p.take("RememberCards").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return fail(self, p, sub),
        };
        let saved_single = std::mem::replace(
            &mut self.single_object_target,
            single_target && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")),
        );
        let specs = if let Some(code) = p.take("CounterType") {
            let Some(kind) = counter_kind(&code) else {
                self.single_object_target = saved_single;
                return fail(self, p, sub);
            };
            let raw_num = p.take("CounterNum");
            let Some(n) = self.amount_or_count_ext(raw_num.as_deref().unwrap_or("1")) else {
                self.single_object_target = saved_single;
                return fail(self, p, sub);
            };
            vec![(kind, n)]
        } else if let Some(codes) = p.take("CounterTypes") {
            let raw_num = p.take("CounterNum");
            let Some(n) = self.amount_or_count_ext(raw_num.as_deref().unwrap_or("1")) else {
                self.single_object_target = saved_single;
                return fail(self, p, sub);
            };
            let Ok(specs) = Self::collect_counter_specs(&codes, &n) else {
                self.single_object_target = saved_single;
                return fail(self, p, sub);
            };
            specs
        } else {
            self.single_object_target = saved_single;
            return fail(self, p, sub);
        };
        self.single_object_target = saved_single;
        let Some(filter_expr) = self.filter_expr(&choices) else {
            return fail(self, p, sub);
        };
        let filter_ref = self.wave28b_filter_ref(&filter_expr);
        let has_outer_object_target = target.is_some_and(|t| {
            !targets_a_player
                && !matches!(
                    t,
                    "TargetSpec::AnyPlayer"
                        | "TargetSpec::AnyOpponent"
                        | "TargetSpec::PlayerWhoAttackedThisTurn"
                )
                && !t.starts_with("TargetSpec::Player(")
        });
        if has_outer_object_target {
            if remember_cards || self.capture_scope.is_some() {
                return fail(self, p, sub);
            }
            let adds: Vec<String> = specs
                .iter()
                .map(|(kind, n)| {
                    format!(
                        "Effect::AddCounterFilter {{ filter: &Filter::IsRemembered, \
                         kind: {kind}, amount: {n} }}"
                    )
                })
                .collect();
            return Some(vec![format!(
                "Effect::CaptureAffected {{ \
                 first: &[Effect::ChooseCardEx {{ chooser: PlayerRel::You, \
                 zone: ChooseZone::Battlefield, filter: {filter_ref}, min: {min}, \
                 max: Amount::Fixed(1), reveal_chosen: false, movement: ChooseMove::None, \
                 not_chosen_movement: ChooseMove::None, then: &[], for_unchosen: &[] }}], \
                 then: &[{}] }}",
                adds.join(", ")
            )]);
        }
        let mut then_effects: Vec<String> = specs
            .iter()
            .map(|(kind, n)| {
                format!(
                    "Effect::AddCounterFilter {{ filter: &Filter::Reference, \
                     kind: {kind}, amount: {n} }}"
                )
            })
            .collect();
        if remember_cards {
            let Some((extra_effects, after_cleanup)) =
                self.wave28b_remembered_pump_followup(sub.as_deref())
            else {
                return fail(self, p, sub);
            };
            then_effects.extend(extra_effects);
            *sub = after_cleanup;
        }
        Some(vec![format!(
            "Effect::ChooseCardEx {{ chooser: PlayerRel::You, \
             zone: ChooseZone::Battlefield, filter: {filter_ref}, min: {min}, \
             max: Amount::Fixed(1), reveal_chosen: false, movement: ChooseMove::None, \
             not_chosen_movement: ChooseMove::None, then: &[{}], for_unchosen: &[] }}",
            then_effects.join(", ")
        )])
    }

    fn wave28b_remembered_pump_followup(
        &self,
        sub: Option<&str>,
    ) -> Option<(Vec<String>, Option<String>)> {
        let sub_name = sub?.trim();
        let sub_body = self.svars.get(sub_name)?;
        let (api, mut sp) = Params::parse(sub_body)?;
        sp.drop_prose();
        if api != "Pump"
            || sp.take("Defined").as_deref() != Some("Remembered")
            || sp.has("NumAtt")
            || sp.has("NumDef")
            || !matches!(sp.take("Duration").as_deref(), None | Some("EndOfTurn"))
        {
            return None;
        }
        let kw_raw = sp.take("KW")?;
        let next_sub = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        let after_cleanup = self.wave28b_consume_optional_cleanup(next_sub).ok()?;
        let mut effects = Vec::new();
        for kw in kw_raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            let k = keyword_const(kw)?;
            effects.push(format!(
                "Effect::continuous(&Filter::Reference, Modifier::AddKeyword({k}), \
                 Duration::UntilEndOfTurn)"
            ));
        }
        (!effects.is_empty()).then_some((effects, after_cleanup))
    }

    /// Consumes a trailing `Cleanup | ClearRemembered$ True` (returning whatever
    /// `SubAbility$` follows the cleanup, if any). Returns `Err(())` if `next` is
    /// something other than a clean `Cleanup` step.
    fn wave28b_consume_optional_cleanup(&self, next: Option<String>) -> Result<Option<String>, ()> {
        let Some(name) = next else {
            return Ok(None);
        };
        let body = self.svars.get(name.trim()).ok_or(())?;
        let (api, mut cp) = Params::parse(body).ok_or(())?;
        cp.drop_prose();
        if api != "Cleanup" || cp.take("ClearRemembered").as_deref() != Some("True") {
            return Err(());
        }
        let after = cp.take("SubAbility");
        if cp.exhausted() { Ok(after) } else { Err(()) }
    }

    /// Reads one Saga chapter `SVar` body (including optional `AB$ ... | Cost$ ...`
    /// chapter costs) and returns the parsed chain plus formatted effects string.
    pub(super) fn wave28b_read_chapter_chain(
        &mut self,
        body: &str,
        name: &str,
    ) -> Option<(Chain, String)> {
        let (body, price) = match self.optional_trigger_price(body) {
            Some((stripped, price)) => (stripped, Some(price)),
            None => (body.to_string(), None),
        };
        self.cost_sacrifices_self = matches!(
            &price,
            Some(OptionalTriggerPrice::CostPart(p)) if p == "Sacrifice(&Filter::This)"
        );
        self.cost_sacrifices_other = matches!(
            &price,
            Some(OptionalTriggerPrice::CostPart(p))
                if p.starts_with("Sacrifice(") && p != "Sacrifice(&Filter::This)"
        );
        self.cost_discards_single_for_cost = matches!(
            &price,
            Some(OptionalTriggerPrice::CostPart(p)) if p.starts_with("Discard(")
        );
        self.in_discard_hand_may = matches!(price, Some(OptionalTriggerPrice::DiscardHand));
        self.in_resolution_chosen_x =
            matches!(price, Some(OptionalTriggerPrice::ChooseNumberPay { .. }));
        let mut chain = Chain::default();
        let chain_ok = self.chain(&body, &mut chain);
        self.cost_sacrifices_self = false;
        self.cost_sacrifices_other = false;
        self.cost_discards_single_for_cost = false;
        self.in_discard_hand_may = false;
        self.in_resolution_chosen_x = false;
        chain_ok?;
        if chain.effects.is_empty() || chain.target_is_x {
            return self.deny(format!("unsupported chapter `{name}`"));
        }
        let raw_effects = chain.effects.join(", ");
        let effects = if let Some(price) = price {
            let one_effect = match chain.effects.as_slice() {
                [only] => only.clone(),
                many => format!("Effect::Sequence(&[{}])", many.join(", ")),
            };
            let has_targets = chain.target.is_some() || chain.second_target.is_some();
            Self::wave28b_wrap_optional_trigger_price(
                price,
                &raw_effects,
                &one_effect,
                has_targets,
            )?
        } else {
            raw_effects
        };
        Some((chain, effects))
    }

    /// Extends `activated::optional_trigger_price` with additional optional
    /// costs on trigger-executed / Saga chapter `AB$` lines.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave28b_optional_trigger_price(
        &mut self,
        body: &str,
    ) -> Option<(String, OptionalTriggerPrice)> {
        if !body.trim_start().starts_with("AB$") {
            return None;
        }
        let parts: Vec<&str> = body.split(" | ").collect();
        let cost = parts
            .iter()
            .find_map(|part| part.strip_prefix("Cost$"))?
            .trim();
        if cost.starts_with("Mandatory") {
            return None;
        }
        let rest_without_cost = || -> String {
            parts
                .iter()
                .copied()
                .filter(|part| !part.starts_with("Cost$"))
                .collect::<Vec<_>>()
                .join(" | ")
        };
        if matches!(cost, "Discard<0/Hand>" | "Discard<1/Hand>") {
            return Some((rest_without_cost(), OptionalTriggerPrice::DiscardHand));
        }
        if cost == "ExileFromGrave<1/CARDNAME>"
            || (cost.starts_with("ExileFromGrave<1/CARDNAME/")
                && cost.find('>') == Some(cost.len() - 1))
        {
            return Some((
                rest_without_cost(),
                OptionalTriggerPrice::ExileFromGraveyardN {
                    count: 1,
                    filter: "&Filter::This".to_string(),
                },
            ));
        }
        if let Some(exile_inner) = cost
            .strip_prefix("ExileFromGrave<")
            .and_then(|s| s.strip_suffix('>'))
            && !exile_inner.contains('>')
            && let Some((n_raw, rest)) = exile_inner.split_once('/')
            && let Ok(count) = n_raw.trim().parse::<u8>()
            && (1..=20).contains(&count)
        {
            let spec = rest.split('/').next().unwrap_or("").trim();
            if !spec.is_empty()
                && !matches!(spec, "CARDNAME" | "NICKNAME")
                && !spec.contains("Triggered")
                && !spec.contains("Remembered")
                && !spec.contains("OppOwn")
                && !spec.contains("Any")
            {
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics = self.body.statics.len();
                if let Some(expr) = self.filter_expr_in_zone(spec, true)
                    && !Self::wave28b_filter_uses_resolution_state(&expr)
                {
                    let filter = self.wave28b_filter_ref(&expr);
                    return Some((
                        rest_without_cost(),
                        OptionalTriggerPrice::ExileFromGraveyardN { count, filter },
                    ));
                }
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics);
            }
        }
        if cost == "PayEnergy<1>" {
            return Some((
                rest_without_cost(),
                OptionalTriggerPrice::ChooseNumberPay {
                    pay_kind: "VariablePayKind::Energy",
                    prompt: "NumberChoiceKind::EnergyPayment",
                    max: "Amount::Fixed(1)".to_string(),
                },
            ));
        }
        if let Some(sub_inner) = cost
            .strip_prefix("SubCounter<")
            .and_then(|s| s.strip_suffix('>'))
            && !sub_inner.contains('>')
            && let Some((n_raw, kind_raw)) = sub_inner.split_once('/')
            && let Ok(n) = n_raw.trim().parse::<u32>()
            && n >= 1
            && let Some(kind) = counter_kind(kind_raw.trim().split('/').next().unwrap_or(""))
        {
            return Some((
                rest_without_cost(),
                OptionalTriggerPrice::RemoveCounterSelf { kind, n },
            ));
        }
        if let Some(life_raw) = cost
            .strip_prefix("PayLife<")
            .and_then(|s| s.strip_suffix('>'))
            && !life_raw.contains('>')
            && life_raw.trim().parse::<u32>().is_err()
        {
            let var = life_raw.trim();
            if self.svars.get(var).map(|s| s.trim()) == Some("Count$xPaid") {
                return self.wave28b_choose_number_pay_price(
                    &parts,
                    "VariablePayKind::Life",
                    "NumberChoiceKind::Choice",
                );
            }
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let saved_statics = self.body.statics.len();
            if let Some(amt) = self.amount_or_count_ext(var) {
                return Some((rest_without_cost(), OptionalTriggerPrice::Life(amt)));
            }
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics);
            return None;
        }
        if cost == "X" {
            return self.wave28b_choose_number_pay_price(
                &parts,
                "VariablePayKind::GenericMana",
                "NumberChoiceKind::ManaPayment",
            );
        }
        if let Some(tap_inner) = cost
            .strip_prefix("tapXType<")
            .and_then(|s| s.strip_suffix('>'))
            && !tap_inner.contains('>')
            && let Some((n_raw, rest)) = tap_inner.split_once('/')
            && let Ok(count) = n_raw.trim().parse::<u32>()
            && (2..=10).contains(&count)
        {
            let spec = rest.split('/').next().unwrap_or("").trim();
            let missing_other = !spec.contains("Other")
                && self.oracle.to_ascii_lowercase().contains("other untapped");
            if !spec.is_empty() && !matches!(spec, "CARDNAME" | "NICKNAME") && !missing_other {
                let saved_unclaimed = self.unclaimed.borrow().clone();
                let saved_statics = self.body.statics.len();
                let normalized = if spec.contains("YouCtrl") || spec.contains("Ctrl") {
                    spec.to_string()
                } else {
                    spec.split(',')
                        .map(|alt| {
                            let alt = alt.trim();
                            if alt.contains('.') {
                                format!("{alt}+YouCtrl")
                            } else {
                                format!("{alt}.YouCtrl")
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                };
                if let Some(expr) = self.filter_expr(&normalized) {
                    let filter_name = self.body.filter_static("COST", &expr);
                    return Some((
                        rest_without_cost(),
                        OptionalTriggerPrice::CostPart(format!(
                            "TapN {{ filter: &{filter_name}, count: {count} }}"
                        )),
                    ));
                }
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics);
            }
        }
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics = self.body.statics.len();
        let Some((mana, cost_parts)) = self.cost_pieces(cost) else {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics);
            return None;
        };
        if !mana.is_empty() && !mana.contains("{X}") && cost_parts.is_empty() {
            return Some((rest_without_cost(), OptionalTriggerPrice::Printed(mana)));
        }
        if !mana.is_empty()
            && !mana.contains("{X}")
            && let [only] = cost_parts.as_slice()
            && let Some(life) = only
                .strip_prefix("PayLife(")
                .and_then(|s| s.strip_suffix(')'))
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|&n| n >= 1)
        {
            return Some((
                rest_without_cost(),
                OptionalTriggerPrice::ManaAndLife { mana, life },
            ));
        }
        if mana.is_empty() {
            if let [only] = cost_parts.as_slice()
                && (only.starts_with("TapN {")
                    || (only.starts_with("RemoveAnyCounter {")
                        && only.contains("count: 1,")
                        && !only.contains("kind: None")))
            {
                return Some((
                    rest_without_cost(),
                    OptionalTriggerPrice::CostPart(only.clone()),
                ));
            }
            if cost_parts.len() >= 2 {
                let first = &cost_parts[0];
                if let Some(filter) = first
                    .strip_prefix("ExileFromGraveyard(")
                    .and_then(|s| s.strip_suffix(')'))
                    && cost_parts.iter().all(|p| p == first)
                    && let Ok(count) = u8::try_from(cost_parts.len())
                {
                    return Some((
                        rest_without_cost(),
                        OptionalTriggerPrice::ExileFromGraveyardN {
                            count,
                            filter: filter.to_string(),
                        },
                    ));
                }
            }
        }
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        self.body.statics.truncate(saved_statics);
        None
    }

    fn wave28b_choose_number_pay_price(
        &mut self,
        parts: &[&str],
        pay_kind: &'static str,
        prompt: &'static str,
    ) -> Option<(String, OptionalTriggerPrice)> {
        if parts.iter().any(|p| p.starts_with("ValidTgts$")) {
            return None;
        }
        let xmax_raw = parts
            .iter()
            .find_map(|p| p.strip_prefix("XMax$"))
            .map(str::trim);
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics = self.body.statics.len();
        let max = if let Some(raw) = xmax_raw {
            let Some(amt) = self.amount_or_count_ext(raw) else {
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics);
                return None;
            };
            amt
        } else {
            "Amount::Fixed(u32::MAX)".to_string()
        };
        let rest = parts
            .iter()
            .copied()
            .filter(|part| !part.starts_with("Cost$") && !part.starts_with("XMax$"))
            .collect::<Vec<_>>()
            .join(" | ");
        Some((
            rest,
            OptionalTriggerPrice::ChooseNumberPay {
                pay_kind,
                prompt,
                max,
            },
        ))
    }

    /// Formats any [`OptionalTriggerPrice`] wrapper around the compiled effects
    /// of a trigger or Saga chapter. Returns `None` if a `ChooseNumberPay`
    /// ability declared targets or never read `Amount::ResolutionChosenNumber`.
    pub(super) fn wave28b_wrap_optional_trigger_price(
        price: OptionalTriggerPrice,
        effects: &str,
        one_effect: &str,
        has_targets: bool,
    ) -> Option<String> {
        Some(match price {
            OptionalTriggerPrice::Generic(mana) => format!(
                "Effect::PlayerMayPayThen {{ player: PlayerRel::You, \
                 mana: {mana}, effects: &[{effects}] }}"
            ),
            OptionalTriggerPrice::Printed(cost) => format!(
                "Effect::PlayerMayPayManaThen {{ player: PlayerRel::You, \
                 cost: mana!(\"{cost}\"), effects: &[{effects}] }}"
            ),
            OptionalTriggerPrice::Life(life) => format!(
                "Effect::PlayerMayPayLifeThen {{ player: PlayerRel::You, \
                 life: {life}, effects: &[{effects}] }}"
            ),
            OptionalTriggerPrice::CostPart(part) => format!(
                "Effect::PlayerMayPayCostThen {{ player: PlayerRel::You, \
                 cost: &CostPart::{part}, effect: &{one_effect} }}"
            ),
            OptionalTriggerPrice::SacrificeN { filter, count } => format!(
                "Effect::PlayerMaySacrificeNThen {{ player: PlayerRel::You, \
                 filter: {filter}, count: {count}, effect: &{one_effect} }}"
            ),
            OptionalTriggerPrice::DiscardHand => format!(
                "Effect::MayDo {{ effects: &[Effect::DiscardHand {{ who: PlayerRel::You }}, \
                 {effects}] }}"
            ),
            OptionalTriggerPrice::RemoveCounterSelf { kind, n } => format!(
                "Effect::MayDo {{ effects: &[Effect::RemoveCounterSelf {{ kind: {kind}, n: {n} }}, \
                 {effects}] }}"
            ),
            OptionalTriggerPrice::ExileFromGraveyardN { count, filter } => format!(
                "Effect::MayExileFromYourGraveyardThen {{ count: {count}, \
                 filter: {filter}, then: &[{effects}] }}"
            ),
            OptionalTriggerPrice::ManaAndLife { mana, life } => format!(
                "Effect::PlayerMayPayManaAndLifeEachThenElse {{ player: PlayerRel::You, \
                 mana: mana!(\"{mana}\"), life: {life}, times: Amount::Fixed(1), \
                 paid: &[{effects}], unpaid: &[] }}"
            ),
            OptionalTriggerPrice::ChooseNumberPay {
                pay_kind,
                prompt,
                max,
            } => {
                if pay_kind == "VariablePayKind::Energy"
                    && max == "Amount::Fixed(1)"
                    && !effects.contains("Amount::ResolutionChosenNumber")
                {
                    format!(
                        "Effect::IfCondition {{ condition: Condition::AmountAtLeast(\
                         Amount::CountersOnYou(CounterKind::Energy), 1), \
                         then: &[Effect::ChooseNumberEx {{ chooser: PlayerRel::You, \
                         min: Amount::Fixed(0), max: Amount::Fixed(1), \
                         prompt: NumberChoiceKind::EnergyPayment, \
                         pay_cost: Some(VariablePayKind::Energy), \
                         then: &[Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                         left: &Amount::ResolutionChosenNumber, op: AmountCmp::Equal, \
                         right: &Amount::Fixed(1) }}, then: &[{effects}], otherwise: &[] }}] }}], \
                         otherwise: &[] }}"
                    )
                } else {
                    if has_targets
                        || effects.contains("Effect::ReflexiveTrigger")
                        || !effects.contains("Amount::ResolutionChosenNumber")
                    {
                        return None;
                    }
                    format!(
                        "Effect::ChooseNumberEx {{ chooser: PlayerRel::You, min: Amount::Fixed(0), \
                         max: {max}, prompt: {prompt}, pay_cost: Some({pay_kind}), \
                         then: &[{effects}] }}"
                    )
                }
            }
        })
    }
}
