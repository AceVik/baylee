//! Exact linked/transform/provenance sentences for the remaining Batch48 trio.
use super::{Params, Tx};
impl Tx<'_> {
    pub(super) fn is_transform48_trigger(&self, p: &Params) -> bool {
        let special_child = p
            .peek("Execute")
            .and_then(|name| self.svars.get(name))
            .and_then(|body| Params::parse(body))
            .is_some_and(|(api, child)| {
                (api == "Discard" && child.peek("AnyNumber") == Some("True"))
                    || (api == "Token"
                        && child.peek("TokenScript")
                            == Some("w_x_x_a_gnome_soldier_total_artifacts_creatures"))
                    || (api == "SetState"
                        && child.peek("Cost")
                            == Some("tapXType<5/Artifact;Creature/artifact or creature>"))
                    || (api == "ChangeZone" && child.peek("RememberTargets") == Some("True"))
            });
        special_child
            || p.peek("ValidSA")
                .is_some_and(|sa| sa.contains("ManaFromCard"))
            || p.peek("ValidPlayer") == Some("You.descended")
            || p.peek("CheckSVar")
                .and_then(|name| self.svars.get(name))
                .is_some_and(|s| s.contains("SacrificedThisTurn Clue"))
    }

    fn transform48_node(&self, name: &str, api: &str, keys: &[(&str, &str)]) -> Option<Params> {
        let (actual, mut p) = Params::parse(self.svars.get(name)?)?;
        p.drop_prose();
        if actual != api {
            return None;
        }
        for &(key, value) in keys {
            if p.take(key).as_deref() != Some(value) {
                return None;
            }
        }
        Some(p)
    }
    fn transform48_end(&self, name: &str) -> Option<()> {
        let p = self.transform48_node(
            name,
            "SetState",
            &[("Defined", "Self"), ("Mode", "Transform")],
        )?;
        p.exhausted().then_some(())
    }
    pub(super) fn transform48_trigger(&mut self, mode: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        let execute = p.take("Execute")?;
        let output = match mode {
            "SpellCast" => self.transform48_source_mana(&mut p, &execute)?,

            "Phase" => self.transform48_phase(&mut p, &execute)?,

            "Transformed" => {
                if p.take("ValidCard").as_deref() != Some("Card.Self") {
                    return None;
                }
                let child = self.transform48_node(
                    &execute,
                    "ChangeZone",
                    &[
                        ("ValidTgts", "Creature.YouOwn"),
                        ("Origin", "Graveyard"),
                        ("Destination", "Battlefield"),
                        ("RememberTargets", "True"),
                    ],
                )?;
                if !child.exhausted() {
                    return None;
                }
                "triggered!(Trigger::TransformsIntoThis, &[Effect::GraveyardToBattlefieldLinked { target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You) }], targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You))))".into()
            }
            "ChangesZone" => self.transform48_entry(&mut p, &execute)?,

            _ => return None,
        };
        if !p.exhausted() {
            return None;
        }
        self.body.abilities.push(output);
        Some(())
    }
    fn transform48_source_mana(&self, p: &mut Params, execute: &str) -> Option<String> {
        Some({
            if p.take("ValidSA").as_deref() != Some("Spell.ManaFromCard.StrictlySelf")
                || p.take("ValidActivatingPlayer").as_deref() != Some("You")
            {
                return None;
            }
            if let Some(zone) = p.take("TriggerZones")
                && zone != "Battlefield"
            {
                return None;
            }
            let filter = match p.take("ValidCard").as_deref()? {
                "Permanent" => {
                    "Filter::HasType(TypeSet::ARTIFACT.union(TypeSet::BATTLE).union(TypeSet::CREATURE).union(TypeSet::ENCHANTMENT).union(TypeSet::LAND).union(TypeSet::PLANESWALKER))"
                }
                "Artifact,Creature" => "Filter::ARTIFACT_OR_CREATURE",
                _ => return None,
            };
            let effect = if filter == "Filter::ARTIFACT_OR_CREATURE" {
                let child = self.transform48_node(
                    execute,
                    "Token",
                    &[(
                        "TokenScript",
                        "w_x_x_a_gnome_soldier_total_artifacts_creatures",
                    )],
                )?;
                if !child.exhausted() {
                    return None;
                }
                "Effect::CreateToken { token: &generated_tokens::GNOME_SOLDIER_WHITE_DYNAMIC }"
            } else {
                let child = self.transform48_node(
                    execute,
                    "Discover",
                    &[("Num", "TriggeredSpellAbility$CardManaCostLKI")],
                )?;
                if !child.exhausted() {
                    return None;
                }
                "Effect::DiscoverAmount { mana_value: Amount::EventManaValue }"
            };
            format!("triggered!(Trigger::SpellCastUsingSourceMana(&{filter}), &[{effect}])")
        })
    }
    fn transform48_phase(&self, p: &mut Params, execute: &str) -> Option<String> {
        Some({
            if p.take("TriggerZones").as_deref() != Some("Battlefield") {
                return None;
            }
            match (p.take("Phase").as_deref(), p.take("ValidPlayer").as_deref()) {
                (Some("Main1"), Some("You")) => {
                    let child = self.transform48_node(
                        execute,
                        "SetState",
                        &[
                            ("Cost", "tapXType<5/Artifact;Creature/artifact or creature>"),
                            ("Defined", "Self"),
                            ("Mode", "Transform"),
                        ],
                    )?;
                    if !child.exhausted() {
                        return None;
                    }
                    "triggered!(Trigger::StepBegin { step: StepKind::FirstMain, whose: PlayerRel::You }, &[Effect::PlayerMayPayCostThen { player: PlayerRel::You, cost: &CostPart::TapN { filter: &Filter::And(&[Filter::ARTIFACT_OR_CREATURE, Filter::ControlledByYou]), count: 5 }, effect: &Effect::TransformSource }])".into()
                }
                (Some("End of Turn"), Some("You")) => {
                    let name = p.take("CheckSVar")?;
                    if self.svars.get(&name)?.trim()
                        != "PlayerCountPropertyYou$SacrificedThisTurn Clue"
                        || p.take("SVarCompare").as_deref() != Some("GE3")
                    {
                        return None;
                    }
                    self.transform48_end(execute)?;
                    "triggered!(Trigger::StepBegin { step: StepKind::End, whose: PlayerRel::You }, &[Effect::TransformSource], condition = Some(Condition::SacrificedSubtypeThisTurn { subtype: subtypes::artifact::CLUE, count: 3 }))".into()
                }
                (Some("End of Turn"), Some("You.descended")) => {
                    let mut put =
                        self.transform48_node(execute, "PutCounter", &[("CounterType", "BORE")])?;
                    let branch = put.take("SubAbility")?;
                    if !put.exhausted() {
                        return None;
                    }
                    let mut branch = self.transform48_node(
                        &branch,
                        "Branch",
                        &[
                            ("BranchConditionSVar", "Count$CardCounters.BORE"),
                            ("BranchConditionSVarCompare", "GE3"),
                        ],
                    )?;
                    let remove = branch.take("TrueSubAbility")?;
                    if !branch.exhausted() {
                        return None;
                    }
                    let mut remove = self.transform48_node(
                        &remove,
                        "RemoveCounter",
                        &[("CounterType", "BORE"), ("CounterNum", "All")],
                    )?;
                    let end = remove.take("SubAbility")?;
                    if !remove.exhausted() {
                        return None;
                    }
                    self.transform48_end(&end)?;
                    "triggered!(Trigger::StepBegin { step: StepKind::End, whose: PlayerRel::You }, &[Effect::AddCounterSource { kind: CounterKind::Bore, amount: Amount::Fixed(1) }, Effect::IfCondition { condition: Condition::CountersOnSelf(CounterKind::Bore, 3), then: &[Effect::RemoveAllCountersSelf { kind: CounterKind::Bore }, Effect::TransformSource], otherwise: &[] }], condition = Some(Condition::DescendedThisTurn))".into()
                }
                _ => return None,
            }
        })
    }
    fn transform48_entry(&self, p: &mut Params, execute: &str) -> Option<String> {
        Some({
            if p.take("Origin").as_deref() != Some("Any")
                || p.take("Destination").as_deref() != Some("Battlefield")
                || p.take("ValidCard").as_deref() != Some("Card.Self")
            {
                return None;
            }
            if let Some(zone) = p.take("TriggerZones")
                && zone != "Battlefield"
            {
                return None;
            }
            let (api, _) = Params::parse(self.svars.get(execute)?)?;
            let effect = match api.as_str() {
                "Token" => {
                    let child = self.transform48_node(
                        execute,
                        "Token",
                        &[(
                            "TokenScript",
                            "w_x_x_a_gnome_soldier_total_artifacts_creatures",
                        )],
                    )?;
                    if !child.exhausted() {
                        return None;
                    }
                    "Effect::CreateToken { token: &generated_tokens::GNOME_SOLDIER_WHITE_DYNAMIC }"
                }
                "Discard" => {
                    let mut child = self.transform48_node(
                        execute,
                        "Discard",
                        &[
                            ("AnyNumber", "True"),
                            ("Optional", "True"),
                            ("Mode", "TgtChoose"),
                            ("RememberDiscarded", "True"),
                        ],
                    )?;
                    let draw = child.take("SubAbility")?;
                    if !child.exhausted() {
                        return None;
                    }
                    let mut draw = self.transform48_node(
                        &draw,
                        "Draw",
                        &[("Defined", "You"), ("NumCards", "Remembered$Amount/Plus.1")],
                    )?;
                    let cleanup = draw.take("SubAbility")?;
                    if !draw.exhausted() {
                        return None;
                    }
                    let cleanup =
                        self.transform48_node(&cleanup, "Cleanup", &[("ClearRemembered", "True")])?;
                    if !cleanup.exhausted() {
                        return None;
                    }
                    "Effect::DiscardAnyThenDraw, Effect::draw(1)"
                }
                _ => return None,
            };
            format!("triggered!(Trigger::ETB, &[{effect}])")
        })
    }
}
