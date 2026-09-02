//! Venser, the Sojourner — {3}{W}{U} — Legendary Planeswalker — Venser
//! Oracle: +2: Exile target permanent you own. Return it to the battlefield under your control at the beginning of the next end step.
//! Oracle: −1: Creatures can't be blocked this turn.
//! Oracle: −8: You get an emblem with "Whenever you cast a spell, exile target permanent."
//! Set: DDI #1 — Duel Decks: Venser vs. Koth | Scryfall ID: 8f61a0ea-c2e8-4571-9669-19abd8bbc874 | Oracle ID: a8bf8ff8-d924-4fd2-b5ed-05b38f55325a
// IMPLEMENTED — all three loyalty abilities, including the −8 emblem
// (emblem objects carry abilities; the command zone is scanned for
// triggers).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::planeswalker;

static PERMANENT_YOU_OWN: Filter =
    Filter::And(&[Filter::OwnedByYou, Filter::InZone(ZoneRef::Battlefield)]);

card! {
    index: 183,
    oracle_id: "a8bf8ff8-d924-4fd2-b5ed-05b38f55325a",
    scryfall_id: "8f61a0ea-c2e8-4571-9669-19abd8bbc874",
    faces: &[face! {
        name: "Venser, the Sojourner",
        mana_cost: baylee_core::mana!("{3}{W}{U}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::VENSER],
        loyalty: Some(3),
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        loyalty!(2, &[Effect::ExileAndReturnAtEndStep], target: Some(TargetSpec::Object(&PERMANENT_YOU_OWN))),
        loyalty!(-1, &[Effect::CreateContinuousEffect {
                layer: Layer::Ability,
                filter: &Filter::CREATURE,
                modifier: Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
                duration: Duration::UntilEndOfTurn,
            }]),
        loyalty!(-8, &[Effect::CreateEmblem {
                abilities: EMBLEM_ABILITIES,
            }]),
    ],
}

static EMBLEM_ABILITIES: &[AbilityDef] = &[
    triggered!(Trigger::SpellCast(&Filter::ControlledByYou), &[Effect::Exile {
        target: TargetSpec::Object(&Filter::Any),
    }], targets: Some(TargetReq::one(TargetSpec::Object(
        &Filter::Any,
    )))),
];
