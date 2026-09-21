//! Mystic Sanctuary — (no cost) — Land — Island
//! Oracle: ({T}: Add {U}.)
//! Oracle: This land enters tapped unless you control three or more other Islands.
//! Oracle: When this land enters untapped, you may put target instant or sorcery card from your graveyard on top of your library.
//! Set: SOC #388 — Secrets of Strixhaven Commander | Scryfall ID: 4cd86997-d7b9-4b5b-9488-11f5c679e4d3 | Oracle ID: 17b60106-a4c7-410a-8ac3-ec8e74e29a7c
// IMPLEMENTED — the printed {U}, TappedUnlessCount on three other Islands as
// it enters, and the enters-untapped trigger as a filter on the entering land
// (the printed clause is the trigger's own event, not an intervening `if`).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_ISLAND: Filter = f!(your Filter::HasSubtype(subtypes::land::ISLAND));

card!(
    index = index::MYSTIC_SANCTUARY,
    oracle_id = "17b60106-a4c7-410a-8ac3-ec8e74e29a7c",
    scryfall_id = "4cd86997-d7b9-4b5b-9488-11f5c679e4d3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mystic Sanctuary",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::ISLAND],
        // The entering permanent never counts itself, so the printed "other"
        // is the engine's default and is not restated here. "You control" is
        // not a default and has to be spelled: `Engine::controls_count`
        // walks the whole battlefield.
        enter_modifiers = &[EnterModifier::TappedUnlessCount {
            filter: &YOUR_ISLAND,
            at_least: 3,
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        triggered!(
            Trigger::EntersBattlefield(&Filter::And(&[Filter::This, Filter::Untapped])),
            &[Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&Filter::INSTANT_OR_SORCERY, PlayerRel::You),
            }],
            targets = Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard(
                &Filter::INSTANT_OR_SORCERY,
                PlayerRel::You,
            ))),
        ),
    ],
);
