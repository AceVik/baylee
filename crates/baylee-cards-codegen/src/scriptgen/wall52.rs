//! Exact Wall-only targeting and blocked-source damage readers.
use super::{Params, Tx};
impl Tx<'_> {
    /// Wall-only means the permission domain, never today's legal target count.
    pub(super) fn wall_only_target_prohibition(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidTarget").as_deref() != Some("Card.Self")
            || p.take("SourceCanOnlyTarget").as_deref() != Some("Wall")
            || !p.exhausted()
        {
            return None;
        }
        self.body.abilities.push(Self::static_expr(
            "Filter::This",
            "Modifier::CantBeTargetedByWallOnly",
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
}
