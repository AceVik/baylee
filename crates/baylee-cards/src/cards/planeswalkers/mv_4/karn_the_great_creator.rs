//! Karn, the Great Creator — {4} — Legendary Planeswalker — Karn
//! Oracle: Activated abilities of artifacts your opponents control can't be activated.
//! Oracle: +1: Until your next turn, up to one target noncreature artifact becomes an artifact creature with power and toughness each equal to its mana value.
//! Oracle: −2: You may reveal an artifact card you own from outside the game or choose a face-up artifact card you own in exile. Put that card into your hand.
//! Set: RVR #1 — Ravnica Remastered | Scryfall ID: deb3721d-fba1-444f-8b31-1cd10c94c4a0 | Oracle ID: a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1
// IMPLEMENTED — artifact lock, +1 animation, and the −2 wish. The wish
// reads the seat's sideboard, which now lives outside the game rather
// than being shuffled into the library.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::planeswalker;

static NONCREATURE_ARTIFACT: Filter = Filter::And(&[Filter::ARTIFACT, Filter::NONCREATURE]);

static ARTIFACT_YOU_OWN: Filter = Filter::And(&[Filter::OwnedByYou, Filter::ARTIFACT]);

card! {
    index: 81,
    oracle_id: "a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1",
    scryfall_id: "deb3721d-fba1-444f-8b31-1cd10c94c4a0",
    faces: &[face! {
        name: "Karn, the Great Creator",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::KARN],
        loyalty: Some(5),
    }],
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::Any,
            modifier: Modifier::CantActivateArtifacts,
            cross_zone: false,
        }),
        loyalty!(1, &[
                Effect::CreateContinuousEffect {
                    layer: Layer::Type,
                    filter: &NONCREATURE_ARTIFACT,
                    modifier: Modifier::AddType(TypeSet::CREATURE),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::SetPTFilter {
                    filter: &NONCREATURE_ARTIFACT,
                    power: Amount::TargetCmc,
                    toughness: Amount::TargetCmc,
                    duration: Duration::UntilEndOfTurn,
                },
            ], target: Some(TargetSpec::Object(&NONCREATURE_ARTIFACT))),
        loyalty!(-2, &[Effect::WishToHand {
                filter: &ARTIFACT_YOU_OWN,
            }]),
    ],
}
