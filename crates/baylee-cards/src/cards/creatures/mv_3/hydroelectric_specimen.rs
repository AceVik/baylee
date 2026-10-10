//! Hydroelectric Specimen // Hydroelectric Laboratory — {2}{U} — Creature — Weird // Land
//! Oracle: Flash
//! Oracle: When this creature enters, you may change the target of target instant or sorcery spell with a single target to this creature.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: MH3 #240 — Modern Horizons 3 | Scryfall ID: 8689ecd7-e9a6-458b-99d2-6dbaca527f00 | Oracle ID: 573151f0-00d4-4a8a-8a09-745c5f376532
//! Face: Hydroelectric Specimen — {2}{U} — Creature — Weird
//! Face: Hydroelectric Laboratory —  — Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static LABORATORY_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

card!(
    index = index::HYDROELECTRIC_SPECIMEN,
    oracle_id = "573151f0-00d4-4a8a-8a09-745c5f376532",
    scryfall_id = "8689ecd7-e9a6-458b-99d2-6dbaca527f00",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Hydroelectric Specimen",
            mana_cost = mana!("{2}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::WEIRD],
            power = Some(1),
            toughness = Some(4),
        ),
        face!(
            name = "Hydroelectric Laboratory",
            types = TypeSet::LAND,
            abilities = LABORATORY_MANA,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
        ),
    ],
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::MayDo {
            effects: &[Effect::ChangeTarget { to: &Filter::This }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::And(&[
            Filter::INSTANT_OR_SORCERY,
            Filter::WithSingleTarget
        ]))))
    )],
);
