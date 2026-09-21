//! Nearby Planet — (no cost) — Land
//! Oracle: Rangeling (This card is every land type, including Plains, Island, Swamp, Mountain, Forest, Desert, Gate, Lair, Locus, and all those Urza's ones.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, sacrifice it unless you pay {1}.
//! Set: UNF #198 — Unfinity | Scryfall ID: da785d1b-6b90-4b65-9efb-d7f329405318 | Oracle ID: 19d34126-8266-4da1-b7ef-67ecfa2dbbee
// PARTIAL — enters tapped (EnterModifier::Tapped) and its enter trigger asks
// for {1} or takes it to the graveyard. Rangeling has no DSL variant; see the
// note on its abilities.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NEARBY_PLANET,
    oracle_id = "19d34126-8266-4da1-b7ef-67ecfa2dbbee",
    scryfall_id = "da785d1b-6b90-4b65-9efb-d7f329405318",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    coverage = Coverage::Partial(
        "Rangeling: every land type — the DSL has no variant for it, and the \
         nearest one, Modifier::AllBasicLandTypes, covers only the five basic \
         land types"
    ),
    faces = &[face!(
        name = "Nearby Planet",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        // NOT SUPPORTED: Rangeling — "this card is every land type", including
        // Desert, Gate, Lair, Locus and the Urza's ones. `Modifier::
        // AllBasicLandTypes` is the nearest variant and is the five basic
        // types only, so a card built on it would answer "is this a Gate?"
        // wrongly. Nothing else is lost by leaving it out: a land with more
        // than one basic land type taps for nothing at all, because CR 305.6
        // grants one mana ability per type and `casting::intrinsic_mana`
        // declines a choice it cannot make.
        triggered!(
            Trigger::ETB,
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::You,
                mana: Amount::Fixed(1),
                effect: &Effect::SacrificeSelf,
            }]
        ),
    ],
);
