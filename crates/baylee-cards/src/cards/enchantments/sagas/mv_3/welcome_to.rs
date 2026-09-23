//! Welcome to . . . // Jurassic Park — {1}{G}{G} — Enchantment — Saga // Legendary Land
//! Oracle: (As this Saga enters and after your draw step, add a lore counter.)
//! Oracle: I — For each opponent, up to one target noncreature artifact they control becomes a 0/4 Wall artifact creature with defender for as long as you control this Saga.
//! Oracle: II — Create a 3/3 green Dinosaur creature token with trample. It gains haste until end of turn.
//! Oracle: III — Destroy all Walls. Exile this Saga, then return it to the battlefield transformed under your control.
//! Oracle: (Transforms from Welcome to ....)
//! Oracle: Each Dinosaur card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard. (You may cast cards from your graveyard for their escape cost.)
//! Oracle: {T}: Add {G} for each Dinosaur you control.
//! Set: REX #7 — Jurassic World Collection | Scryfall ID: 6d84e2d4-38bf-4d46-99a6-37c2dda66b16 | Oracle ID: a4b37d16-95b3-4143-a0b2-ad9f2aba91f8
//! Face: Welcome to . . . — {1}{G}{G} — Enchantment — Saga
//! Face: Jurassic Park —  — Legendary Land
// IMPLEMENTED — chapter III (destroy every Wall, then transform into Jurassic
// Park) and the back face's "{T}: Add {G} for each Dinosaur you control".
// Chapters I and II and the escape grant are NOT SUPPORTED; see below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;
use baylee_core::generated::subtypes::creature;

/// "Each Dinosaur you control" — the count Jurassic Park's mana ability
/// reads. A Dinosaur an opponent controls makes no mana.
static YOUR_DINOSAURS: Filter = Filter::And(&[
    Filter::HasSubtype(creature::DINOSAUR),
    Filter::ControlledByYou,
]);

/// Jurassic Park's only ability: `{T}: Add {G} for each Dinosaur you control.`
static JURASSIC_PARK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_dynamic(
    ManaColor::Green,
    Amount::CountOf {
        filter: &YOUR_DINOSAURS,
        zone: ZoneSel::Battlefield,
    },
)])];

card!(
    index = index::WELCOME_TO,
    oracle_id = "a4b37d16-95b3-4143-a0b2-ad9f2aba91f8",
    scryfall_id = "6d84e2d4-38bf-4d46-99a6-37c2dda66b16",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "chapter I's per-opponent target count, chapter II's Dinosaur token and \
         its haste rider, and escape on Dinosaur cards in the graveyard"
    ),
    faces = &[
        face!(
            name = "Welcome to . . .",
            mana_cost = mana!("{1}{G}{G}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[subtypes::enchantment::SAGA],
        ),
        face!(
            name = "Jurassic Park",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            // The back of a transforming card is reached by turning the card
            // over and never by casting it (CR 712.2). The stub reader leaves
            // this line off every *land* back, because an MDFC's land back is
            // the one a player does play — so this is the case it cannot tell
            // apart and a person has to.
            castable_from_hand = false,
            // NOT SUPPORTED: "Each Dinosaur card in your graveyard has escape.
            // The escape cost is equal to the card's mana cost plus exile
            // three other cards from your graveyard." — no `Modifier` grants
            // escape, and `Modifier::GrantsFlashback` is flashback: it pays
            // the mana cost alone and exiles the card afterwards.
            abilities = JURASSIC_PARK_MANA,
        ),
    ],
    abilities = &[
        // NOT SUPPORTED: chapter I — "For each opponent, up to one target
        // noncreature artifact they control becomes a 0/4 Wall artifact
        // creature with defender for as long as you control this Saga."
        // `TargetReq` states one fixed `max` for the whole effect, so "up to
        // one per opponent" has no spelling at all; and the change is four
        // layers at once (type, subtype, P/T, keyword) where a
        // `CreateContinuousEffect` carries one `Modifier`.
        // NOT SUPPORTED: chapter II — "Create a 3/3 green Dinosaur creature
        // token with trample. It gains haste until end of turn." No token in
        // `crate::tokens` is a 3/3 green Dinosaur and a card file may not
        // define one, and "it gains haste" is a continuous effect on the
        // token *this* effect created, which no `Filter` can name —
        // `PumpFilter` would give haste to every Dinosaur you control.
        chapter!(
            3,
            &[
                Effect::destroy_all(&Filter::HasSubtype(creature::WALL)),
                Effect::ExileSelfReturnAsFace { face: 1 },
            ]
        ),
    ],
);
