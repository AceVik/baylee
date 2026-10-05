//! Camel — {W} — Creature — Camel
//! Oracle: Banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Oracle: As long as this creature is attacking, prevent all damage Deserts would deal to this creature and to creatures banded with this creature.
//! Set: ARN #3 — Arabian Nights | Scryfall ID: e0078aa8-bfb8-43b0-a6b7-1991596c21e1 | Oracle ID: d33b3591-c01f-4ac4-8626-7cbfdabaf90d
// PARTIAL — banding is a keyword bit; the damage prevention is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

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
    coverage = Coverage::Partial(
        "the prevention is not written: no Modifier prevents damage from a \
         filtered source without also stopping targeting and blocking, and \
         no Filter names the creatures banded with the source"
    ),
    // NOT SUPPORTED: "As long as this creature is attacking, prevent all
    // damage Deserts would deal to this creature and to creatures banded
    // with this creature." — the nearest piece, `Modifier::ProtectionFrom`,
    // can name Deserts as its source filter and does prevent their damage,
    // but it also stops Deserts from targeting and from blocking, which the
    // card does not say; `Modifier::PreventDamageToIt` prevents combat
    // damage only, from any source, and reaches only the affected object.
    // The banded-with relation is missing either way: no `Filter` names the
    // creatures banded with the source.
    abilities = &[],
);
