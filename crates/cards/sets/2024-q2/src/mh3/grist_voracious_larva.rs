//! Grist, Voracious Larva // Grist, the Plague Swarm — {G} — Legendary Creature — Insect // Legendary Planeswalker — Grist
//! Oracle: Deathtouch
//! Oracle: Whenever Grist or another creature you control enters, if it entered from your graveyard or you cast it from your graveyard, you may pay {G}. If you do, exile Grist, then return it to the battlefield transformed under its owner's control.
//! Oracle: +1: Create a 1/1 black and green Insect creature token, then mill two cards. Put a deathtouch counter on the token if a black card was milled this way.
//! Oracle: −2: Destroy target artifact or enchantment.
//! Oracle: −6: For each creature card in your graveyard, create a token that's a copy of it, except it's a 1/1 black and green Insect.
//! Set: MH3 #251 — Modern Horizons 3 | Scryfall ID: 68239b41-b7db-4044-b672-6808c2c342ec | Oracle ID: 196f8f13-d4df-4740-8746-8fed3bc71840
//! Face: Grist, Voracious Larva — {G} — Legendary Creature — Insect
//! Face: Grist, the Plague Swarm —  — Legendary Planeswalker — Grist
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRIST_VORACIOUS_LARVA,
    oracle_id = "196f8f13-d4df-4740-8746-8fed3bc71840",
    scryfall_id = "68239b41-b7db-4044-b672-6808c2c342ec",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[
        face!(
            name = "Grist, Voracious Larva",
            mana_cost = mana!("{G}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::INSECT],
            power = Some(1),
            toughness = Some(2),
        ),
        face!(
            name = "Grist, the Plague Swarm",
            types = TypeSet::PLANESWALKER,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::planeswalker::GRIST],
            loyalty = Some(3),
            color_indicator = ColorSet::from_slice(&[Color::Black, Color::Green]),
            castable_from_hand = false,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
