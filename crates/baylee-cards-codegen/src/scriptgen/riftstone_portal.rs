//! Exact graveyard-source land mana grant, retaining all unknown-key guards.
use super::{Params, Tx};
impl Tx<'_> {
    pub(super) fn riftstone_graveyard_grant(&mut self, mode: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        if mode != "Continuous"
            || p.take("EffectZone").as_deref() != Some("Graveyard")
            || p.take("Affected").as_deref() != Some("Land.YouCtrl")
        {
            return self.deny("unsupported graveyard-source land grant".into());
        }
        let name = p.take("AddAbility")?;
        if !p.exhausted() {
            return self.deny("unclaimed graveyard grant parameter".into());
        }
        let (api, mut child) = Params::parse(self.svars.get(&name)?)?;
        child.drop_prose();
        if api != "Mana"
            || child.take("Cost").as_deref() != Some("T")
            || child.take("Produced").as_deref() != Some("Combo G W")
        {
            return self.deny("graveyard grant child outside exact tap-for-green-or-white".into());
        }
        if let Some(amount) = child.take("Amount")
            && amount != "1"
        {
            return self.deny("graveyard mana grant amount other than one".into());
        }
        if !child.exhausted() {
            return self.deny("unclaimed graveyard mana child parameter".into());
        }
        self.body.abilities.push("static_ability!(Filter::YOUR_LAND, Modifier::GrantActivated { cost: Cost::TAP, effects: &[Effect::mana_choice(&[ManaColor::Green, ManaColor::White])], mana_ability: true }, zone = ZoneRef::Graveyard)".into());
        Some(())
    }
}
