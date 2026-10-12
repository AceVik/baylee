//! Narrow complete rules shapes for Bronze Horse and Johan.
use super::{Params, Tx};
impl Tx<'_> {
    /// Damage source must be a spell whose selected targets include this incarnation.
    pub(super) fn bronze_horse_prevention(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Prevent").as_deref() != Some("True")
            || p.take("ValidCause").as_deref() != Some("Spell.IsTargeting Self")
            || p.take("ValidTarget").as_deref() != Some("Creature.Self")
            || p.take("CauseIsSource").as_deref() != Some("True")
            || p.take("IsPresent").as_deref() != Some("Creature.YouCtrl+Other")
            || !p.exhausted()
        {
            return None;
        }
        // ControlCount rebases This to each counted candidate, so Another
        // must not be put into that filter. Count whether this object itself
        // is a creature and choose the corresponding exact lower bound.
        self.body.abilities.push("static_ability!(Filter::This, Modifier::PreventDamageFrom(&Filter::SpellTargetingAffected), condition = Some(Condition::Any(&[Condition::All(&[Condition::SourceMatches(&Filter::CREATURE), Condition::ControlCount(&Filter::CREATURE, 2)]), Condition::All(&[Condition::Not(&Condition::SourceMatches(&Filter::CREATURE)), Condition::ControlCount(&Filter::CREATURE, 1)])])))".into());
        Some(())
    }
    /// Whole optional combat chain; cannot read an isolated vigilance-looking half.
    pub(super) fn johan_combat_trigger(&mut self, mode: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        if mode != "Phase"
            || p.take("Phase").as_deref() != Some("BeginCombat")
            || p.take("ValidPlayer").as_deref() != Some("You")
            || p.take("TriggerZones").as_deref() != Some("Battlefield")
            || p.take("OptionalDecider").as_deref() != Some("You")
        {
            return None;
        }
        let name = p.take("Execute")?;
        if !p.exhausted() {
            return None;
        }
        let (api, mut pump) = Params::parse(self.svars.get(&name)?)?;
        pump.drop_prose();
        if api != "Pump"
            || pump.take("Defined").as_deref() != Some("Self")
            || pump.take("KW").as_deref() != Some("CARDNAME can't attack.")
            || pump.take("Duration").as_deref() != Some("UntilEndOfCombat")
        {
            return None;
        }
        let name = pump.take("SubAbility")?;
        if !pump.exhausted() {
            return None;
        }
        let (api, mut effect) = Params::parse(self.svars.get(&name)?)?;
        effect.drop_prose();
        if api != "Effect" || effect.take("Duration").as_deref() != Some("UntilEndOfCombat") {
            return None;
        }
        let name = effect.take("StaticAbilities")?;
        if !effect.exhausted() {
            return None;
        }
        let (mode, mut rule) = Params::parse(self.svars.get(&name)?)?;
        rule.drop_prose();
        if mode != "AttackVigilance"
            || rule.take("ValidCard").as_deref() != Some("Creature.YouCtrl")
            || rule.take("IsPresent").as_deref() != Some("Card.EffectSource+untapped")
            || !rule.exhausted()
        {
            return None;
        }
        self.body.abilities.push("triggered!(Trigger::StepBegin { step: StepKind::CombatBegin, whose: PlayerRel::You }, &[Effect::MayDo { effects: &[Effect::continuous(&Filter::This, Modifier::AddKeyword(KeywordSet::CANT_ATTACK), Duration::UntilEndOfCombat), Effect::AttackWithoutTappingWhileSourceUntappedThisCombat] }])".into());
        Some(())
    }
}
