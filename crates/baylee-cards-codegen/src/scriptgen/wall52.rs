//! Exact Wall-only targeting and blocked-source damage readers.
use super::{Params, Tx};
impl Tx<'_> {
    /// Wall-only means the permission domain, never today's legal target count.
    pub(super) fn wall_only_target_prohibition(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidTarget").as_deref() != Some("Card.Self") {
            return None;
        }
        if p.peek("SourceCanOnlyTarget").is_some() {
            if p.take("SourceCanOnlyTarget").as_deref() != Some("Wall") || !p.exhausted() {
                return None;
            }
            self.body.abilities.push(Self::static_expr(
                "Filter::This",
                "Modifier::CantBeTargetedByWallOnly",
            ));
            return Some(());
        }
        let source = p.take("ValidSource")?;
        if !p.exhausted() {
            return None;
        }
        let filter = self.filter_expr(&source)?;
        let name = self.body.filter_static("SOURCE", &filter);
        self.body.abilities.push(Self::static_expr(
            "Filter::This",
            &format!("Modifier::CantBeTargetedBy(&{name})"),
        ));
        Some(())
    }
    /// All damage from the creature the permanent currently blocks.
    pub(super) fn wall_blocked_damage_prevention(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Prevent").as_deref() != Some("True")
            || p.take("ValidSource").as_deref() != Some("Creature.blockedBySource")
            || p.take("ValidTarget").as_deref() != Some("Creature.Self")
            || !p.exhausted()
        {
            return None;
        }
        self.body.abilities.push(Self::static_expr(
            "Filter::This",
            "Modifier::PreventDamageFrom(&Filter::BlockedBySource)",
        ));
        Some(())
    }
    /// All damage from the creature this Aura enchants (Muzzle).
    pub(super) fn enchanted_source_damage_prevention(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        match p.take("ActiveZones").as_deref() {
            None | Some("Battlefield") => {}
            Some(_) => return None,
        }
        if p.take("Prevent").as_deref() != Some("True")
            || p.take("ValidSource").as_deref() != Some("Creature.EnchantedBy")
            || !p.exhausted()
        {
            return None;
        }
        let filter = self.filter_expr("Creature.EnchantedBy")?;
        self.body.abilities.push(Self::static_expr(
            &filter,
            "Modifier::PreventAllDamageFromIt",
        ));
        Some(())
    }
    /// All damage that would be dealt to the creature this Aura enchants (Inviolability).
    pub(super) fn enchanted_target_damage_prevention(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        match p.take("ActiveZones").as_deref() {
            None | Some("Battlefield") => {}
            Some(_) => return None,
        }
        if p.take("Prevent").as_deref() != Some("True")
            || p.take("ValidTarget").as_deref() != Some("Creature.EnchantedBy")
            || !p.exhausted()
        {
            return None;
        }
        let filter = self.filter_expr("Creature.EnchantedBy")?;
        self.body.abilities.push(Self::static_expr(
            &filter,
            "Modifier::PreventDamageFrom(&Filter::Any)",
        ));
        Some(())
    }
    /// Combat damage dealt to or by creatures you control (Statecraft).
    pub(super) fn combat_damage_your_creatures_prevention(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        match p.take("ActiveZones").as_deref() {
            None | Some("Battlefield") => {}
            Some(_) => return None,
        }
        match p.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return None,
        }
        if p.take("Prevent").as_deref() != Some("True")
            || p.take("IsCombat").as_deref() != Some("True")
        {
            return None;
        }
        let modifier = match (
            p.take("ValidTarget").as_deref(),
            p.take("ValidSource").as_deref(),
        ) {
            (Some("Creature.YouCtrl"), None) => "Modifier::PreventDamageToIt",
            (None, Some("Creature.YouCtrl")) => "Modifier::PreventDamageFromIt",
            _ => return None,
        };
        if !p.exhausted() {
            return None;
        }
        self.body
            .abilities
            .push(Self::static_expr("Filter::YOUR_CREATURE", modifier));
        Some(())
    }
}
