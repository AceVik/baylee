//! Exact readers for the Titania's Song whole-card semantics.
use super::{CardScript, Params, Tx};

/// Candidate is deliberately broad enough to refuse a missing departure half.
pub(super) fn song_candidate(script: &CardScript) -> bool {
    script.rules.iter().any(|(kind, spec)| {
        *kind == 'S'
            && Params::parse(spec).is_some_and(|(mode, p)| {
                mode == "Continuous"
                    && p.peek("Affected") == Some("Artifact.nonCreature")
                    && p.peek("RemoveAllAbilities") == Some("True")
                    && p.peek("SetPower") == Some("AffectedX")
            })
    })
}
impl Tx<'_> {
    /// Read both Song clauses together; Static True is not a triggered ability.
    pub(super) fn song_whole_card(&mut self, script: &CardScript) -> Option<()> {
        if !script.keywords.is_empty() || script.rules.len() != 2 {
            return None;
        }
        let (_, static_spec) = script.rules.iter().find(|(k, _)| *k == 'S')?;
        let (mode, mut st) = Params::parse(static_spec)?;
        st.drop_prose();
        if mode != "Continuous"
            || st.take("Affected").as_deref() != Some("Artifact.nonCreature")
            || st.take("AddType").as_deref() != Some("Artifact & Creature")
            || st.take("RemoveCardTypes").as_deref() != Some("True")
            || st.take("RemoveAllAbilities").as_deref() != Some("True")
            || !self.song_power(&mut st)
            || !st.exhausted()
        {
            return None;
        }
        let (_, departure) = script.rules.iter().find(|(k, _)| *k == 'T')?;
        let (mode, mut leave) = Params::parse(departure)?;
        leave.drop_prose();
        if mode != "ChangesZone"
            || leave.take("Origin").as_deref() != Some("Battlefield")
            || leave.take("Destination").as_deref() != Some("Any")
            || leave.take("ValidCard").as_deref() != Some("Card.Self")
            || leave.take("Static").as_deref() != Some("True")
        {
            return None;
        }
        let name = leave.take("Execute")?;
        if !leave.exhausted() {
            return None;
        }
        let (api, mut effect) = Params::parse(self.svars.get(&name)?)?;
        effect.drop_prose();
        if api != "Effect" {
            return None;
        }
        let name = effect.take("StaticAbilities")?;
        if !effect.exhausted() {
            return None;
        }
        let (mode, mut child) = Params::parse(self.svars.get(&name)?)?;
        child.drop_prose();
        if mode != "Continuous"
            || child.take("AffectedZone").as_deref() != Some("Battlefield")
            || child.take("Affected").as_deref() != Some("Artifact.nonCreature")
            || child.take("AddType").as_deref() != Some("Creature")
            || child.take("RemoveAllAbilities").as_deref() != Some("True")
            || !self.song_power(&mut child)
            || !child.exhausted()
        {
            return None;
        }
        self.body.abilities.push(Self::static_expr(
            "Filter::ARTIFACT",
            "Modifier::AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture",
        ));
        Some(())
    }
    /// The named value is mana value, not an announced X or card count.
    fn song_power(&self, p: &mut Params) -> bool {
        p.take("SetPower").as_deref() == Some("AffectedX")
            && p.take("SetToughness").as_deref() == Some("AffectedX")
            && self.svars.get("AffectedX").map(String::as_str) == Some("Count$CardManaCost")
    }
}
