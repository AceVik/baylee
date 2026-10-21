//! Orcish Conscripts — {R} — Creature — Orc
//! Oracle: This creature can't attack unless at least two other creatures attack.
//! Oracle: This creature can't block unless at least two other creatures block.
//! Set: ME2 #140 — Masters Edition II | Scryfall ID: 58248234-caea-4d55-aedb-07d28617e35b | Oracle ID: d99e1d08-32ca-49c5-97cc-e064d661202f
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: No Modifier restricts attacking or blocking unless at least two other creatures also attack or block; Orcish Conscripts attacks and blocks alone as an unrestricted 2/2.
card!(
    index = index::ORCISH_CONSCRIPTS,
    oracle_id = "d99e1d08-32ca-49c5-97cc-e064d661202f",
    scryfall_id = "58248234-caea-4d55-aedb-07d28617e35b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no modifier requires at least two other attackers or blockers, so it attacks and blocks alone as a 2/2",
    ),
    faces = &[face!(
        name = "Orcish Conscripts",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC],
        power = Some(2),
        toughness = Some(2),
    ),],
);
