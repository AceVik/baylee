//! Saffi Eriksdotter — {G}{W} — Legendary Creature — Human Scout
//! Oracle: Sacrifice Saffi Eriksdotter: When target creature is put into your graveyard this turn, return that card to the battlefield.
//! Set: TSR #260 — Time Spiral Remastered | Scryfall ID: 4176dda4-ec5c-4734-bb48-0876304aa219 | Oracle ID: 84ba3e2c-ea96-470b-92c6-b937dc87549b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAFFI_ERIKSDOTTER,
    oracle_id = "84ba3e2c-ea96-470b-92c6-b937dc87549b",
    scryfall_id = "4176dda4-ec5c-4734-bb48-0876304aa219",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Saffi Eriksdotter",
        mana_cost = mana!("{G}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SCOUT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
