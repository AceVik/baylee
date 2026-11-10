//! Daughter of Autumn — {2}{G}{G} — Legendary Creature — Avatar
//! Oracle: {W}: The next 1 damage that would be dealt to target white creature this turn is dealt to Daughter of Autumn instead.
//! Set: HML #85 — Homelands | Scryfall ID: 972e9c59-f340-414c-b55b-39d46dd97e8e | Oracle ID: 45f3b9f2-3fce-4f91-bcbd-de069e5f9e4c
// PARTIAL — a 2/4 body: the {W} damage redirection is not said.
// NOT SUPPORTED: "{W}: The next 1 damage that would be dealt to target white creature this turn is dealt to Daughter of Autumn instead." RedirectNextDamage sends damage to a player and not to a creature; the old card prevented the damage outright, which spared the white creature without Daughter of Autumn paying for it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DAUGHTER_OF_AUTUMN,
    oracle_id = "45f3b9f2-3fce-4f91-bcbd-de069e5f9e4c",
    scryfall_id = "972e9c59-f340-414c-b55b-39d46dd97e8e",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "the {W} redirection of the next 1 damage to target white creature onto Daughter of Autumn is not said (RedirectNextDamage sends damage to a player, not to a creature); prevention would spare the creature for free, so the ability is off and only the 2/4 body is played"
    ),
    faces = &[face!(
        name = "Daughter of Autumn",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(2),
        toughness = Some(4),
    ),],
);
