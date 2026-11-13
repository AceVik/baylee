//! The Black Gate — (no cost) — Legendary Land — Gate
//! Oracle: As The Black Gate enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}, {T}: Choose a player with the most life or tied for most life. Target creature can't be blocked by creatures that player controls this turn.
//! Set: LTC #80 — Tales of Middle-earth Commander | Scryfall ID: 46418186-c215-47c4-9d0a-d15a1d8ca613 | Oracle ID: 40eb9904-dea3-47cf-963a-04821f98ba64
// IMPLEMENTED — pay 3 life as it enters or have it enter tapped
// (EnterModifier::TappedOrPayLife), {T}: Add {B}, and {1}{B}, {T}: choose a
// player with the most life or tied, and target creature can't be blocked by
// that player's creatures this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_BLACK_GATE,
    oracle_id = "40eb9904-dea3-47cf-963a-04821f98ba64",
    scryfall_id = "46418186-c215-47c4-9d0a-d15a1d8ca613",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "The Black Gate",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::land::GATE],
        enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        // The player is chosen as this resolves; life totals that change
        // afterwards do not move the choice.
        activated!(
            cost!("{1}{B}", TapSelf),
            &[
                Effect::ChoosePlayerFrom {
                    among: PlayerRel::MostLifeOrTied,
                },
                Effect::CantBeBlockedByPlayerThisTurn {
                    by: PlayerRel::Chosen,
                },
            ],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
