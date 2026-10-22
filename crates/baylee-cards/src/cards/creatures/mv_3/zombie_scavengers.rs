//! Zombie Scavengers — {2}{B} — Creature — Zombie
//! Oracle: Exile the top creature card of your graveyard: Regenerate this creature.
//! Set: WTH #87 — Weatherlight | Scryfall ID: 2ec786b1-6097-4e97-99b0-571d6e3e73e7 | Oracle ID: 8c5298da-6759-444e-8870-6c96d226c940
// PARTIAL — Activation cost exiles any creature card of your choice from your graveyard to regenerate this creature rather than strictly the top creature card of your graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZOMBIE_SCAVENGERS,
    oracle_id = "8c5298da-6759-444e-8870-6c96d226c940",
    scryfall_id = "2ec786b1-6097-4e97-99b0-571d6e3e73e7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Activation cost exiles any creature card of your choice from your graveyard to regenerate this creature rather than strictly the top creature card of your graveyard."
    ),
    faces = &[face!(
        name = "Zombie Scavengers",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(ExileFromGraveyard(&Filter::CREATURE)),
        &[Effect::regenerate(TargetSpec::ThisObject)]
    ),],
);
