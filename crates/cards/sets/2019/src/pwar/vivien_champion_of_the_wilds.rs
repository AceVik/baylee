//! Vivien, Champion of the Wilds — {2}{G} — Legendary Planeswalker — Vivien
//! Oracle: You may cast creature spells as though they had flash.
//! Oracle: +1: Until your next turn, up to one target creature gains reach and vigilance.
//! Oracle: −2: Look at the top three cards of your library. Exile one face down and put the rest on the bottom of your library in any order. For as long as it remains exiled, you may cast it if it's a creature spell.
//! Set: CLB #838 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: e11dc967-ae38-4c19-bf54-b3209e203b39 | Oracle ID: 146eb8c8-93d5-4dd5-af7a-46d5acd4a89d
// IMPLEMENTED — creature flash, temporary reach/vigilance, and private exile casting.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIVIEN_CHAMPION_OF_THE_WILDS,
    oracle_id = "146eb8c8-93d5-4dd5-af7a-46d5acd4a89d",
    scryfall_id = "e11dc967-ae38-4c19-bf54-b3209e203b39",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Vivien, Champion of the Wilds",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::VIVIEN],
        loyalty = Some(4),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            Filter::Any,
            Modifier::SpellsHaveFlash(Some(TypeSet::CREATURE))
        ),
        loyalty!(
            1,
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::REACH.union(KeywordSet::VIGILANCE),
                duration: Duration::UntilYourNextTurn,
            }],
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        loyalty!(
            -2,
            &[Effect::LookAtTopExileFaceDown {
                count: 3,
                cast_types: TypeSet::CREATURE,
            }]
        ),
    ],
);
