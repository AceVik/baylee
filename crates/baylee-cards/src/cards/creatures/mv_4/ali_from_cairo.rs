//! Ali from Cairo — {2}{R}{R} — Creature — Human
//! Oracle: Damage that would reduce your life total to less than 1 reduces it to 1 instead.
//! Set: ME4 #107 — Masters Edition IV | Scryfall ID: 2719516a-7a47-413e-b1a3-15543d229e08 | Oracle ID: 6517edb3-30e2-40ba-b6e4-4554b4bbb342
// IMPLEMENTED — damage-result replacement limits life loss while retaining
// the full damage, its other results, and simultaneous life gains.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ALI_FROM_CAIRO,
    oracle_id = "6517edb3-30e2-40ba-b6e4-4554b4bbb342",
    scryfall_id = "2719516a-7a47-413e-b1a3-15543d229e08",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Ali from Cairo",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(0),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::DamageLifeLossFloor {
            players: PlayerRel::You,
            minimum: 1,
        }
    )],
);
