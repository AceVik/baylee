//! Wave 18 Lane A scriptgen support:
//! - `TgtZone$ Graveyard` targeted cast permissions & cost/replacement riders
//!   (*Havengul Lich*, *Heiko Yamazaki*, *Mavinda*, *Norika Yamazaki*, *Silas Renn*,
//!   *Sins of the Past*, *Vohar*, *Zul Ashur*),
//! - *Jolted Awake* (`PutCounter` + `PayEnergy<Targeted$CardManaCost>`),
//! - *Command the Dreadhorde* (`ChooseCard | TargetMax$ NumCreatPW` + total MV damage + reanimate),
//! - *Rescue from the Underworld* (`DelayedTrigger | RememberObjects$ Targeted & SacrificedCards`),
//! - *Spirit-Sister's Call* (`ChooseCard` + `Sacrifice | SacValid$ ...sharesCardTypeWith Targeted` + leaves-exile).

use super::{Chain, Params, Tx, statics::simple_const_filter};

pub(super) enum SpecialChain {
    Matched,
    Refused,
}

impl SpecialChain {
    pub(super) const fn into_option(self) -> Option<()> {
        match self {
            Self::Matched => Some(()),
            Self::Refused => None,
        }
    }
}

enum SinsExileSub {
    Done,
    Next(String),
}

impl Tx<'_> {
    /// Intercepts Wave 18a `TgtZone$ Graveyard` chains before generic target parsing.
    /// Returns `Some(SpecialChain::Matched | Refused)` when the chain shape matches,
    /// or `None` to fall back to generic `chain()` parsing.
    pub(super) fn wave18a_try_special_chain(
        &mut self,
        api: &str,
        p: &Params,
        graveyard_description: Option<&str>,
        chain: &mut Chain,
    ) -> Option<SpecialChain> {
        if chain.target.is_some()
            || chain.second_target.is_some()
            || chain.third_target.is_some()
            || p.peek("TgtZone") != Some("Graveyard")
            || !p.has("ValidTgts")
        {
            return None;
        }
        let mut probe = p.clone();
        match api {
            "PutCounter"
                if probe.peek("CounterType") == Some("ENERGY")
                    && probe.peek("Defined") == Some("You") =>
            {
                self.wave18a_jolted_awake(&mut probe, graveyard_description, chain)
            }
            "ChooseCard" if probe.has("TargetMax") => {
                self.wave18a_command_dreadhorde(&mut probe, graveyard_description, chain)
            }
            "ChooseCard" if probe.peek("DefinedCards") == Some("Targeted") => {
                self.wave18a_spirit_sisters_call(&mut probe, graveyard_description, chain)
            }
            "DelayedTrigger"
                if self.cost_sacrifices_other
                    && probe.peek("RememberObjects") == Some("Targeted & SacrificedCards") =>
            {
                self.wave18a_rescue_from_underworld(&mut probe, graveyard_description, chain)
            }
            "Effect" if probe.peek("RememberObjects") == Some("Targeted") => {
                self.wave18a_targeted_gy_cast_effect(&mut probe, graveyard_description, chain)
            }
            _ => None,
        }
    }

    fn wave18a_jolted_awake(
        &mut self,
        p: &mut Params,
        graveyard_description: Option<&str>,
        chain: &mut Chain,
    ) -> Option<SpecialChain> {
        if p.take("TgtZone").as_deref() != Some("Graveyard")
            || p.take("Defined").as_deref() != Some("You")
            || p.take("CounterType").as_deref() != Some("ENERGY")
            || p.take("TargetMin").as_deref() != Some("0")
            || p.take("TargetMax").as_deref() != Some("1")
        {
            return None;
        }
        let n: u32 = p
            .take("CounterNum")?
            .trim()
            .parse()
            .ok()
            .filter(|&n| n > 0)?;
        p.claim_label("ValidTgtsDesc", "ValidTgts");
        let valid = p.take("ValidTgts")?;
        let defined_ctrl = p.take("TargetsWithDefinedController");
        let sub_name = p.take("SubAbility")?;
        if !p.exhausted() {
            return None;
        }
        let sub_body = self.svars.get(sub_name.trim())?.clone();
        let (sub_api, mut sp) = Params::parse(&sub_body)?;
        sp.drop_prose();
        if sub_api != "ChangeZone"
            || sp.take("Defined").as_deref() != Some("Targeted")
            || sp.take("Origin").as_deref() != Some("Graveyard")
            || sp.take("Destination").as_deref() != Some("Battlefield")
            || sp.take("UnlessPayer").as_deref() != Some("You")
            || sp.take("UnlessSwitched").as_deref() != Some("True")
            || sp.has("SubAbility")
        {
            return None;
        }
        let unless_var = sp
            .take("UnlessCost")?
            .strip_prefix("PayEnergy<")?
            .strip_suffix('>')?
            .trim()
            .to_string();
        if !sp.exhausted() {
            return None;
        }
        if self
            .svars
            .get(&unless_var)
            .map(String::as_str)
            .map(str::trim)
            != Some("Targeted$CardManaCost")
        {
            return None;
        }
        let Some(tgt) =
            self.graveyard_target(&valid, defined_ctrl.as_deref(), graveyard_description)
        else {
            return Some(SpecialChain::Refused);
        };
        chain.target = Some(tgt.clone());
        chain.target_up_to = Some(1);
        chain.effects.push(format!(
            "Effect::Energy {{ who: PlayerRel::You, amount: Amount::Fixed({n}) }}"
        ));
        chain.effects.push(format!(
            "Effect::MayPayEnergyEqualToTargetCmcThen {{ then: &[Effect::reanimate({tgt})] }}"
        ));
        Some(SpecialChain::Matched)
    }

    fn wave18a_command_dreadhorde(
        &mut self,
        p: &mut Params,
        graveyard_description: Option<&str>,
        chain: &mut Chain,
    ) -> Option<SpecialChain> {
        if p.take("TgtZone").as_deref() != Some("Graveyard")
            || p.take("TargetMin").as_deref() != Some("0")
        {
            return None;
        }
        let max_var = p.take("TargetMax")?;
        p.claim_label("ValidTgtsDesc", "ValidTgts");
        let valid = p.take("ValidTgts")?;
        let defined_ctrl = p.take("TargetsWithDefinedController");
        let sub1 = p.take("SubAbility")?;
        if !p.exhausted() {
            return None;
        }
        let max_def = self.svars.get(max_var.trim())?.trim().to_string();
        let count_valid = max_def.strip_prefix("Count$ValidGraveyard ")?.trim();
        if count_valid != valid
            && !Self::same_valid_atoms(
                &count_valid.replace("YouOwn", "YouCtrl"),
                &valid.replace("YouOwn", "YouCtrl"),
            )
        {
            return None;
        }
        let body1 = self.svars.get(sub1.trim())?.clone();
        let (api1, mut p1) = Params::parse(&body1)?;
        p1.drop_prose();
        if api1 != "DealDamage" || p1.take("Defined").as_deref() != Some("You") {
            return None;
        }
        let dmg_var = p1.take("NumDmg")?;
        let sub2 = p1.take("SubAbility")?;
        if !p1.exhausted()
            || self.svars.get(dmg_var.trim()).map(|s| s.trim()) != Some("Targeted$CardManaCost")
        {
            return None;
        }
        let body2 = self.svars.get(sub2.trim())?.clone();
        let (api2, mut p2) = Params::parse(&body2)?;
        p2.drop_prose();
        if api2 != "ChangeZone"
            || p2.take("Defined").as_deref() != Some("Targeted")
            || p2.take("GainControl").as_deref() != Some("True")
            || p2.take("Origin").as_deref() != Some("Graveyard")
            || p2.take("Destination").as_deref() != Some("Battlefield")
            || p2.has("SubAbility")
            || !p2.exhausted()
        {
            return None;
        }
        let Some(tgt) =
            self.graveyard_target(&valid, defined_ctrl.as_deref(), graveyard_description)
        else {
            return Some(SpecialChain::Refused);
        };
        chain.target = Some(tgt.clone());
        chain.target_up_to = Some(u8::MAX);
        chain.effects.push(
            "Effect::DealDamage { amount: Amount::TotalTargetsManaValue, target: TargetSpec::Player(PlayerRel::You) }"
                .to_string(),
        );
        chain.effects.push(format!("Effect::reanimate({tgt})"));
        Some(SpecialChain::Matched)
    }

    fn wave18a_spirit_sisters_call(
        &mut self,
        p: &mut Params,
        graveyard_description: Option<&str>,
        chain: &mut Chain,
    ) -> Option<SpecialChain> {
        if p.take("TgtZone").as_deref() != Some("Graveyard")
            || p.take("DefinedCards").as_deref() != Some("Targeted")
        {
            return None;
        }
        p.claim_label("ValidTgtsDesc", "ValidTgts");
        let valid = p.take("ValidTgts")?;
        let defined_ctrl = p.take("TargetsWithDefinedController");
        let sub1 = p.take("SubAbility")?;
        if !p.exhausted() {
            return None;
        }
        let body1 = self.svars.get(sub1.trim())?.clone();
        let (api1, mut p1) = Params::parse(&body1)?;
        p1.drop_prose();
        if api1 != "Sacrifice"
            || p1.take("Optional").as_deref() != Some("True")
            || p1.take("Amount").as_deref() != Some("1")
            || p1.take("RememberSacrificed").as_deref() != Some("True")
        {
            return None;
        }
        p1.take("SacMessage");
        let sac_valid = p1.take("SacValid")?;
        let sub2 = p1.take("SubAbility")?;
        if !p1.exhausted() {
            return None;
        }
        let sac_base = sac_valid
            .strip_suffix(".sharesCardTypeWith Targeted")
            .or_else(|| sac_valid.strip_suffix("+sharesCardTypeWith Targeted"))?;
        let sac_base_expr = self.filter_expr_in_zone(sac_base, false)?;
        let sac_expr = if sac_base_expr == "Filter::Any" {
            "Filter::SharesCardTypeWithReference".to_string()
        } else {
            format!("Filter::And(&[{sac_base_expr}, Filter::SharesCardTypeWithReference])")
        };

        let body2 = self.svars.get(sub2.trim())?.clone();
        let (api2, mut p2) = Params::parse(&body2)?;
        p2.drop_prose();
        if api2 != "ChangeZone"
            || p2.take("ConditionDefined").as_deref() != Some("Remembered")
            || p2.take("ConditionPresent").as_deref() != Some("Card")
            || p2.take("Defined").as_deref() != Some("Targeted")
            || p2.take("Origin").as_deref() != Some("Graveyard")
            || p2.take("Destination").as_deref() != Some("Battlefield")
        {
            return None;
        }
        let sub3 = p2.take("SubAbility")?;
        if !p2.exhausted() {
            return None;
        }

        let body3 = self.svars.get(sub3.trim())?.clone();
        let (api3, mut p3) = Params::parse(&body3)?;
        p3.drop_prose();
        if api3 != "Animate"
            || p3.take("ConditionDefined").as_deref() != Some("Remembered")
            || p3.take("ConditionPresent").as_deref() != Some("Card")
            || p3.take("Defined").as_deref() != Some("Targeted")
            || p3.take("Duration").as_deref() != Some("Permanent")
        {
            return None;
        }
        let rep_name = p3.take("Replacements")?;
        let sub4 = p3.take("SubAbility");
        if !p3.exhausted() || !self.wave18a_is_leaves_bf_exile_replacement(&rep_name) {
            return None;
        }
        if let Some(sub4_name) = sub4
            && !self.wave18a_is_cleanup_svar(&sub4_name)
        {
            return None;
        }

        let Some(tgt) =
            self.graveyard_target(&valid, defined_ctrl.as_deref(), graveyard_description)
        else {
            return Some(SpecialChain::Refused);
        };
        let sac_static = self.body.filter_static("SACRIFICE", &sac_expr);
        chain.target = Some(tgt.clone());
        chain.effects.push(format!(
            "Effect::PlayerMaySacrificeNThen {{ player: PlayerRel::You, filter: &{sac_static}, count: 1, \
             effect: &Effect::Sequence(&[Effect::reanimate({tgt}), \
             Effect::ModifyTargets {{ modifier: Modifier::ExileInsteadOfLeavingBattlefield, \
             duration: Duration::Indefinitely }}]) }}"
        ));
        Some(SpecialChain::Matched)
    }

    fn wave18a_is_leaves_bf_exile_replacement(&self, name: &str) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((event, mut rp)) = Params::parse(body) else {
            return false;
        };
        rp.drop_prose();
        if event != "Moved"
            || !matches!(
                rp.take("ActiveZones").as_deref(),
                None | Some("Battlefield")
            )
            || rp.take("Origin").as_deref() != Some("Battlefield")
            || rp.take("ValidCard").as_deref() != Some("Card.Self")
        {
            return false;
        }
        let Some(replace_with) = rp.take("ReplaceWith") else {
            return false;
        };
        if !rp.exhausted() {
            return false;
        }
        let Some(ex_body) = self.svars.get(replace_with.trim()) else {
            return false;
        };
        let Some((ex_api, mut ep)) = Params::parse(ex_body) else {
            return false;
        };
        ep.drop_prose();
        ex_api == "ChangeZone"
            && ep.take("Origin").as_deref() == Some("Battlefield")
            && ep.take("Destination").as_deref() == Some("Exile")
            && ep.take("Defined").as_deref() == Some("ReplacedCard")
            && ep.exhausted()
    }

    fn wave18a_rescue_from_underworld(
        &mut self,
        p: &mut Params,
        graveyard_description: Option<&str>,
        chain: &mut Chain,
    ) -> Option<SpecialChain> {
        if p.take("TgtZone").as_deref() != Some("Graveyard")
            || p.take("Mode").as_deref() != Some("Phase")
            || p.take("Phase").as_deref() != Some("Upkeep")
            || p.take("ValidPlayer").as_deref() != Some("You")
            || p.take("RememberObjects").as_deref() != Some("Targeted & SacrificedCards")
        {
            return None;
        }
        p.claim_label("ValidTgtsDesc", "ValidTgts");
        let valid = p.take("ValidTgts")?;
        let defined_ctrl = p.take("TargetsWithDefinedController");
        let execute_name = p.take("Execute")?;
        let sub_name = p.take("SubAbility");
        if !p.exhausted() {
            return None;
        }
        let ex_body = self.svars.get(execute_name.trim())?.clone();
        let (ex_api, mut ep) = Params::parse(&ex_body)?;
        ep.drop_prose();
        if ex_api != "ChangeZone"
            || ep.take("Origin").as_deref() != Some("Graveyard")
            || ep.take("Destination").as_deref() != Some("Battlefield")
            || ep.take("Defined").as_deref() != Some("DelayTriggerRememberedLKI")
            || ep.has("SubAbility")
        {
            return None;
        }
        let owner_control = match ep.take("GainControl").as_deref() {
            Some("True") => false,
            None | Some("False") => true,
            Some(_) => return None,
        };
        if !ep.exhausted() {
            return None;
        }
        let mut trailing_exile = false;
        if let Some(sub) = sub_name {
            if self.wave18a_is_exile_self_from_stack(&sub) {
                trailing_exile = true;
            } else {
                return None;
            }
        }
        let Some(tgt) =
            self.graveyard_target(&valid, defined_ctrl.as_deref(), graveyard_description)
        else {
            return Some(SpecialChain::Refused);
        };
        chain.target = Some(tgt);
        chain.effects.push(format!(
            "Effect::RegisterDelayedTrigger {{ when: DelayedTriggerWhen::AtStep {{ step: DelayedStep::Upkeep, \
             whose: Some(PlayerRel::You), this_turn_only: false, recurring: false }}, condition: None, \
             targets: None, remember: TriggerRemember::TargetAndSacrificed, \
             effects: &[Effect::ReturnDelayedObjectsToBattlefield {{ owner_control: {owner_control} }}] }}"
        ));
        if trailing_exile {
            chain.effects.push("Effect::ExileSource".to_string());
        }
        Some(SpecialChain::Matched)
    }

    fn wave18a_parse_gy_effect_statics(
        &mut self,
        statics: &str,
    ) -> Option<(bool, Option<(String, u8)>)> {
        let mut free_play: Option<bool> = None;
        let mut cost_tax: Option<(String, u8)> = None;
        for st_name in statics.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let st_body = self.svars.get(st_name)?.clone();
            let (mode, mut sp) = Params::parse(&st_body)?;
            sp.drop_prose();
            if mode == "Continuous" && sp.peek("MayPlay") == Some("True") {
                if free_play.is_some()
                    || sp.take("MayPlay").as_deref() != Some("True")
                    || sp.take("AffectedZone").as_deref() != Some("Graveyard")
                    || !matches!(
                        sp.take("Affected").as_deref(),
                        Some("Card.IsRemembered" | "Card.IsRemembered+nonLand")
                    )
                {
                    return None;
                }
                let free = match sp.take("MayPlayWithoutManaCost").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                if !sp.exhausted() {
                    return None;
                }
                free_play = Some(free);
            } else if mode == "RaiseCost" {
                if cost_tax.is_some()
                    || sp.take("ValidCard").as_deref()
                        != Some("Card.IsRemembered+CastSa Spell.MayPlaySource")
                    || sp.take("UnlessValidTarget").as_deref() != Some("True")
                    || sp.take("Activator").as_deref() != Some("You")
                    || sp.take("Type").as_deref() != Some("Spell")
                {
                    return None;
                }
                let raw_target = sp.take("ValidTarget")?;
                let generic: u8 = sp.take("Amount")?.trim().parse().ok().filter(|&n| n > 0)?;
                if !sp.exhausted() {
                    return None;
                }
                let stripped_target = raw_target
                    .strip_suffix("+inZoneBattlefield")
                    .or_else(|| raw_target.strip_suffix("+inRealZoneBattlefield"))
                    .unwrap_or(&raw_target);
                let target_expr = self.filter_expr_in_zone(stripped_target, false)?;
                let target_ref = if simple_const_filter(&target_expr) {
                    format!("&{target_expr}")
                } else {
                    format!("&{}", self.body.filter_static("COST_TGT", &target_expr))
                };
                cost_tax = Some((target_ref, generic));
            } else {
                return None;
            }
        }
        Some((free_play?, cost_tax))
    }

    fn wave18a_targeted_gy_cast_effect(
        &mut self,
        p: &mut Params,
        graveyard_description: Option<&str>,
        chain: &mut Chain,
    ) -> Option<SpecialChain> {
        if p.take("TgtZone").as_deref() != Some("Graveyard")
            || p.take("RememberObjects").as_deref() != Some("Targeted")
            || !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn"))
            || !matches!(p.take("ExileOnMoved").as_deref(), None | Some("Graveyard"))
        {
            return None;
        }
        p.take("Name");
        p.claim_label("ValidTgtsDesc", "ValidTgts");
        let valid = p.take("ValidTgts")?;
        let defined_ctrl = p.take("TargetsWithDefinedController");
        let statics = p.take("StaticAbilities")?;
        let replacements = p.take("ReplacementEffects");
        let triggers = p.take("Triggers");
        let sub_name = p.take("SubAbility");
        if !p.exhausted() {
            return None;
        }

        let (free, cost_tax) = self.wave18a_parse_gy_effect_statics(&statics)?;

        let mut exile_instead_of_graveyard = false;
        if let Some(rep_raw) = replacements {
            for r_name in rep_raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                if !self.wave18a_is_gy_cast_exile_replacement(r_name) {
                    return None;
                }
                exile_instead_of_graveyard = true;
            }
        }

        let mut steal_activated = false;
        if let Some(trig_raw) = triggers {
            for t_name in trig_raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                if !self.wave18a_is_havengul_lich_trigger(t_name) {
                    return None;
                }
                steal_activated = true;
            }
        }

        let mut trailing_exile = false;
        if let Some(sub) = sub_name {
            match self.wave18a_sins_exile_effect_sub(&sub) {
                Some(SinsExileSub::Next(next)) => {
                    exile_instead_of_graveyard = true;
                    if self.wave18a_is_exile_self_from_stack(&next) {
                        trailing_exile = true;
                    } else if !self.wave18a_is_cleanup_svar(&next) {
                        return None;
                    }
                }
                Some(SinsExileSub::Done) => {
                    exile_instead_of_graveyard = true;
                }
                None if !self.wave18a_is_cleanup_svar(&sub) => return None,
                None => {}
            }
        }

        let Some(tgt) =
            self.graveyard_target(&valid, defined_ctrl.as_deref(), graveyard_description)
        else {
            return Some(SpecialChain::Refused);
        };
        chain.target = Some(tgt);
        chain.effects.push(format!(
            "Effect::ModifyTargets {{ modifier: Modifier::MayCastFromGraveyardTargeted {{ \
             free: {free}, exile_instead_of_graveyard: {exile_instead_of_graveyard} }}, \
             duration: Duration::UntilEndOfTurn }}"
        ));
        if let Some((target_ref, generic)) = cost_tax {
            chain.effects.push(format!(
                "Effect::ModifyTargets {{ modifier: Modifier::CastCostIncreaseUnlessTargets {{ \
                 filter: {target_ref}, generic: {generic} }}, duration: Duration::UntilEndOfTurn }}"
            ));
        }
        if steal_activated {
            chain.effects.push(
                "Effect::GrantActivatedAbilitiesWhenTargetCast { duration: Duration::UntilEndOfTurn }"
                    .to_string(),
            );
        }
        if trailing_exile {
            chain.effects.push("Effect::ExileSource".to_string());
        }
        Some(SpecialChain::Matched)
    }

    fn wave18a_is_gy_cast_exile_replacement(&self, name: &str) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((event, mut rp)) = Params::parse(body) else {
            return false;
        };
        rp.drop_prose();
        if event != "Moved"
            || rp.take("Destination").as_deref() != Some("Graveyard")
            || !matches!(rp.take("Origin").as_deref(), None | Some("Stack"))
            || !matches!(
                rp.take("ValidCard").as_deref(),
                None | Some("Card.IsRemembered")
            )
            || !matches!(
                rp.take("ValidLKI").as_deref(),
                None | Some("Card.IsRemembered+CastSa Spell.MayPlaySource")
            )
        {
            return false;
        }
        let Some(rep_with) = rp.take("ReplaceWith") else {
            return false;
        };
        if !rp.exhausted() {
            return false;
        }
        let Some(ex_body) = self.svars.get(rep_with.trim()) else {
            return false;
        };
        let Some((ex_api, mut ep)) = Params::parse(ex_body) else {
            return false;
        };
        ep.drop_prose();
        if ex_api != "ChangeZone"
            || ep.take("Defined").as_deref() != Some("ReplacedCard")
            || !matches!(ep.take("Origin").as_deref(), Some("All" | "Stack"))
            || ep.take("Destination").as_deref() != Some("Exile")
        {
            return false;
        }
        let sub = ep.take("SubAbility");
        if !ep.exhausted() {
            return false;
        }
        sub.is_none_or(|sub_name| self.wave18a_is_exile_command_self(&sub_name))
    }

    fn wave18a_is_exile_command_self(&self, name: &str) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return false;
        };
        p.drop_prose();
        api == "ChangeZone"
            && p.take("Defined").as_deref() == Some("Self")
            && p.take("Origin").as_deref() == Some("Command")
            && p.take("Destination").as_deref() == Some("Exile")
            && p.exhausted()
    }

    fn wave18a_is_exile_self_from_stack(&self, name: &str) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return false;
        };
        p.drop_prose();
        api == "ChangeZone"
            && matches!(p.take("Defined").as_deref(), None | Some("Self"))
            && p.take("Origin").as_deref() == Some("Stack")
            && p.take("Destination").as_deref() == Some("Exile")
            && p.exhausted()
    }

    fn wave18a_is_cleanup_svar(&self, name: &str) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return false;
        };
        p.drop_prose();
        if api != "Cleanup" {
            return false;
        }
        p.take("ClearRemembered");
        p.take("ClearChosenCard");
        p.exhausted()
    }

    fn wave18a_sins_exile_effect_sub(&self, name: &str) -> Option<SinsExileSub> {
        let body = self.svars.get(name.trim())?;
        let (api, mut p) = Params::parse(body)?;
        p.drop_prose();
        if api != "Effect"
            || p.take("RememberObjects").as_deref() != Some("Targeted")
            || p.take("ExileOnMoved").as_deref() != Some("Stack")
        {
            return None;
        }
        let rep = p.take("ReplacementEffects")?;
        let next = p.take("SubAbility");
        if !p.exhausted() || !self.wave18a_is_gy_cast_exile_replacement(&rep) {
            return None;
        }
        Some(match next {
            Some(next_name) => SinsExileSub::Next(next_name),
            None => SinsExileSub::Done,
        })
    }

    fn wave18a_is_havengul_lich_trigger(&self, name: &str) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((mode, mut tp)) = Params::parse(body) else {
            return false;
        };
        tp.drop_prose();
        if mode != "SpellCast"
            || tp.take("ValidCard").as_deref() != Some("Card.IsRemembered+nonLand")
        {
            return false;
        }
        let Some(exec_name) = tp.take("Execute") else {
            return false;
        };
        if !tp.exhausted() {
            return false;
        }
        let Some(exec_body) = self.svars.get(exec_name.trim()) else {
            return false;
        };
        let Some((e_api, mut ep)) = Params::parse(exec_body) else {
            return false;
        };
        ep.drop_prose();
        if e_api != "Effect" || ep.take("RememberObjects").as_deref() != Some("TriggeredCard") {
            return false;
        }
        let Some(st_name) = ep.take("StaticAbilities") else {
            return false;
        };
        if !ep.exhausted() {
            return false;
        }
        let Some(st_body) = self.svars.get(st_name.trim()) else {
            return false;
        };
        let Some((st_mode, mut sp)) = Params::parse(st_body) else {
            return false;
        };
        sp.drop_prose();
        st_mode == "Continuous"
            && sp.take("AffectedDefined").as_deref() == Some("EffectSource")
            && sp.take("GainsAbilitiesOfDefined").as_deref() == Some("RememberedLKI")
            && sp.exhausted()
    }
}
