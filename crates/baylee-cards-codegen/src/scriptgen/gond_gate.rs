//! Exact Gate entry and reflected-colour grammar, with exhaustive parameter refusal.
use super::{Params, Tx};
impl Tx<'_> {
    pub(super) fn gond_entry(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCard").as_deref() != Some("Gate.YouCtrl")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ReplacementResult").as_deref() != Some("Updated")
            || p.take("ActiveZones").as_deref() != Some("Battlefield")
        {
            return self.deny("unsupported Gate entry replacement shape".into());
        }
        let name = p.take("ReplaceWith")?;
        if !p.exhausted() {
            return self.deny("unclaimed Gate entry parameter".into());
        }
        let (api, mut child) = Params::parse(self.svars.get(&name)?)?;
        child.drop_prose();
        if api != "Untap"
            || child.take("ETB").as_deref() != Some("True")
            || child.take("Defined").as_deref() != Some("ReplacedCard")
            || !child.exhausted()
        {
            return self.deny("Gate entry must untap exactly the replaced entrant".into());
        }
        let filter = self.filter_expr("Gate.YouCtrl")?;
        let filter = self.body.filter_static("GATE_ENTRY", &filter);
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::EnterUntapped(&{filter}))"
        ));
        Some(())
    }
    pub(super) fn gond_mana_reflected(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Cost").as_deref() != Some("T")
            || p.take("ColorOrType").as_deref() != Some("Color")
            || p.take("ReflectProperty").as_deref() != Some("Produce")
        {
            return self.deny("reflected mana outside the exact your-Gate colours grammar".into());
        }
        let effect = match p.take("Valid").as_deref() {
            Some("Gate.YouCtrl") => {
                "Effect::mana_from_lands(&Filter::HasSubtype(subtypes::land::GATE), true, false)"
            }
            Some("Basic.YouCtrl") => "Effect::mana_from_lands(&Filter::BASIC_LAND, true, false)",
            Some("Land.OppCtrl") => "Effect::mana_land_color(false)",
            _ => {
                return self
                    .deny("reflected mana outside the exact your-Gate colours grammar".into());
            }
        };
        if let Some(amount) = p.take("Amount")
            && amount != "1"
        {
            return self.deny("reflected Gate amount other than one".into());
        }
        if !p.exhausted() {
            return self.deny("unclaimed reflected Gate mana parameter".into());
        }
        self.body
            .abilities
            .push(format!("mana_ability!(&[{effect}])"));
        Some(())
    }
}
