//! The Everflowing Well // The Myriad Pools — {2}{U} — Legendary Artifact // Legendary Artifact Land
//! Oracle: When The Everflowing Well enters, mill two cards, then draw two cards.
//! Oracle: Descend 8 — At the beginning of your upkeep, if there are eight or more permanent cards in your graveyard, transform The Everflowing Well.
//! Oracle: (Transforms from The Everflowing Well.)
//! Oracle: {T}: Add {U}.
//! Oracle: Whenever you cast a permanent spell using mana produced by The Myriad Pools, up to one other target permanent you control becomes a copy of that spell until end of turn.
//! Set: LCI #56 — The Lost Caverns of Ixalan | Scryfall ID: bf573fb7-fa6c-4df7-8e5e-1e071585361e | Oracle ID: 1f57a9f1-6b95-4395-bdf0-c5289b786ab1
//! Face: The Everflowing Well — {2}{U} — Legendary Artifact
//! Face: The Myriad Pools —  — Legendary Artifact Land
// Partial — the front face's enter trigger (mill two, then draw two) and the
// back face's {T}: Add {U} are built; the descend-8 transform and the
// mana-provenance copy are not, see the two NOT SUPPORTED clauses below.

use baylee_cards_dsl::prelude::*;

/// The Myriad Pools' `{T}: Add {U}.`
static MYRIAD_POOLS_MANA: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
    // NOT SUPPORTED: "Whenever you cast a permanent spell using mana
    // produced by The Myriad Pools, up to one other target permanent you
    // control becomes a copy of that spell until end of turn." Pool mana
    // carries no provenance — nothing records which source produced the
    // mana that paid for a spell, so no `Trigger` can ask the question —
    // and no `Effect` makes a permanent become a copy of a spell until end
    // of turn (`CopyOnEnterUntilEot` and `Modifier::BecomeCopyOf` act as a
    // permanent arrives, which is a different sentence).
];

card!(
    index = index::THE_EVERFLOWING_WELL,
    oracle_id = "1f57a9f1-6b95-4395-bdf0-c5289b786ab1",
    scryfall_id = "bf573fb7-fa6c-4df7-8e5e-1e071585361e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "The Everflowing Well",
            mana_cost = mana!("{2}{U}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "The Myriad Pools",
            types = TypeSet::ARTIFACT.union(TypeSet::LAND),
            supertypes = SupertypeSet::LEGENDARY,
            // CR 712.8a: in hand this card has only its *front* face's
            // characteristics, so the Pools are not a land anybody may play
            // -- they are only ever arrived at by the upkeep transform. The
            // field defaults to `true`, which is right for a modal card's
            // back face and wrong for a transforming one, and without it the
            // engine offered and accepted this as a land drop.
            castable_from_hand = false,
            abilities = MYRIAD_POOLS_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "no Condition counts \"eight or more permanent cards in your graveyard\", and mana produced by a named source is not tracked",
    ),
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[
                Effect::Mill {
                    amount: Amount::Fixed(2),
                    target: PlayerRel::You,
                },
                Effect::draw(2),
            ]
        ),
        // NOT SUPPORTED: "Descend 8 — At the beginning of your upkeep, if there
        // are eight or more permanent cards in your graveyard, transform The
        // Everflowing Well." — the trigger itself (`StepBegin { step: Upkeep,
        // whose: You }`) and the transform (`Effect::ExileSelfReturnAsFace`)
        // both exist, but no `Condition` can state the intervening `if`:
        // `OpponentGraveyardCountAtLeast` reads the other seat and counts cards
        // rather than permanent cards, and there is no reading of the
        // controller's own graveyard at all.
    ],
);
