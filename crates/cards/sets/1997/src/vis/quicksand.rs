//! Quicksand — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice this land: Target attacking creature without flying gets -1/-2 until end of turn.
//! Set: A25 #245 — Masters 25 | Scryfall ID: ce833a90-7c1d-4c05-ace3-57e974c0769b | Oracle ID: ef2bb4fa-f292-4d19-aaa4-cfbe445caf45
// IMPLEMENTED — {T} for {C}, and the sacrifice ability layers a -1/-2 onto a
// target attacking creature without flying until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::QUICKSAND,
    oracle_id = "ef2bb4fa-f292-4d19-aaa4-cfbe445caf45",
    scryfall_id = "ce833a90-7c1d-4c05-ace3-57e974c0769b",
    faces = &[face!(name = "Quicksand", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(TapSelf, SacrificeSelf),
            &[Effect::PumpTarget {
                power: Amount::NegXFixed(1),
                toughness: Amount::NegXFixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::ATTACKING_CREATURE,
                Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
            ])))
        ),
    ],
);
