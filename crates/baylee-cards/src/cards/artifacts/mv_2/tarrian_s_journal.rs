//! Tarrian's Journal // The Tomb of Aclazotz — {1}{B} — Legendary Artifact — Book // Legendary Land — Cave
//! Oracle: {T}, Sacrifice another artifact or creature: Draw a card. Activate only as a sorcery.
//! Oracle: {2}, {T}, Discard your hand: Transform Tarrian's Journal.
//! Oracle: (Transforms from Tarrian's Journal.)
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: You may cast a creature spell from your graveyard this turn. If you do, it enters with a finality counter on it and is a Vampire in addition to its other types. (If a creature with a finality counter on it would die, exile it instead.)
//! Set: LCI #126 — The Lost Caverns of Ixalan | Scryfall ID: 99255a66-b868-45fc-a2a9-0c89bd851b69 | Oracle ID: a75b02ba-b0c8-47e3-a05c-e9ba221a7578
//! Face: Tarrian's Journal — {1}{B} — Legendary Artifact — Book
//! Face: The Tomb of Aclazotz —  — Legendary Land — Cave
// PARTIAL — the front's sorcery-speed "{T}, Sacrifice another artifact or creature: Draw a
// card" and the back's "{T}: Add {B}" are built; the transform and the back's graveyard
// permission are not, so the back face stays unreachable (see the NOT SUPPORTED lines).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The back face's one expressible line: "{T}: Add {B}."
///
/// NOT SUPPORTED: "{T}: You may cast a creature spell from your graveyard this
/// turn. If you do, it enters with a finality counter on it and is a Vampire in
/// addition to its other types." — nothing grants permission to cast a creature
/// spell out of a graveyard for the turn (`Modifier::GrantsFlashback` names one
/// card and `Modifier::PlayLandsFromGraveyard` is lands), the finality counter is
/// a `CounterKind` variant that does not exist, and the Vampire rider is a type
/// change nothing carries onto a spell cast later.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card!(
    index = index::TARRIAN_S_JOURNAL,
    oracle_id = "a75b02ba-b0c8-47e3-a05c-e9ba221a7578",
    scryfall_id = "99255a66-b868-45fc-a2a9-0c89bd851b69",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Tarrian's Journal",
            mana_cost = mana!("{1}{B}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::artifact::BOOK],
        ),
        face!(
            name = "The Tomb of Aclazotz",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::land::CAVE],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "no transform — `CostPart` cannot discard a whole hand and no effect turns a \
         permanent over where it stands, so the back face is unreachable; and the back's \
         cast-from-graveyard permission with its finality counter and Vampire rider has no \
         variant either"
    ),
    // NOT SUPPORTED: "{2}, {T}, Discard your hand: Transform Tarrian's Journal." —
    // `CostPart::Discard(filter)` pays with one card of your choice and cannot say
    // "your hand", and nothing in the DSL turns a permanent over in place
    // (`Effect::ExileSelfReturnAsFace` exiles it and returns it, a different object).
    abilities = &[activated!(
        cost!(TapSelf, Sacrifice(&f!(another ARTIFACT_OR_CREATURE))),
        &[Effect::draw(1)],
        timing = ActivationTiming::SorcerySpeed
    )],
);
