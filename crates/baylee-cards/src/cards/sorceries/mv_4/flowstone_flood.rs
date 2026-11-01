//! Flowstone Flood — {3}{R} — Sorcery
//! Oracle: Buyback—Pay 3 life, Discard a card at random. (You may pay 3 life and discard a card at random in addition to any other costs as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Destroy target land.
//! Set: EXO #83 — Exodus | Scryfall ID: 8bcda003-6fac-4879-87e6-ec0c115630ba | Oracle ID: 7f77de5d-1088-42d0-8f68-5633aeab0d74
// PARTIAL — Buyback is not supported; this spell only destroys target land and goes to the graveyard as it resolves.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Buyback is not supported; this spell only destroys target land and goes to the graveyard as it resolves.
card!(
    index = index::FLOWSTONE_FLOOD,
    oracle_id = "7f77de5d-1088-42d0-8f68-5633aeab0d74",
    scryfall_id = "8bcda003-6fac-4879-87e6-ec0c115630ba",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Buyback is not supported; this spell only destroys target land and goes to the graveyard as it resolves."
    ),
    faces = &[face!(
        name = "Flowstone Flood",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    )],
);
