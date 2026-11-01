//! Narrow count restriction and external entry replacement readers.
use super::{Params, Tx};
impl Tx<'_> {
    pub(super) fn one_spell_each_turn(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if let Some(zone) = p.take("EffectZone")
            && zone != "Battlefield"
        {
            return self.deny(format!("a spell quota from `{zone}`"));
        }
        if p.peek("Caster") == Some("Opponent") && !p.has("NumLimitEachTurn") {
            p.take("Caster");
            let Some(valid) = p.take("ValidCard") else {
                return self.deny("an opponent cast prohibition without `ValidCard$`".into());
            };
            if !p.exhausted() {
                return self.deny("an opponent cast prohibition with extra clauses".into());
            }
            let expr = self.filter_expr(&valid)?;
            let name = self.body.filter_static("CANT_CAST", &expr);
            self.body.abilities.push(Self::static_expr(
                "Filter::Any",
                &format!("Modifier::OpponentsCantCast(&{name})"),
            ));
            return Some(());
        }
        if p.take("ValidCard").as_deref() != Some("Card")
            || p.take("Caster").as_deref() != Some("Player")
            || p.take("NumLimitEachTurn").as_deref() != Some("1")
            || !p.exhausted()
        {
            return self.deny("a spell quota other than each player one each turn".into());
        }
        self.body.abilities.push(Self::static_expr(
            "Filter::Any",
            "Modifier::SpellCastLimitPerTurn { who: PlayerRel::EachPlayer, limit: 1 }",
        ));
        Some(())
    }
    pub(super) fn external_enter_tapped(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let valid = p.take("ValidCard")?;
        if !matches!(
            valid.as_str(),
            "Land.OppCtrl+nonBasic" | "Artifact.OppCtrl,Creature.OppCtrl,Land.OppCtrl"
        ) || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("ActiveZones").as_deref() != Some("Battlefield")
            || p.take("ReplacementResult").as_deref() != Some("Updated")
        {
            return self
                .deny("an external entry replacement outside the supported opponent kinds".into());
        }
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("an external entry replacement with no child".into());
        };
        if !p.exhausted() {
            return self.deny("unclaimed external entry replacement parameter".into());
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|text| Params::parse(text))
        else {
            return self.deny("an unreadable external entry replacement child".into());
        };
        body.drop_prose();
        if api != "Tap"
            || body.take("ETB").as_deref() != Some("True")
            || body.take("Defined").as_deref() != Some("ReplacedCard")
            || !body.exhausted()
        {
            return self
                .deny("external entry replacement must tap only the replaced entrant".into());
        }
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("ENTER", &expr);
        self.body.abilities.push(format!(
            "AbilityDef::Replacement(ReplacementRule::EnterTapped(&{filter}))"
        ));
        Some(())
    }
}
