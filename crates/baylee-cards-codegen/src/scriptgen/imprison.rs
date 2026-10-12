//! Exact enchanted-creature payment branches; no broad unless-chain fallback.
use super::{Params, Tx};
impl Tx<'_> {
    pub(super) fn is_imprison_clause(&self, mode: &str, p: &Params) -> bool {
        if mode == "AbilityCast" && p.peek("ValidCard") == Some("Creature.EnchantedBy") {
            return true;
        }
        matches!(mode, "Attacks" | "Blocks")
            && p.peek("ValidCard") == Some("Card.EnchantedBy")
            && p.peek("Execute")
                .and_then(|name| self.svars.get(name))
                .and_then(|body| Params::parse(body))
                .is_some_and(|(_, child)| child.has("UnlessResolveSubs"))
    }
    pub(super) fn imprison_clause(&mut self, mode: &str, mut p: Params) -> Option<()> {
        let (valid, trigger) = match mode {
            "AbilityCast" => (
                "Creature.EnchantedBy",
                "Trigger::AbilityActivated { filter: &Filter::AttachedToBySource, who: PlayerRel::EachPlayer, tap_cost: Some(true), mana_ability: Some(false) }",
            ),
            "Attacks" => (
                "Card.EnchantedBy",
                "Trigger::Attacks(&Filter::AttachedToBySource)",
            ),
            "Blocks" => (
                "Card.EnchantedBy",
                "Trigger::Blocks(&Filter::AttachedToBySource)",
            ),
            _ => return self.deny("unsupported enchanted-creature payment trigger".into()),
        };
        if p.take("ValidCard").as_deref() != Some(valid)
            || p.take("TriggerZones").as_deref() != Some("Battlefield")
        {
            return self.deny("enchanted-creature payment trigger has wrong source or zone".into());
        }
        if mode == "AbilityCast"
            && p.take("ValidSA").as_deref() != Some("SpellAbility.!ManaAbility+hasTapCost")
        {
            return self
                .deny("enchanted activation requires nonmana ability with a tap symbol".into());
        }
        if mode == "Blocks" && p.take("Secondary").as_deref() != Some("True") {
            return self.deny("unsupported secondary enchanted block clause".into());
        }
        let Some(execute) = p.take("Execute") else {
            return self.deny("enchanted-creature payment trigger has no effect".into());
        };
        if !p.exhausted() {
            return self.deny("unclaimed enchanted-creature trigger parameter".into());
        }
        let child = self.imprison_unpaid(&execute)?;
        let paid = self.imprison_paid(&child, mode)?;
        self.body.abilities.push(format!(
            "triggered!({trigger}, &[Effect::PlayerMayPayThenElse {{ player: PlayerRel::You, mana: Amount::Fixed(1), paid: &[{paid}], unpaid: &[Effect::destroy(TargetSpec::ThisObject)] }}])"
        ));
        Some(())
    }
    fn imprison_unpaid(&mut self, name: &str) -> Option<String> {
        let Some((api, mut p)) = self.svars.get(name).and_then(|text| Params::parse(text)) else {
            return self.deny("unreadable enchanted-creature payment".into());
        };
        p.drop_prose();
        if api != "Destroy"
            || p.take("Defined").as_deref() != Some("Self")
            || p.take("UnlessCost").as_deref() != Some("1")
            || p.take("UnlessPayer").as_deref() != Some("You")
            || p.take("UnlessResolveSubs").as_deref() != Some("WhenPaid")
        {
            return self.deny(
                "enchanted-creature payment must destroy this Aura unless its controller pays one"
                    .into(),
            );
        }
        let child = p.take("SubAbility")?;
        if !p.exhausted() {
            return self.deny("unclaimed enchanted-creature payment parameter".into());
        }
        Some(child)
    }
    fn imprison_paid(&mut self, name: &str, mode: &str) -> Option<String> {
        let Some((api, mut p)) = self.svars.get(name).and_then(|text| Params::parse(text)) else {
            return self.deny("unreadable paid enchanted-creature branch".into());
        };
        p.drop_prose();
        if mode == "AbilityCast" {
            if api == "Counter"
                && p.take("Defined").as_deref() == Some("TriggeredSpellAbility")
                && p.exhausted()
            {
                return Some("Effect::CounterEventAbility".into());
            }
            return self.deny(
                "enchanted-creature payment must counter exactly the triggering ability".into(),
            );
        }
        let role = if mode == "Attacks" {
            "TriggeredAttackerLKICopy"
        } else {
            "TriggeredBlockerLKICopy"
        };
        if api != "Tap" || p.take("Defined").as_deref() != Some(role) {
            return self.deny("enchanted combat payment must tap the event creature".into());
        }
        let child = p.take("SubAbility")?;
        if !p.exhausted() {
            return self.deny("unclaimed enchanted combat tap parameter".into());
        }
        self.imprison_remove(&child, role, mode == "Blocks")?;
        Some("Effect::TapEventObject, Effect::RemoveEventObjectFromCombat { unblock: true }".into())
    }
    fn imprison_remove(&mut self, name: &str, role: &str, blocker: bool) -> Option<()> {
        let Some((api, mut p)) = self.svars.get(name).and_then(|text| Params::parse(text)) else {
            return self.deny("unreadable enchanted combat removal".into());
        };
        p.drop_prose();
        if api != "RemoveFromCombat"
            || p.take("Defined").as_deref() != Some(role)
            || (blocker
                && p.take("UnblockCreaturesBlockedOnlyBy").as_deref() != Some("TriggeredBlocker"))
            || !p.exhausted()
        {
            return self
                .deny("enchanted combat removal must preserve the exact sole-blocker rule".into());
        }
        Some(())
    }
}
