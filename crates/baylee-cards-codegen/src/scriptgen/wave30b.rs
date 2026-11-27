//! Wave 30b pure-`scriptgen` lowerings across:
//! - `DealDamage` & `DealDamage.TargetMin`:
//!   - `TapsForMana` `TriggeredCard` damage source (*Barbflare Gremlin*)
//!   - `ImmediateTrigger` reflexive `Sac<1/...>` / `RememberObjects$ Sacrificed`
//!     `TriggerRemembered$CardPower` (*Ziatora, the Incinerator*) and two-branch
//!     `ConditionDefined$ DelayTriggerRememberedLKI` Giant power doubling
//!     (*Surtland Flinger*)
//!   - `ImmediateTrigger` under `ChangesZone` `Spawner>TriggeredCardLKICopy` +
//!     `Spawner>TriggeredCard$CardPower` (*Halana, Kessig Ranger*)
//!   - `second_target_is_x` (`TargetMin$ X | TargetMax$ X` where `X = Count$xPaid`)
//!     for Slot-2 `TargetReq::x_targets` (*Spinning Wheel Kick*, *Suffer the Past*)
//! - `Discard`:
//!   - `Cost$ Draw<1/You>` with conditional `TriggeredSource$Valid Card.!ThisTurnEntered`
//!     (*Moon-Circuit Hacker*)
//! - Multi-target chains & `TargetOnly.TargetMin`:
//!   - Dual-group `TargetOnly` (`0..=X` / `0..=Y`) -> `PutCounterAll` with
//!     `RememberTargets$ True` + `ImprintCards$ ThisTargetedCard` (*Filigree Vector*)
//!   - 3-slot graveyard `TargetOnly` -> `WithAllTargets` `MoveAndModify`
//!     (*Relive the Past*)
//!   - Slot-1 graveyard -> Slot-2 `TargetOnly` graveyard conditional
//!     (`Adamant`) + `ExileSource` (*Once and Future*)
//!   - Slot-2 `ChangeZone` (`CardInFirstTargetsGraveyard` -> `Exile`) with
//!     `RememberChanged$ True` + trailing `LoseLife` / `GainLife` syphon
//!     (*Suffer the Past*)
//!   - Slot-2 `AlterAttribute | Attributes$ Suspected` (*Reasonable Doubt*)

use super::chains::LineFlags;
use super::wave27c::MultiTargetAction;
use super::{Chain, Params, Tx, counter_kind, mana_color_const};

impl Tx<'_> {
    /// True when Slot-1 `TargetOnly | RememberTargets$ True` leads into
    /// Slot-2 `TargetOnly | ImprintCards$ ThisTargetedCard` (*Filigree Vector*).
    pub(super) fn wave30b_slot1_remember_targets_ok(&self, sub: Option<&str>) -> bool {
        let Some(sub_name) = sub else {
            return false;
        };
        let Some(body) = self.svars.get(sub_name.trim()) else {
            return false;
        };
        let Some((api, sp)) = Params::parse(body) else {
            return false;
        };
        self.wave30b_target_only_multi_safe(&api, &sp)
    }

    /// True when Slot-2 is a Wave 30b shape allowed by
    /// `wave27c_chain_extra_target` (*Filigree Vector*, *Once and Future*, or
    /// *Suffer the Past*).
    pub(super) fn wave30b_extra_target_slot2_allowed(
        &self,
        api: &str,
        p: &Params,
        chain: &Chain,
        spec: &str,
    ) -> bool {
        if api == "ChangeZone"
            && chain.effects.is_empty()
            && matches!(
                chain.target.as_deref(),
                Some("TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent")
            )
            && spec.starts_with("TargetSpec::CardInFirstTargetsGraveyard(")
            && p.peek("Origin") == Some("Graveyard")
            && p.peek("Destination") == Some("Exile")
            && p.peek("RememberChanged") == Some("True")
            && !self.later_line_targets(p.peek("SubAbility"))
        {
            return true;
        }
        if api != "TargetOnly" {
            return false;
        }
        // Filigree Vector: Slot-1 `TargetOnly` -> Slot-2 `TargetOnly | ImprintCards$ ThisTargetedCard` -> `PutCounterAll`.
        if chain.effects.is_empty()
            && chain
                .target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && spec.starts_with("TargetSpec::Object(")
            && self.wave30b_target_only_multi_safe(api, p)
        {
            return true;
        }
        // Once and Future: Slot-1 GraveyardToHand -> Slot-2 `TargetOnly | TgtZone$ Graveyard | RememberObjects$ ThisTargetedCard`.
        chain.effects.len() == 1
            && chain.effects[0].starts_with("Effect::GraveyardToHand")
            && spec.starts_with("TargetSpec::CardInGraveyard(")
            && p.peek("RememberObjects") == Some("ThisTargetedCard")
            && p.peek("SubAbility").is_some()
    }

    /// True when Slot-1 `TargetOnly | SubAbility$` targets another `TargetOnly`
    /// that terminates cleanly in a dual-group `PutCounterAll` (*Filigree Vector*).
    pub(super) fn wave30b_target_only_multi_safe(&self, sub_api: &str, sp: &Params) -> bool {
        if sub_api != "TargetOnly" || sp.peek("ImprintCards") != Some("ThisTargetedCard") {
            return false;
        }
        let Some(next_name) = sp.peek("SubAbility") else {
            return false;
        };
        let Some(next_body) = self.svars.get(next_name.trim()) else {
            return false;
        };
        let Some((api, np)) = Params::parse(next_body) else {
            return false;
        };
        api == "PutCounterAll"
            && np
                .peek("ValidCards")
                .is_some_and(|v| v.contains("IsRemembered"))
            && np
                .peek("ValidCards2")
                .is_some_and(|v| v.contains("IsImprinted"))
    }

    /// Additional Slot-2 `TargetOnly` follow-ups:
    /// 1. Dual-group `TargetOnly` (`RememberTargets$ True` / `ImprintCards$ ThisTargetedCard`)
    ///    -> `PutCounterAll` (`*Filigree Vector*`).
    /// 2. 2-slot graveyard `TargetOnly | RememberObjects$ ThisTargetedCard` ->
    ///    `Adamant`-branched `Library` vs `Hand` + `ExileSource` (*Once and Future*).
    pub(super) fn wave30b_target_only_slot2_followup(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if let Some(effs) = self.wave30b_dual_put_counter_all(p, sub, chain) {
            return Some(effs);
        }
        self.wave30b_once_and_future_slot2(p, sub, chain)
    }

    fn wave30b_dual_put_counter_all(
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
            || p.peek("ImprintCards") != Some("ThisTargetedCard")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("ImprintCards");
        probe.take("IsCurse");
        if !probe.exhausted() {
            return None;
        }
        let sub_name = sub.as_deref()?.trim().to_string();
        let body = self.svars.get(&sub_name)?.clone();
        let (api, mut dp) = Params::parse(&body)?;
        dp.drop_prose();
        if api != "PutCounterAll" {
            return None;
        }
        let v1 = dp.take("ValidCards")?;
        let v2 = dp.take("ValidCards2")?;
        let base1 = v1.strip_suffix(".IsRemembered")?;
        let base2 = v2.strip_suffix(".IsImprinted")?;
        if format!(
            "TargetSpec::Object(&{})",
            self.body.filter_static("TARGET", &self.filter_expr(base1)?)
        ) != *chain.target.as_deref()?
            && !self.wave30b_object_spec_matches(chain.target.as_deref()?, base1)
        {
            return None;
        }
        if !self.wave30b_object_spec_matches(chain.second_target.as_deref()?, base2) {
            return None;
        }
        let kind1 = counter_kind(dp.take("CounterType")?.trim())?;
        let amt1 = self.amount_or_count_ext(dp.take("CounterNum").as_deref().unwrap_or("1"))?;
        let kind2 = counter_kind(dp.take("CounterType2")?.trim())?;
        let amt2 = self.amount_or_count_ext(dp.take("CounterNum2").as_deref().unwrap_or("1"))?;
        let next = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        let mut trailing = None;
        self.wave30b_consume_cleanup(next, &["ClearImprinted", "ClearRemembered"], &mut trailing)?;
        *p = probe;
        *sub = trailing;
        Some(vec![
            format!("Effect::AddCounter {{ kind: {kind1}, amount: {amt1} }}"),
            format!(
                "Effect::WithSecondTarget(&[Effect::AddCounter {{ kind: {kind2}, amount: {amt2} }}])"
            ),
        ])
    }

    fn wave30b_object_spec_matches(&self, spec: &str, valid: &str) -> bool {
        let Some(expr) = self.filter_expr(valid) else {
            return false;
        };
        if spec == format!("TargetSpec::Object(&{expr})") {
            return true;
        }
        spec.strip_prefix("TargetSpec::Object(&")
            .and_then(|s| s.strip_suffix(')'))
            .is_some_and(|name| {
                self.body
                    .statics
                    .contains(&format!("static {name}: Filter = {expr};"))
            })
    }

    pub(super) fn wave30b_consume_cleanup(
        &self,
        next: Option<String>,
        expected_flags: &[&str],
        out: &mut Option<String>,
    ) -> Option<()> {
        let name = next?;
        let body = self.svars.get(name.trim())?;
        let (api, mut cp) = Params::parse(body)?;
        cp.drop_prose();
        if api != "Cleanup" {
            return None;
        }
        for flag in expected_flags {
            if cp.take(flag).as_deref() != Some("True") {
                return None;
            }
        }
        let trailing = cp.take("SubAbility");
        if !cp.exhausted() {
            return None;
        }
        *out = trailing;
        Some(())
    }

    /// Lowers *Once and Future* Slot 2:
    /// `TargetOnly | TgtZone$ Graveyard | RememberObjects$ ThisTargetedCard | SubAbility$ DBReturn`
    /// -> `DBReturn` (`Library` when `X == 0`) -> `DBReturn2` (`Hand` when `X >= 1`)
    /// -> `DBExile` (`Origin$ Stack | Destination$ Exile`) -> `DBCleanup`.
    fn wave30b_once_and_future_slot2(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if chain.effects.len() != 1
            || !chain.effects[0].starts_with("Effect::GraveyardToHand")
            || p.peek("RememberObjects") != Some("ThisTargetedCard")
            || !chain
                .second_target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::CardInGraveyard("))
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("RememberObjects");
        probe.take("IsCurse");
        if !probe.exhausted() {
            return None;
        }
        let slot2_spec = chain.second_target.as_deref()?.to_string();

        // 1. False arm (`ConditionSVarCompare$ EQ0`): Graveyard -> Top of Library
        let b1_name = sub.as_deref()?.trim().to_string();
        let b1_body = self.svars.get(&b1_name)?.clone();
        let (b1_api, mut b1) = Params::parse(&b1_body)?;
        b1.drop_prose();
        if b1_api != "ChangeZone"
            || b1.take("Origin").as_deref() != Some("Graveyard")
            || b1.take("Destination").as_deref() != Some("Library")
            || b1.take("Defined").as_deref() != Some("Remembered")
            || b1.take("ConditionSVarCompare").as_deref() != Some("EQ0")
            || b1.has("Shuffle")
            || !matches!(b1.take("LibraryPosition").as_deref(), None | Some("0"))
        {
            return None;
        }
        b1.take("Hidden");
        let cond_var = b1.take("ConditionCheckSVar")?;
        let b2_name = b1.take("SubAbility")?;
        if !b1.exhausted() {
            return None;
        }

        // 2. True arm (`ConditionCheckSVar$ <var>`, implicit `GE1`): Graveyard -> Hand
        let b2_body = self.svars.get(b2_name.trim())?.clone();
        let (b2_api, mut b2) = Params::parse(&b2_body)?;
        b2.drop_prose();
        if b2_api != "ChangeZone"
            || b2.take("Origin").as_deref() != Some("Graveyard")
            || b2.take("Destination").as_deref() != Some("Hand")
            || b2.take("Defined").as_deref() != Some("Remembered")
            || b2.take("ConditionCheckSVar").as_deref() != Some(cond_var.as_str())
            || !matches!(
                b2.take("ConditionSVarCompare").as_deref(),
                None | Some("GE1")
            )
        {
            return None;
        }
        b2.take("Hidden");
        let ex_name = b2.take("SubAbility")?;
        if !b2.exhausted() {
            return None;
        }

        // Parse Adamant condition (`Count$Adamant.<Color>.1.0`)
        let cond_def = self.svars.get(cond_var.trim())?.trim().to_string();
        let color_raw = cond_def
            .strip_prefix("Count$Adamant.")?
            .strip_suffix(".1.0")?;
        let color = match color_raw.trim() {
            "White" | "W" => "ManaColor::White",
            "Blue" | "U" => "ManaColor::Blue",
            "Black" | "B" => "ManaColor::Black",
            "Red" | "R" => "ManaColor::Red",
            "Green" | "G" => "ManaColor::Green",
            "Colorless" | "C" => "ManaColor::Colorless",
            other => mana_color_const(other)?,
        };
        let adamant_cond =
            format!("Condition::AmountAtLeast(Amount::ManaColorSpentToCast({color}), 3)");

        // 3. Self exile (`Origin$ Stack | Destination$ Exile`) + cleanup
        let ex_body = self.svars.get(ex_name.trim())?.clone();
        let (ex_api, mut ex) = Params::parse(&ex_body)?;
        ex.drop_prose();
        if ex_api != "ChangeZone"
            || !matches!(ex.take("Defined").as_deref(), None | Some("Self"))
            || ex.take("Origin").as_deref() != Some("Stack")
            || ex.take("Destination").as_deref() != Some("Exile")
        {
            return None;
        }
        let clean_name = ex.take("SubAbility");
        if !ex.exhausted() {
            return None;
        }
        let mut trailing = None;
        self.wave30b_consume_cleanup(clean_name, &["ClearRemembered"], &mut trailing)?;

        *p = probe;
        *sub = trailing;
        Some(vec![
            format!(
                "Effect::WithSecondTarget(&[Effect::IfCondition {{ condition: {adamant_cond}, \
                 then: &[Effect::GraveyardToHand {{ target: {slot2_spec} }}], \
                 otherwise: &[Effect::GraveyardToTop {{ target: {slot2_spec} }}] }}])"
            ),
            "Effect::ExileSource".to_string(),
        ])
    }

    /// True when `chain` declares 2 or 3 graveyard slots sharing the same `PlayerRel`
    /// (allowing `TargetOnly` chaining into `WithAllTargets` `MoveAndModify` for
    /// *Relive the Past*).
    pub(super) fn wave30b_graveyard_slots_share_rel(
        chain: &Chain,
        extra_spec: Option<&str>,
    ) -> bool {
        let Some(first_rel) = chain.target.as_deref().and_then(Self::gy_spec_rel) else {
            return false;
        };
        let Some(second_rel) = chain.second_target.as_deref().and_then(Self::gy_spec_rel) else {
            return false;
        };
        if first_rel != second_rel {
            return false;
        }
        if let Some(third) = chain.third_target.as_deref()
            && Self::gy_spec_rel(third) != Some(first_rel)
        {
            return false;
        }
        if let Some(extra) = extra_spec
            && Self::gy_spec_rel(extra) != Some(first_rel)
        {
            return false;
        }
        true
    }

    fn gy_spec_rel(spec: &str) -> Option<&str> {
        let inner = spec
            .strip_prefix("TargetSpec::CardInGraveyard(")?
            .strip_suffix(')')?;
        let (_, rel) = inner.rsplit_once(',')?;
        Some(rel.trim())
    }

    /// True when `sub` is a `Defined$ Targeted` graveyard-to-battlefield `ChangeZone`
    /// with `StaticEffect$` (or without) that can run inside `WithAllTargets`.
    pub(super) fn wave30b_is_all_gy_targets_followup(&self, sub: Option<&str>) -> bool {
        let Some(sub_name) = sub else {
            return false;
        };
        let Some(body) = self.svars.get(sub_name.trim()) else {
            return false;
        };
        let Some((api, sp)) = Params::parse(body) else {
            return false;
        };
        api == "ChangeZone"
            && sp.peek("Defined") == Some("Targeted")
            && sp.peek("Origin") == Some("Graveyard")
            && sp.peek("Destination") == Some("Battlefield")
            && !self.later_line_targets(sp.peek("SubAbility"))
    }

    /// Lowers trailing `ChangeZone | Origin$ Graveyard | Destination$ Battlefield | Defined$ Targeted`
    /// across 2 or 3 graveyard target slots inside `Effect::WithAllTargets(&[...])`
    /// (*Relive the Past*).
    pub(super) fn wave30b_multi_target_step(
        &mut self,
        api: &str,
        p: &mut Params,
        chain: &Chain,
        targets_here: bool,
    ) -> Option<(Vec<String>, Option<MultiTargetAction>)> {
        if targets_here
            || api != "ChangeZone"
            || p.peek("Defined") != Some("Targeted")
            || p.peek("Origin") != Some("Graveyard")
            || p.peek("Destination") != Some("Battlefield")
            || p.has("RememberChanged")
            || !Self::wave30b_graveyard_slots_share_rel(chain, None)
        {
            return None;
        }
        let rel = Self::gy_spec_rel(chain.target.as_deref()?)?;
        let union_target = format!("TargetSpec::CardInGraveyard(&Filter::Any, {rel})");
        let mut probe = p.clone();
        probe.take("Defined");
        probe.take("IsCurse");
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics = self.body.statics.len();
        let effs = if probe.has("StaticEffect") {
            probe.take("Origin");
            probe.take("Destination");
            let owner_control = match probe.take("GainControl").as_deref() {
                None | Some("True") => false,
                Some("False") => true,
                Some(_) => {
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics);
                    return None;
                }
            };
            let tapped = match probe.take("Tapped").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => {
                    *self.unclaimed.borrow_mut() = saved_unclaimed;
                    self.body.statics.truncate(saved_statics);
                    return None;
                }
            };
            let Ok(counters) = self.take_with_counters_amount(&mut probe) else {
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics);
                return None;
            };
            let Some(modifiers) = self.take_optional_static_effect(&mut probe, false) else {
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics);
                return None;
            };
            if modifiers.is_empty() {
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics);
                return None;
            }
            vec![Self::format_move_and_modify(
                &format!("MoveSubject::Target({union_target})"),
                Some("ZoneRef::Graveyard"),
                false,
                owner_control,
                tapped,
                None,
                counters.as_ref(),
                &modifiers,
                "Duration::Permanent",
            )]
        } else {
            let Some(effs) = self.effect_of_inner(
                api,
                &mut probe,
                Some(&union_target),
                LineFlags {
                    targets_here: true,
                    targets_a_player: false,
                    single_target: false,
                    prior_target_used: false,
                },
            ) else {
                *self.unclaimed.borrow_mut() = saved_unclaimed;
                self.body.statics.truncate(saved_statics);
                return None;
            };
            effs
        };
        if !probe.exhausted() || effs.is_empty() {
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics);
            return None;
        }
        *p = probe;
        Some((effs, Some(MultiTargetAction::AllTargets)))
    }

    /// Lowers *Suffer the Past* Slot-2 `ChangeZone`:
    /// `TargetOnly (Player)` -> `ChangeZone | TargetMin$ X | TargetMax$ X | Origin$ Graveyard | Destination$ Exile | TargetsWithDefinedController$ ParentTarget | RememberChanged$ True | SubAbility$ DBSyphonLife`
    /// -> `LoseLife | Defined$ Targeted | LifeAmount$ Y` -> `GainLife | Defined$ You | LifeAmount$ Y` -> `Cleanup`.
    pub(super) fn wave30b_first_target_gy_remembered_exile(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        second_spec: &str,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if !second_spec.starts_with("TargetSpec::CardInFirstTargetsGraveyard(")
            || !matches!(
                chain.target.as_deref(),
                Some("TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent")
            )
            || p.peek("Origin") != Some("Graveyard")
            || p.peek("Destination") != Some("Exile")
            || p.peek("RememberChanged") != Some("True")
            || p.has("Imprint")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Origin");
        probe.take("Destination");
        probe.take("RememberChanged");
        probe.take("IsCurse");
        if !probe.exhausted() {
            return None;
        }
        let filter_ref = second_spec
            .strip_prefix("TargetSpec::CardInFirstTargetsGraveyard(")?
            .strip_suffix(')')?
            .trim();

        // Next line: LoseLife | Defined$ Targeted | LifeAmount$ <var>
        let l1_name = sub.as_deref()?.trim().to_string();
        let l1_body = self.svars.get(&l1_name)?.clone();
        let (l1_api, mut l1) = Params::parse(&l1_body)?;
        l1.drop_prose();
        if l1_api != "LoseLife" || l1.take("Defined").as_deref() != Some("Targeted") {
            return None;
        }
        let amt_var1 = l1.take("LifeAmount")?;
        let l2_name = l1.take("SubAbility")?;
        if !l1.exhausted() {
            return None;
        }

        // Next line: GainLife | Defined$ You | LifeAmount$ <var>
        let l2_body = self.svars.get(l2_name.trim())?.clone();
        let (l2_api, mut l2) = Params::parse(&l2_body)?;
        l2.drop_prose();
        if l2_api != "GainLife" || l2.take("Defined").as_deref() != Some("You") {
            return None;
        }
        let amt_var2 = l2.take("LifeAmount")?;
        let clean_name = l2.take("SubAbility");
        if !l2.exhausted() {
            return None;
        }

        let def1 = self.svars.get(amt_var1.trim())?.trim();
        let def2 = self.svars.get(amt_var2.trim())?.trim();
        if def1 != "Remembered$Amount" || def2 != "Remembered$Amount" {
            return None;
        }
        let mut trailing = None;
        self.wave30b_consume_cleanup(clean_name, &["ClearRemembered"], &mut trailing)?;
        let count_expr = format!("Amount::ThisWay(ThisWay::ExiledFromGraveyard({filter_ref}))");
        *p = probe;
        *sub = trailing;
        Some(vec![
            format!("Effect::WithSecondTarget(&[Effect::exile({second_spec})])"),
            format!(
                "Effect::LoseLife {{ amount: {count_expr}, target: PlayerRel::TargetPlayerOrController }}"
            ),
            format!("Effect::GainLife {{ amount: {count_expr} }}"),
        ])
    }

    /// Lowers *Moon-Circuit Hacker*: `Discard | Mode$ TgtChoose | Cost$ Draw<1/You>`
    /// when `NumCards$` names `TriggeredSource$Valid Card.!ThisTurnEntered` (or
    /// `Card.ThisTurnEntered`) on a `Card.Self` trigger.
    pub(super) fn wave30b_discard_draw_cost_conditional(&mut self, raw_n: &str) -> Option<String> {
        if !self.trigger_is_self {
            return None;
        }
        let def = self
            .svars
            .get(raw_n.trim())
            .map_or(raw_n.trim(), String::as_str)
            .trim();
        let filter_expr = match def {
            "TriggeredSource$Valid Card.!ThisTurnEntered" => {
                "Filter::Not(&Filter::EnteredThisTurn)"
            }
            "TriggeredSource$Valid Card.ThisTurnEntered" => "Filter::EnteredThisTurn",
            _ => return None,
        };
        let filter_ref = if filter_expr == "Filter::EnteredThisTurn" {
            "Filter::EnteredThisTurn".to_string()
        } else {
            self.body.filter_static("COND", filter_expr)
        };
        Some(format!(
            "Effect::IfCondition {{ condition: Condition::ObjectMatches {{ \
             what: &TargetSpec::ThisObject, filter: &{filter_ref} }}, \
             then: &[Effect::DiscardForPlayers {{ who: PlayerRel::You, count: 1 }}], \
             otherwise: &[] }}"
        ))
    }

    /// Lowers *Surtland Flinger*'s two-branch `ImmediateTrigger` `DealDamage`:
    /// `ConditionDefined$ DelayTriggerRememberedLKI | ConditionPresent$ <Type> | ConditionCompare$ EQ0 | ValidTgts$ Any | NumDmg$ TriggerRemembered$CardPower | SubAbility$ DBGiantDamage`
    /// where `DBGiantDamage` doubles the damage for `<Type>` (`ConditionCompare$ GE1 | Defined$ Targeted`).
    pub(super) fn wave30b_surtland_flinger_deal_damage(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if !self.in_delayed
            || p.peek("ConditionDefined") != Some("DelayTriggerRememberedLKI")
            || p.peek("ConditionCompare") != Some("EQ0")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("ConditionDefined");
        probe.take("ConditionCompare");
        let type_raw = probe.take("ConditionPresent")?;
        let num_raw = probe.take("NumDmg")?;
        probe.take("Defined");
        if !probe.exhausted() {
            return None;
        }
        let to = target?;

        let sub_name = sub.as_deref()?.trim().to_string();
        let sub_body = self.svars.get(&sub_name)?.clone();
        let (s_api, mut sp) = Params::parse(&sub_body)?;
        sp.drop_prose();
        if s_api != "DealDamage"
            || sp.take("ConditionDefined").as_deref() != Some("DelayTriggerRememberedLKI")
            || sp.take("ConditionPresent").as_deref() != Some(type_raw.as_str())
            || sp.take("ConditionCompare").as_deref() != Some("GE1")
            || sp.take("Defined").as_deref() != Some("Targeted")
        {
            return None;
        }
        let giant_num_raw = sp.take("NumDmg")?;
        let trailing = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }

        let def1 = self
            .svars
            .get(num_raw.trim())
            .map_or(num_raw.trim(), String::as_str)
            .trim();
        let def2 = self
            .svars
            .get(giant_num_raw.trim())
            .map_or(giant_num_raw.trim(), String::as_str)
            .trim();
        if def1 != "TriggerRemembered$CardPower" {
            return None;
        }
        let factor: u32 = if def2 == "TriggerRemembered$CardPower/Twice" {
            2
        } else {
            let n_str = def2.strip_prefix("TriggerRemembered$CardPower/Times.")?;
            n_str.trim().parse().ok().filter(|&n| n >= 2)?
        };

        let trimmed = type_raw.trim();
        let type_expr = self
            .cats
            .const_path(trimmed)
            .map(|path| format!("Filter::HasSubtype({path})"))
            .or_else(|| self.filter_expr(trimmed))
            .or_else(|| self.filter_expr(&format!("Card.{trimmed}")))?;
        let filter_name = self.body.filter_static("COND", &type_expr);
        *p = probe;
        *sub = trailing;
        Some(vec![format!(
            "Effect::DealDamage {{ amount: Amount::IfCondition {{ \
             condition: &Condition::ObjectMatches {{ what: &TargetSpec::EventObject, filter: &{filter_name} }}, \
             then: &Amount::Times {{ base: &Amount::EventPower, factor: {factor} }}, \
             otherwise: &Amount::EventPower }}, target: {to} }}"
        )])
    }
}
