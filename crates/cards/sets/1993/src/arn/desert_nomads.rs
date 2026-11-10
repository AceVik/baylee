//! Desert Nomads — {2}{R} — Creature — Human Nomad
//! Oracle: Desertwalk
//! Oracle: Prevent all damage that would be dealt to this creature by Deserts.
//! Set: ARN #38 — Arabian Nights | Scryfall ID: e46d0c10-ec09-48ba-9e93-1392dca8111a | Oracle ID: 3247fca3-7458-47ee-875b-c55f2a3e2962
// IMPLEMENTED — desertwalk is Modifier::LandwalkMatching over Deserts;
// the prevention is Modifier::PreventDamageFrom(Deserts), all damage.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DESERT: Filter = Filter::HasSubtype(subtypes::land::DESERT);

card!(
    index = index::DESERT_NOMADS,
    oracle_id = "3247fca3-7458-47ee-875b-c55f2a3e2962",
    scryfall_id = "e46d0c10-ec09-48ba-9e93-1392dca8111a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Desert Nomads",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::LandwalkMatching(&DESERT)),
        static_ability!(Filter::This, Modifier::PreventDamageFrom(&DESERT)),
    ],
);
