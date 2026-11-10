//! Finale of Devastation — {X}{G}{G} — Sorcery
//! Oracle: Search your library and/or graveyard for a creature card with mana value X or less and put it onto the battlefield. If you search your library this way, shuffle. If X is 10 or more, creatures you control get +X/+X and gain haste until end of turn.
//! Set: CMM #289 — Commander Masters | Scryfall ID: b10d99bf-b2ce-4443-b924-ff0eb8be1033 | Oracle ID: 69872a9a-fe54-4e58-940c-89395af71acd

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FINALE_OF_DEVASTATION,
    oracle_id = "69872a9a-fe54-4e58-940c-89395af71acd",
    scryfall_id = "b10d99bf-b2ce-4443-b924-ff0eb8be1033",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Finale of Devastation",
        mana_cost = mana!("{X}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[
        Effect::SearchLibraryOrGraveyard {
            // "a creature card with mana value X or less" — Filter::CmcAtMostX
            // reads the announced X off the spell, so the bound is not a
            // constant.
            filter: &Filter::And(&[Filter::CREATURE, Filter::CmcAtMostX]),
            find: &Find::BATTLEFIELD,
        },
        Effect::IfCondition {
            condition: Condition::XAtLeast(10),
            then: &[Effect::PumpFilter {
                filter: &Filter::YOUR_CREATURE,
                controlled_by: None,
                power: Amount::X,
                toughness: Amount::X,
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn,
            }],
            otherwise: &[],
        },
    ])],
);
