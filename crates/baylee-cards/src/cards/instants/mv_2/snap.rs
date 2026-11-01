//! Snap — {1}{U} — Instant
//! Oracle: Return target creature to its owner's hand. Untap up to two lands.
//! Set: DMR #66 — Dominaria Remastered | Scryfall ID: f13406c6-f208-402a-94d3-a94a24f03563 | Oracle ID: ac914d98-221e-426c-8a50-342896b15f9e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SNAP,
    oracle_id = "ac914d98-221e-426c-8a50-342896b15f9e",
    scryfall_id = "f13406c6-f208-402a-94d3-a94a24f03563",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Snap",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::ReturnToHand {
                target: TargetSpec::Object(&Filter::CREATURE),
            },
            Effect::UntapChosen {
                filter: &Filter::LAND,
                count: 2,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
