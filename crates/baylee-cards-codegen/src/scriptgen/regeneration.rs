//! Regeneration restrictions created by resolving effects.

use super::{Params, Tx};

impl Tx<'_> {
    /// Consume a complete damage-memory suffix only on the initial instruction.
    pub(super) fn take_damage_recipient_regeneration(
        &self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        initial: bool,
    ) -> Option<bool> {
        if api != "DealDamage" || !p.has("RememberDamaged") {
            return Some(false);
        }
        // Runtime damage facts span the resolution, not one instruction.
        if !initial {
            return self.deny("damage-recipient memory after an earlier instruction".into());
        }
        if p.take("RememberDamaged").as_deref() != Some("True") {
            return self.deny("a nontrue damage-recipient memory flag".into());
        }
        self.damage_recipient_regeneration(sub.as_deref()?)?;
        *sub = None;
        Some(true)
    }

    /// A temporary source ability forbidding regeneration of creatures it damaged.
    /// The referenced static is read as a rule, not an ignored animation label.
    pub(super) fn animate_regeneration_grant(
        &mut self,
        p: &mut Params,
        source_only: bool,
        duration: &str,
    ) -> Option<Vec<String>> {
        let lower = p.take("staticAbilities");
        let upper = p.take("StaticAbilities");
        let name = match (lower, upper) {
            (None, None) => return Some(Vec::new()),
            (Some(name), None) | (None, Some(name)) if source_only => name,
            _ => {
                return self.deny(
                    "a static animation grant with multiple references or another subject".into(),
                );
            }
        };
        let valid = self.no_regeneration_filter(name.trim())?;
        if valid != "Creature.DamagedBy" {
            return self.deny(format!("a granted regeneration rule on `{valid}`"));
        }
        Some(vec![Self::animate_expr(
            "Modifier::GrantStatic { filter: &Filter::And(&[Filter::CREATURE, \
             Filter::DealtDamageBySourceThisTurn]), modifier: &Modifier::CantBeRegenerated }",
            duration,
        )])
    }

    /// Validate the complete remembered-recipient consumer and optional cleanup.
    /// Runtime resolution already records actual recipients, including redirection.
    pub(super) fn damage_recipient_regeneration(&self, name: &str) -> Option<()> {
        let (api, mut p) = Params::parse(self.svars.get(name)?)?;
        p.drop_prose();
        if api != "Effect" || p.take("RememberObjects").as_deref() != Some("Remembered.Creature") {
            return self.deny("a damage memory consumed by another effect or recipient set".into());
        }
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if ends.as_deref() != Some("Battlefield")
            || !matches!(p.take("IsCurse").as_deref(), None | Some("True"))
        {
            return self
                .deny("a damage-recipient restriction with another lifetime or flag".into());
        }
        let rule = p.take("StaticAbilities")?;
        if self.no_regeneration_filter(rule.trim())? != "Card.IsRemembered" {
            return self.deny("a damage memory used by another static subject".into());
        }
        let cleanup = p.take("SubAbility");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        if let Some(name) = cleanup {
            let (api, mut p) = Params::parse(self.svars.get(&name)?)?;
            p.drop_prose();
            if api != "Cleanup"
                || p.take("ClearRemembered").as_deref() != Some("True")
                || !p.exhausted()
            {
                return self
                    .deny("a damage memory followed by something other than its cleanup".into());
            }
        }
        Some(())
    }

    /// The complete static restriction, returning only the filter it actually names.
    fn no_regeneration_filter(&self, name: &str) -> Option<String> {
        if name.is_empty() || name.contains(',') {
            return self.deny("a regeneration restriction naming multiple or no statics".into());
        }
        let (mode, mut p) = Params::parse(self.svars.get(name)?)?;
        p.drop_prose();
        let valid = p.take("ValidCard")?;
        if mode != "CantRegenerate" || !p.exhausted() {
            return self.deny(
                "a referenced static other than an unconditional regeneration restriction".into(),
            );
        }
        Some(valid)
    }

    /// An `Effect` line: a static ability that lasts the turn, read by the
    /// one sentence its static says. Every other static stays refused.
    pub(super) fn static_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.peek("ReplacementEffects").is_some() {
            return self.replace_target_white_mana(p, target);
        }
        if p.peek("Triggers").is_some() {
            return self.delayed_destroy_target_effect(p, target);
        }
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        let body = self.svars.get(&statics)?.clone();
        let (mode, _) = Params::parse(&body)?;
        match mode.as_str() {
            "CantChangeLife" => self.life_total_lock(p),
            "CantBlockBy" => self.unblockable_effect(p, target),
            "CantRegenerate" => self.cant_regenerate_effect(p, target),
            "AssignNoCombatDamage" => self.assign_no_combat_damage_effect(p),
            _ => None,
        }
    }

    /// "Assigns no combat damage this turn" (Laccolith Whelp, Laccolith Grunt,
    /// Laccolith Warrior, Laccolith Titan): an `Effect` whose one static is
    /// `Mode$ AssignNoCombatDamage | ValidCard$ Card.EffectSource`.
    fn assign_no_combat_damage_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if self.on_a_spell {
            return None;
        }
        let statics = p.take("StaticAbilities")?;
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "AssignNoCombatDamage"
            || st.take("ValidCard").as_deref() != Some("Card.EffectSource")
            || !st.exhausted()
        {
            return None;
        }
        if !matches!(
            p.take("Duration").as_deref(),
            None | Some("UntilHostLeavesPlayOrEOT" | "UntilEndOfCombat")
        ) {
            return None;
        }
        Some(vec![
            "Effect::ModifySource { modifier: Modifier::AssignsNoCombatDamage, \
             duration: Duration::UntilEndOfTurn }"
                .to_string(),
        ])
    }

    /// "Destroy target blocking creature at end of combat" (Silent Assassin):
    /// an `Effect` remembering `Targeted` with a `Triggers$` delayed trigger
    /// at `Phase$ EndCombat` that destroys `Remembered`.
    fn delayed_destroy_target_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let target = target.filter(|t| t.starts_with("TargetSpec::Object("))?;
        let trig_name = p.take("Triggers")?;
        if p.take("RememberObjects").as_deref() != Some("Targeted") {
            return None;
        }
        p.take("IsCurse");
        p.take("Name");
        let trig_body = self.svars.get(trig_name.trim())?.clone();
        let (mode, mut tp) = Params::parse(&trig_body)?;
        tp.drop_prose();
        if mode != "Phase" || tp.take("Phase").as_deref() != Some("EndCombat") {
            return None;
        }
        let exec_name = tp.take("Execute")?;
        if !tp.exhausted() {
            return None;
        }
        let exec_body = self.svars.get(exec_name.trim())?.clone();
        let (exec_api, mut ep) = Params::parse(&exec_body)?;
        ep.drop_prose();
        if exec_api != "Destroy"
            || ep.take("Defined").as_deref() != Some("Remembered")
            || !ep.exhausted()
        {
            return None;
        }
        Some(vec![format!(
            "Effect::AtEndOfCombat {{ about: {target}, \
             effects: &[Effect::destroy(TargetSpec::EventObject)] }}"
        )])
    }

    /// "It can't be regenerated this turn" (Disintegrate, Carbonize): an
    /// `Effect` whose one static says no regeneration applies to what it
    /// remembers, which is the line's target, and which ends when that
    /// leaves the battlefield — `Effect::CantBeRegeneratedThisTurn`, kept
    /// for that object only (CR 400.7, 701.19c).
    ///
    /// "If it's a creature" (`ConditionDefined$ ParentTarget |
    /// ConditionPresent$ Creature`) is asked of the target as the line
    /// resolves. Actual damage-recipient memory is the separate complete
    /// chain read by [`Self::damage_recipient_regeneration`].
    pub(super) fn cant_regenerate_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let statics = p.take("StaticAbilities")?;
        if self.no_regeneration_filter(statics.trim())? != "Card.IsRemembered" {
            return None;
        }
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if ends.as_deref() != Some("Battlefield") || p.take("Duration").is_some() {
            return None;
        }
        p.take("IsCurse");
        let target = target.filter(|t| {
            !matches!(
                *t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        })?;
        if !matches!(
            p.take("RememberObjects").as_deref(),
            Some("Targeted" | "ParentTarget")
        ) {
            return None;
        }
        let effect = format!("Effect::CantBeRegeneratedThisTurn {{ target: {target} }}");
        let creature = match (p.peek("ConditionDefined"), p.peek("ConditionPresent")) {
            (None, None) => false,
            (Some("ParentTarget" | "Targeted"), Some("Creature")) => {
                p.take("ConditionDefined");
                p.take("ConditionPresent");
                true
            }
            _ => return None,
        };
        Some(vec![if creature {
            format!("Effect::IfTargetMatches {{ filter: &Filter::CREATURE, then: &[{effect}] }}")
        } else {
            effect
        }])
    }
}
