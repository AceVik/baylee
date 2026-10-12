//! Narrow player protection/life-lock readers, retaining unknown-key refusal.
use super::{Params, Tx};
impl Tx<'_> {
    pub(super) fn everything_protection(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.take("Defined").as_deref() != Some("You")
            || p.take("Duration").as_deref() != Some("UntilYourNextTurn")
            || p.take("KW").as_deref() != Some("Protection from everything")
        {
            return self
                .deny("player protection outside the exact self/until-next-turn shape".into());
        }
        Some(vec!["Effect::continuous(&Filter::Any, Modifier::PlayerProtectionFromEverything { who: PlayerRel::You }, Duration::UntilYourNextTurn)".into()])
    }
    pub(super) fn life_total_lock(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.take("Duration").as_deref() != Some("UntilYourNextTurn") {
            return self.deny("life lock with an unsupported duration".into());
        }
        let name = p.take("StaticAbilities")?;
        let (mode, mut child) = self.svars.get(&name).and_then(|s| Params::parse(s))?;
        child.drop_prose();
        if mode != "CantChangeLife"
            || child.take("ValidPlayer").as_deref() != Some("You")
            || !child.exhausted()
        {
            return self.deny("life lock outside the exact self-player static".into());
        }
        Some(vec!["Effect::continuous(&Filter::Any, Modifier::LifeTotalCantChange { who: PlayerRel::You }, Duration::UntilYourNextTurn)".into()])
    }
    pub(super) fn phase_your_permanents(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.take("Defined").as_deref() != Some("Valid Permanent.YouCtrl") {
            return self
                .deny("phase-out other than the simultaneous controlled permanent set".into());
        }
        Some(vec![
            "Effect::PhaseOutFilter(&Filter::ControlledByYou)".into(),
        ])
    }
    pub(super) fn cast_self_protection_trigger(&mut self, mode: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        if mode != "ChangesZone"
            || p.take("ValidCard").as_deref() != Some("Card.wasCastByYou+Self")
            || p.take("Destination").as_deref() != Some("Battlefield")
        {
            return self.deny("unsupported cast-self entry trigger".into());
        }
        if let Some(origin) = p.take("Origin")
            && origin != "Any"
        {
            return self.deny("cast-self entry from a restricted origin".into());
        }
        if let Some(zone) = p.take("TriggerZones")
            && zone != "Battlefield"
        {
            return self.deny("cast-self trigger outside battlefield".into());
        }
        let name = p.take("Execute")?;
        if !p.exhausted() {
            return self.deny("unclaimed cast-self entry parameter".into());
        }
        let (api, mut child) = self.svars.get(&name).and_then(|s| Params::parse(s))?;
        child.drop_prose();
        if api != "Pump" {
            return self.deny("cast-self entry child is not protection".into());
        }
        let effects = self.everything_protection(&mut child)?;
        if !child.exhausted() {
            return self.deny("unclaimed cast-self protection child parameter".into());
        }
        self.body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[{}], condition = Some(Condition::SourceWasCastByYou))",
            effects.join(", ")
        ));
        Some(())
    }
}
