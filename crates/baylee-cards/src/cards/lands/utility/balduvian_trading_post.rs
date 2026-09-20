//! Balduvian Trading Post — (no cost) — Land
//! Oracle: If this land would enter, sacrifice an untapped Mountain instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add {C}{R}.
//! Oracle: {1}, {T}: This land deals 1 damage to target attacking creature.
//! Set: ME2 #226 — Masters Edition II | Scryfall ID: 7eee4911-a654-46c3-a998-a3eac9c31791 | Oracle ID: 7647940e-c99c-401c-ad1d-9ec730f66b6f
// PARTIAL — {T}: Add {C}{R} and the {1}, {T} ping at an attacking creature are
// built; the entry replacement has no variant and is marked below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BALDUVIAN_TRADING_POST,
    oracle_id = "7647940e-c99c-401c-ad1d-9ec730f66b6f",
    scryfall_id = "7eee4911-a654-46c3-a998-a3eac9c31791",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    // NOT SUPPORTED: If this land would enter, sacrifice an untapped Mountain
    // instead. If you do, put this land onto the battlefield. If you don't,
    // put it into its owner's graveyard. — no `EnterModifier` variant asks a
    // question whose no-answer sends the card to the graveyard, and none of
    // them names a permanent to sacrifice.
    faces = &[face!(
        name = "Balduvian Trading Post",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Partial(
        "entry replacement \"sacrifice an untapped Mountain instead, else put this into its graveyard\" has no EnterModifier variant"
    ),
    abilities = &[
        mana_ability!(&[
            Effect::mana(ManaColor::Colorless, 1),
            Effect::mana(ManaColor::Red, 1),
        ]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Object(&Filter::ATTACKING_CREATURE),
            }],
            target = Some(TargetSpec::Object(&Filter::ATTACKING_CREATURE)),
        ),
    ],
);
