//! Energy Arc — {W}{U} — Instant
//! Oracle: Untap any number of target creatures. Prevent all combat damage that would be dealt to and dealt by those creatures this turn.
//! Set: ME1 #144 — Masters Edition | Scryfall ID: ef7aae6c-199f-486f-9de6-a41dd8b847a3 | Oracle ID: 959bcf99-3c9a-4f99-98b2-13514d3dac16
// NOT SUPPORTED: preventing all combat damage that would be dealt to and dealt by the targeted creatures this turn is not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENERGY_ARC,
    oracle_id = "959bcf99-3c9a-4f99-98b2-13514d3dac16",
    scryfall_id = "ef7aae6c-199f-486f-9de6-a41dd8b847a3",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Partial(
        "untaps any number of target creatures; preventing all combat damage that would be dealt to and dealt by those creatures this turn is not supported",
    ),
    faces = &[face!(
        name = "Energy Arc",
        mana_cost = mana!("{W}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::UntapTarget],
        targets = Some(TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 255)),
    )],
);
