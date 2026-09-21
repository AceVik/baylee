//! Flamekin Village — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Elemental card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: {R}, {T}: Target creature gains haste until end of turn.
//! Set: ECC #149 — Lorwyn Eclipsed Commander | Scryfall ID: df0affe7-92d8-422f-833d-74089626b829 | Oracle ID: 34a1eb04-08f6-49d8-a1d1-b987a76bd8b1
// PARTIAL — tap for {R} and {R}, {T}: target creature gains haste until end of turn;
// revealing an Elemental card from hand on entering is not expressible in EnterModifier.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLAMEKIN_VILLAGE,
    oracle_id = "34a1eb04-08f6-49d8-a1d1-b987a76bd8b1",
    scryfall_id = "df0affe7-92d8-422f-833d-74089626b829",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "EnterModifier has no variant for revealing a card from hand as a condition to enter untapped",
    ),
    faces = &[face!(name = "Flamekin Village", types = TypeSet::LAND,)],
    abilities = &[
        // NOT SUPPORTED: As this land enters, you may reveal an Elemental card from your hand. If you don't, this land enters tapped.
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{R}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
