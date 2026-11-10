//! Camel — {W} — Creature — Camel
//! Oracle: Banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Oracle: As long as this creature is attacking, prevent all damage Deserts would deal to this creature and to creatures banded with this creature.
//! Set: ARN #3 — Arabian Nights | Scryfall ID: e0078aa8-bfb8-43b0-a6b7-1991596c21e1 | Oracle ID: d33b3591-c01f-4ac4-8626-7cbfdabaf90d
// IMPLEMENTED — banding is a keyword bit; the prevention is
// Modifier::PreventDamageFrom(Deserts) on this creature while it attacks
// and on the creatures banded with it (Filter::BandedWithSource).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DESERT: Filter = Filter::HasSubtype(subtypes::land::DESERT);

card!(
    index = index::CAMEL,
    oracle_id = "d33b3591-c01f-4ac4-8626-7cbfdabaf90d",
    scryfall_id = "e0078aa8-bfb8-43b0-a6b7-1991596c21e1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Camel",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAMEL],
        power = Some(0),
        toughness = Some(1),
    ),],
    keywords = KeywordSet::BANDING,
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::Or(&[
            Filter::And(&[Filter::This, Filter::Attacking]),
            Filter::BandedWithSource,
        ]),
        Modifier::PreventDamageFrom(&DESERT)
    )],
);
