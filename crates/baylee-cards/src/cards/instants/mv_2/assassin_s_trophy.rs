//! Assassin's Trophy — {B}{G} — Instant
//! Oracle: Destroy target permanent an opponent controls. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
//! Set: SOC #294 — Secrets of Strixhaven Commander | Scryfall ID: aaf258fc-3ba4-4b83-bdbf-10a07e0b6c03 | Oracle ID: ac10d218-f9a6-4058-9cda-a15ca1b0b7b5
// IMPLEMENTED — the destroy, then the destroyed permanent's controller may
// search their own library for a basic land, onto the battlefield untapped.

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
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&Filter::ControlledByOpponent)),
            Effect::SearchLibraryOf {
                library: PlayerRel::ControllerOfTarget,
                owner_searches: true,
                filter: &Filter::BASIC_LAND,
                mana_value: None,
                finds: &[Find::BATTLEFIELD],
                optional: true,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ControlledByOpponent
        )))
    )],
);
