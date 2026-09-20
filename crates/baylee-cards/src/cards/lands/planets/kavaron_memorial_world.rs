//! Kavaron, Memorial World — (no cost) — Land — Planet
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Planet. Station only as a sorcery.)
//! Oracle: 12+ | {1}{R}, {T}, Sacrifice a land: Create a 2/2 colorless Robot artifact creature token, then creatures you control get +1/+0 and gain haste until end of turn.
//! Set: EOE #255 — Edge of Eternities | Scryfall ID: 60f3ca25-9dcc-4781-bf7b-ab6736d8db29 | Oracle ID: 4fa826ca-d361-4391-ad0d-989ebcfa4a91
// PARTIAL — enters tapped, {T}: Add {R}, and the 12+ ability gated on twelve
// charge counters (Condition::CountersOnSelf). Station itself is not
// expressible; see the NOT SUPPORTED note beside the abilities.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KAVARON_MEMORIAL_WORLD,
    oracle_id = "4fa826ca-d361-4391-ad0d-989ebcfa4a91",
    scryfall_id = "60f3ca25-9dcc-4781-bf7b-ab6736d8db29",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Station — the charge counters are equal to the power of the creature its cost tapped, and no Amount reads an object named by a cost"
    ),
    faces = &[face!(
        name = "Kavaron, Memorial World",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::PLANET],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        // NOT SUPPORTED: Station (Tap another creature you control: Put
        // charge counters equal to its power on this Planet. Station only as
        // a sorcery.) — the count comes off the creature the cost tapped,
        // and the DSL has no `Amount` that reads an object a cost named;
        // `Amount::TargetPower` reads a *target*, and this ability prints no
        // "target" for one to point at. `Station` is also a keyword no
        // engine rule reads, so it would be a dead bit.
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{1}{R}", TapSelf, Sacrifice(&Filter::LAND)),
            &[
                Effect::CreateToken {
                    token: &generated_tokens::ROBOT_ARTIFACT_2_2,
                },
                Effect::PumpFilter {
                    filter: &Filter::YOUR_CREATURE,
                    controlled_by: None,
                    power: Amount::Fixed(1),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::HASTE,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
            condition = Some(Condition::CountersOnSelf(CounterKind::Charge, 12)),
        ),
    ],
);
