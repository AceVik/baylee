//! Assassin's Trophy — {B}{G} — Instant
//! Oracle: Destroy target permanent an opponent controls. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
//! Set: SOC #294 — Secrets of Strixhaven Commander | Scryfall ID: aaf258fc-3ba4-4b83-bdbf-10a07e0b6c03 | Oracle ID: ac10d218-f9a6-4058-9cda-a15ca1b0b7b5
// PARTIAL — the destroy half and the search half are both built; the found
// basic land enters tapped, which the card does not print.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASSASSIN_S_TROPHY,
    oracle_id = "ac10d218-f9a6-4058-9cda-a15ca1b0b7b5",
    scryfall_id = "aaf258fc-3ba4-4b83-bdbf-10a07e0b6c03",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Assassin's Trophy",
        mana_cost = mana!("{B}{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "the searched basic land enters tapped — \"put it onto the battlefield\" prints no such word, and Effect::OptionalBasicLandSearchFor is the only variant that reaches a relative player's library"
    ),
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&Filter::ControlledByOpponent)),
            // NOT SUPPORTED: "put it onto the battlefield" untapped. The only
            // variant that lets *another* player search is
            // Effect::OptionalBasicLandSearchFor, which always puts the found
            // basic land onto the battlefield tapped (Path to Exile).
            Effect::OptionalBasicLandSearchFor {
                player: PlayerRel::ControllerOfTarget,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ControlledByOpponent
        )))
    )],
);
