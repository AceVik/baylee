//! Persistent prevention whose sources are explicitly instant or sorcery spells.

use super::{Params, Tx};

impl Tx<'_> {
    /// All damage from matching spell sources, without a recipient or combat limit.
    pub(super) fn spell_damage_prevention(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if !matches!(p.take("ActiveZones").as_deref(), None | Some("Battlefield"))
            || p.take("Prevent").as_deref() != Some("True")
        {
            return self.deny("a persistent spell prevention with another zone or result".into());
        }
        let valid = p.take("ValidSource")?;
        // A creature source could be a permanent or an ability. This reader
        // promises spell sources, so every alternative must name one of the
        // two nonpermanent spell types before its ordinary filter clauses.
        if !valid.split(',').all(|alternative| {
            matches!(
                alternative.trim().split(['.', '+']).next(),
                Some("Instant" | "Sorcery")
            )
        }) {
            return self.deny(format!(
                "a prevention source outside instant/sorcery spells `{valid}`"
            ));
        }
        let filter = self.filter_expr(&valid)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        self.body.abilities.push(format!(
            "static_ability!(Filter::And(&[Filter::InZone(ZoneRef::Stack), {filter}]), \
             Modifier::PreventAllDamageFromIt)"
        ));
        Some(())
    }
}
