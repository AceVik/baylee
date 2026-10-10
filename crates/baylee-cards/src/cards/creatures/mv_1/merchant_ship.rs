//! Merchant Ship — {U} — Creature — Human
//! Oracle: This creature can't attack unless defending player controls an Island.
//! Oracle: Whenever this creature attacks and isn't blocked, you gain 2 life.
//! Oracle: When you control no Islands, sacrifice this creature.
//! Set: ARN #17 — Arabian Nights | Scryfall ID: 2b827094-fb2c-46db-b898-02e0c308601f | Oracle ID: 69556f6c-c05b-4902-bac7-012f0ed81b75
// IMPLEMENTED — the attack restriction, Trigger::AttacksAndIsntBlocked for
// the life gain, and the no-Islands sacrifice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);

static YOUR_ISLANDS: Filter = Filter::And(&[ISLAND, Filter::ControlledByYou]);

static NO_ISLANDS: Condition = Condition::ControlCountAtMost(&YOUR_ISLANDS, 0);

card!(
    index = index::MERCHANT_SHIP,
    oracle_id = "69556f6c-c05b-4902-bac7-012f0ed81b75",
    scryfall_id = "2b827094-fb2c-46db-b898-02e0c308601f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Merchant Ship",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(0),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CantAttackUnlessDefenderControls(&ISLAND)
        ),
        triggered!(
            Trigger::AttacksAndIsntBlocked(&Filter::This),
            &[Effect::gain_life(2)]
        ),
        triggered!(Trigger::State(&NO_ISLANDS), &[Effect::SacrificeSelf]),
    ],
);
