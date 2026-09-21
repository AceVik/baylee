//! Daily Bugle Building — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: Smear Campaign — {1}, {T}: Target legendary creature gains menace until end of turn. Activate only as a sorcery.
//! Set: SPM #179 — Marvel's Spider-Man | Scryfall ID: 669bbcb1-0981-40e7-905e-b94e74bc4861 | Oracle ID: 483e0c6c-8131-486c-b482-cc3396c9786b
// IMPLEMENTED — {T} for {C}, {1}/{T} for one mana of any color, and Smear
// Campaign: {1}, {T} at sorcery speed to give a target legendary creature
// menace until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DAILY_BUGLE_BUILDING,
    oracle_id = "483e0c6c-8131-486c-b482-cc3396c9786b",
    scryfall_id = "669bbcb1-0981-40e7-905e-b94e74bc4861",
    faces = &[face!(name = "Daily Bugle Building", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::MENACE,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::LEGENDARY_CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
