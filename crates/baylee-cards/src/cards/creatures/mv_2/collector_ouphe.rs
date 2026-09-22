//! Collector Ouphe — {1}{G} — Creature — Ouphe
//! Oracle: Activated abilities of artifacts can't be activated.
//! Set: MH1 #158 — Modern Horizons | Scryfall ID: 085107a2-c1ec-473c-81d8-23e5a7197776 | Oracle ID: 0c4bc9ea-a5fd-4f44-96a1-5448eee228c4
// PARTIAL — the 2/2 body is built; the card's one clause is not expressible,
// see the NOT SUPPORTED line below. No ability is claimed for it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COLLECTOR_OUPHE,
    oracle_id = "0c4bc9ea-a5fd-4f44-96a1-5448eee228c4",
    scryfall_id = "085107a2-c1ec-473c-81d8-23e5a7197776",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Collector Ouphe",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OUPHE],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial(
        "Activated abilities of artifacts can't be activated — \
         Modifier::CantActivateArtifacts reaches the artifacts the effect's \
         opponents control (Karn, the Great Creator), and the DSL has no \
         Modifier whose reach is every artifact, its controller's included"
    ),
);

// NOT SUPPORTED: Activated abilities of artifacts can't be activated.
//
// `Modifier::CantActivateArtifacts` is the one variant that says anything
// like this sentence, and its reach is "artifacts the effect's opponents
// control" — Karn, the Great Creator's wording, not this card's. A static
// written with it would be a Collector Ouphe whose own controller keeps
// activating their artifacts' abilities (their mana rocks, their Equipment),
// which is the half of the card that plays, and no `Filter` can widen a
// modifier that scopes itself to the effect's opponents. The ability comes
// off the card rather than on: an ability that plays the wrong way is worse
// than one that is missing.
