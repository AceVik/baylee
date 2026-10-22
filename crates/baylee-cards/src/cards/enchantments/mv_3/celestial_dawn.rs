//! Celestial Dawn — {1}{W}{W} — Enchantment
//! Oracle: Lands you control are Plains.
//! Oracle: Nonland permanents you control are white. The same is true for spells you control and nonland cards you own that aren't on the battlefield.
//! Oracle: You may spend white mana as though it were mana of any color. You may spend other mana only as though it were colorless mana.
//! Set: TSB #3 — Time Spiral Timeshifted | Scryfall ID: 251880e9-a472-4f33-9988-4871d31aa1d9 | Oracle ID: b4d27fad-9a94-4760-80a6-6a2948109864
// NOT SUPPORTED: Making spells you control and nonland cards you own outside the battlefield white, spending white mana as though it were mana of any color, and spending nonwhite mana only as though it were colorless mana are not yet supported by `Modifier`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const YOUR_NONLAND: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::LacksType(TypeSet::LAND)]);

card!(
    index = index::CELESTIAL_DAWN,
    oracle_id = "b4d27fad-9a94-4760-80a6-6a2948109864",
    scryfall_id = "251880e9-a472-4f33-9988-4871d31aa1d9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Making spells you control and nonland cards you own outside the battlefield white, spending white mana as though it were mana of any color, and spending nonwhite mana only as though it were colorless mana are not yet supported; in play, Celestial Dawn makes lands you control Plains (`SetLandType(PLAINS)`) and nonland permanents you control white (`SetColor(White)`).",
    ),
    faces = &[face!(
        name = "Celestial Dawn",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(
            Filter::YOUR_LAND,
            Modifier::SetLandType(subtypes::land::PLAINS),
        ),
        static_ability!(
            YOUR_NONLAND,
            Modifier::SetColor(ColorSet::from_slice(&[Color::White])),
        ),
    ],
);
