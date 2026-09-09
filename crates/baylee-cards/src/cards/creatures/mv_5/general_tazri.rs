//! General Tazri — {4}{W} — Legendary Creature — Human Ally
//! Oracle: When General Tazri enters, you may search your library for an Ally creature card, reveal it, put it into your hand, then shuffle.
//! Oracle: {W}{U}{B}{R}{G}: Ally creatures you control get +X/+X until end of turn, where X is the number of colors among those creatures.
//! Set: OGW #19 — Oath of the Gatewatch | Scryfall ID: 34e9aa86-1a31-4c0f-928d-923f066286b6 | Oracle ID: b0f19cba-1339-4518-8320-d7b1dcaf2eb0
// IMPLEMENTED — ETB ally tutor and the {WUBRG} pump, whose X is the number of
// colours among the Allies. Reveal is presentation-only (protocol, M3).

static ALLIES_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasSubtype(creature::ALLY),
    Filter::CREATURE,
    Filter::ControlledByYou,
]);
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self, creature};

static ALLY_CARD: Filter = Filter::And(&[Filter::HasSubtype(creature::ALLY), Filter::CREATURE]);

card! {
    index: 57,
    oracle_id: "b0f19cba-1339-4518-8320-d7b1dcaf2eb0",
    scryfall_id: "34e9aa86-1a31-4c0f-928d-923f066286b6",
    faces: &[face! {
        name: "General Tazri",
        mana_cost: baylee_core::mana!("{4}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::ALLY],
        power: Some(3),
        toughness: Some(4),
    }],
    color_identity: ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::White,
        Color::Blue,
    ]),
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::SearchLibrary {
                filter: &ALLY_CARD,
                finds: &[Find::HAND],
                optional: true,
            }]),
        activated!(Cost {
                mana: baylee_core::mana!("{W}{U}{B}{R}{G}"),
                parts: &[],
            }, &[Effect::PumpFilter {
                filter: &ALLIES_YOU_CONTROL,
                power: Amount::DistinctColorsAmong(&ALLIES_YOU_CONTROL),
                toughness: Amount::DistinctColorsAmong(&ALLIES_YOU_CONTROL),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }]),
    ],
}

// Engine-level coverage lives in baylee-engine m2 tests: with Maskwood
// Nexus on the battlefield, a non-Ally creature card in the library is
// a legal tutor option (cross-zone projection).
