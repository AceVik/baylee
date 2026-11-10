//! Mudslide — {2}{R} — Enchantment
//! Oracle: Creatures without flying don't untap during their controllers' untap steps.
//! Oracle: At the beginning of each player's upkeep, that player may choose any number of tapped creatures without flying they control and pay {2} for each creature chosen this way. If the player does, untap those creatures.
//! Set: ME2 #136 — Masters Edition II | Scryfall ID: 01199046-ea98-4249-85a2-6258195c2be5 | Oracle ID: ac1d7dfe-1454-46a9-acf6-66ae86b6a6c5
use baylee_cards_dsl::prelude::*;

static CREATURE_WITHOUT_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::MUDSLIDE,
    oracle_id = "ac1d7dfe-1454-46a9-acf6-66ae86b6a6c5",
    scryfall_id = "01199046-ea98-4249-85a2-6258195c2be5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mudslide",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(CREATURE_WITHOUT_FLYING, Modifier::DoesNotUntap),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::UntapChosenPayingEach {
                who: PlayerRel::ActivePlayer,
                filter: &CREATURE_WITHOUT_FLYING,
                each: mana!("{2}"),
            }],
        ),
    ],
);
