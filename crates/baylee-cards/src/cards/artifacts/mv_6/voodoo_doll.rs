//! Voodoo Doll — {6} — Artifact
//! Oracle: At the beginning of your upkeep, put a pin counter on this artifact.
//! Oracle: At the beginning of your end step, if this artifact is untapped, destroy this artifact and it deals damage to you equal to the number of pin counters on it.
//! Oracle: {X}{X}, {T}: This artifact deals damage equal to the number of pin counters on it to any target. X is the number of pin counters on this artifact.
//! Set: ME3 #203 — Masters Edition III | Scryfall ID: c60ea64d-0209-4ca4-bee6-f9eb63784c9e | Oracle ID: 4d330c40-3d72-4528-a254-d036683958d3

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOODOO_DOLL,
    oracle_id = "4d330c40-3d72-4528-a254-d036683958d3",
    scryfall_id = "c60ea64d-0209-4ca4-bee6-f9eb63784c9e",
    faces = &[face!(
        name = "Voodoo Doll",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::AddCounterFilter {
                filter: &Filter::This,
                kind: counters::PIN,
                amount: Amount::Fixed(1)
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You
            },
            &[
                Effect::destroy(TargetSpec::ThisObject),
                Effect::DealDamage {
                    amount: Amount::CountersOnSource(counters::PIN),
                    target: TargetSpec::Player(PlayerRel::You)
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::Untapped)),
        ),
        activated!(
            cost!("{X}{X}", TapSelf),
            &[Effect::DealDamage {
                amount: Amount::CountersOnSource(counters::PIN),
                target: TargetSpec::AnyTarget
            }],
            target = Some(TargetSpec::AnyTarget),
            x_value = ActivationX::Defined(Amount::CountersOnSource(counters::PIN)),
        ),
    ],
);
