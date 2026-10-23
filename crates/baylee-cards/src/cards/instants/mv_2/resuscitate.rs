//! Resuscitate — {1}{G} — Instant
//! Oracle: Until end of turn, creatures you control gain "{1}: Regenerate this creature."
//! Set: EXO #121 — Exodus | Scryfall ID: f5707560-fcc6-4aca-adce-d41de45f37e8 | Oracle ID: 4390e251-e154-4ef8-9929-0c78aa2c8f10
// PARTIAL: Granting an activated ability from a resolving spell is not supported by the ability-line mapper; resolves with no effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RESUSCITATE,
    oracle_id = "4390e251-e154-4ef8-9929-0c78aa2c8f10",
    scryfall_id = "f5707560-fcc6-4aca-adce-d41de45f37e8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Granting an activated ability from a resolving spell is not supported by the ability-line mapper; resolves with no effect.",
    ),
    faces = &[face!(
        name = "Resuscitate",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[])],
);
