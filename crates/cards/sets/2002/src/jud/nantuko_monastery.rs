//! Nantuko Monastery — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: Threshold — {G}{W}: This land becomes a 4/4 green and white Insect Monk creature with first strike until end of turn. It's still a land. Activate only if there are seven or more cards in your graveyard.
//! Set: DMR #252 — Dominaria Remastered | Scryfall ID: 4188a561-46e3-4ddd-8797-f33c37e9adb2 | Oracle ID: c6de0ee9-785d-4cd8-8a7f-5bb715763131
// IMPLEMENTED — {T}: Add {C}, and the {G}{W} animation into a 4/4 green and
// white Insect Monk with first strike, gated on threshold. The gate is the
// half that had no shape: ungated, this is a land that attacks for four.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NANTUKO_MONASTERY,
    oracle_id = "c6de0ee9-785d-4cd8-8a7f-5bb715763131",
    scryfall_id = "4188a561-46e3-4ddd-8797-f33c37e9adb2",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(name = "Nantuko Monastery", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // "It's still a land" is the default and is written nowhere: an
        // `AddType` adds, so nothing here takes the land type away. The
        // colour is `SetColor` and not `AddColor` — the printed animation
        // says the creature *is* green and white, and a land is colourless
        // to begin with, so the two agree here and would not on a coloured
        // permanent.
        activated!(
            cost!("{G}{W}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::INSECT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::MONK),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::from_slice(&[Color::Green, Color::White])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(4, 4),
                    Duration::UntilEndOfTurn,
                ),
            ],
            condition = Some(Condition::GraveyardCountAtLeast(7)),
        ),
    ],
);
