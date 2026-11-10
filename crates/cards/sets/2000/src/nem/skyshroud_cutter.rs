//! Skyshroud Cutter — {3}{G} — Creature — Beast
//! Oracle: If you control a Forest, rather than pay this spell's mana cost, you may have each other player gain 5 life.
//! Set: NEM #118 — Nemesis | Scryfall ID: a558c4f5-a716-4e46-9234-5f84f1bd57aa | Oracle ID: fbd71691-d0fd-4842-913b-fe6096d509a3
// PARTIAL — {3}{G} 2/2 Beast creature.
// NOT SUPPORTED: conditional alternative cost (`CostPart` has no opponent-life-gain payment).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKYSHROUD_CUTTER,
    oracle_id = "fbd71691-d0fd-4842-913b-fe6096d509a3",
    scryfall_id = "a558c4f5-a716-4e46-9234-5f84f1bd57aa",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "CostPart has no opponent-life-gain payment for the Forest alternative cost",
    ),
    faces = &[face!(
        name = "Skyshroud Cutter",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(2),
    ),],
);
