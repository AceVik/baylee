//! Exact new-Aura and hidden control permission readers.
use super::{Params, Tx};
pub(super) fn candidate(mode: &str, p: &Params) -> bool {
    mode == "CantAttach"
        || (mode == "Continuous"
            && p.peek("AddHiddenKeyword") == Some("Other players can't gain control of CARDNAME."))
}
impl Tx<'_> {
    pub(super) fn guardian_permission(&mut self, mode: &str, p: Params) -> Option<()> {
        if mode == "CantAttach" {
            self.guardian_new_aura_prohibition(p)
        } else {
            self.guardian_indestructible_control(p)
        }
    }

    /// New attachment gate, intentionally distinct from ongoing Aura legality.
    pub(super) fn guardian_new_aura_prohibition(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("ValidCardToTarget").as_deref() != Some("Aura.!Attached")
            || p.take("Target").as_deref() != Some("Artifact.nonCreature+YouCtrl")
            || p.take("IsPresent").as_deref() != Some("Card.Self+untapped")
            || !p.exhausted()
        {
            return None;
        }
        self.body.abilities.push("static_ability!(Filter::And(&[Filter::ARTIFACT, Filter::NONCREATURE, Filter::ControlledByYou]), Modifier::CantBecomeEnchanted, condition = Some(Condition::SourceMatches(&Filter::Untapped)))".into());
        Some(())
    }
    /// Do not discard the hidden control sentence while reading indestructible.
    pub(super) fn guardian_indestructible_control(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        if p.take("Affected").as_deref() != Some("Artifact.nonCreature+YouCtrl")
            || p.take("AddKeyword").as_deref() != Some("Indestructible")
            || p.take("AddHiddenKeyword").as_deref()
                != Some("Other players can't gain control of CARDNAME.")
            || p.take("IsPresent").as_deref() != Some("Card.Self+untapped")
            || !p.exhausted()
        {
            return None;
        }
        for modifier in [
            "Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE)",
            "Modifier::OtherPlayersCantGainControl",
        ] {
            self.body.abilities.push(format!("static_ability!(Filter::And(&[Filter::ARTIFACT, Filter::NONCREATURE, Filter::ControlledByYou]), {modifier}, condition = Some(Condition::SourceMatches(&Filter::Untapped)))"));
        }
        Some(())
    }
}
