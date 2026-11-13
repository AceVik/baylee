//! Legion Leadership // Legion Stronghold — {1}{R/W} — Instant // Land
//! Oracle: Until end of turn, double target creature's power and it gains first strike.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R} or {W}.
//! Set: MH3 #255 — Modern Horizons 3 | Scryfall ID: 7676abd9-0a3d-4721-b17b-778d2e3c2e25 | Oracle ID: ad225ec2-ff3a-48f6-81a7-dfdd1b75e1f7
//! Face: Legion Leadership — {1}{R/W} — Instant
//! Face: Legion Stronghold —  — Land
// IMPLEMENTED — the front face doubles a target creature's power
// (Amount::TargetPower, which is the target's current power) and grants it
// first strike until end of turn; the back face enters tapped and taps for
// {R} or {W}.

use baylee_cards_dsl::prelude::*;

/// `{T}: Add {R} or {W}.` — the back face's only ability.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice(&[
    ManaColor::Red,
    ManaColor::White
])])];

card!(
    index = index::LEGION_LEADERSHIP,
    oracle_id = "ad225ec2-ff3a-48f6-81a7-dfdd1b75e1f7",
    scryfall_id = "7676abd9-0a3d-4721-b17b-778d2e3c2e25",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[
        face!(
            name = "Legion Leadership",
            mana_cost = mana!("{1}{R/W}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Legion Stronghold",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::TargetPower,
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::FIRST_STRIKE,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
