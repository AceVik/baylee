//! Reign of Chaos — {2}{R}{R} — Sorcery
//! Oracle: Choose one —
//! Oracle: • Destroy target Plains and target white creature.
//! Oracle: • Destroy target Island and target blue creature.
//! Set: MIR #190 — Mirage | Scryfall ID: 9285b14a-fc8e-457a-b803-202e05be41e5 | Oracle ID: 436b5989-9b8c-4e03-a82e-046837c891ca
// PARTIAL: Effect::Destroy reads only the first target instance rather than also destroying the second target instance; each mode requires both targets on cast and destroys the targeted Plains or Island.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PLAINS: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::PLAINS)]);
static WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);
static ISLAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::ISLAND)]);
static BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

card!(
    index = index::REIGN_OF_CHAOS,
    oracle_id = "436b5989-9b8c-4e03-a82e-046837c891ca",
    scryfall_id = "9285b14a-fc8e-457a-b803-202e05be41e5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Effect::Destroy reads only the first target instance rather than also destroying the second target instance; each mode requires both targets on cast and destroys the targeted Plains or Island",
    ),
    faces = &[face!(
        name = "Reign of Chaos",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::destroy(TargetSpec::Object(&PLAINS))],
                targets = Some(TargetReq::one(TargetSpec::Object(&PLAINS))),
                second_targets = Some(TargetReq::one(TargetSpec::Object(&WHITE_CREATURE))),
            ),
            mode!(
                &[Effect::destroy(TargetSpec::Object(&ISLAND))],
                targets = Some(TargetReq::one(TargetSpec::Object(&ISLAND))),
                second_targets = Some(TargetReq::one(TargetSpec::Object(&BLUE_CREATURE))),
            ),
        ],
    }],
);
