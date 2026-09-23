//! Stump Stomp // Burnwillow Clearing — {1}{R/G} — Sorcery // Land
//! Oracle: Target creature you control deals damage equal to its power to target creature or planeswalker you don't control.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R} or {G}.
//! Set: MH3 #259 — Modern Horizons 3 | Scryfall ID: 49974246-0a3b-4ec9-b5ea-2a89df9bb0b5 | Oracle ID: eb7b1284-0b2c-4b6a-a389-b2b932838083
//! Face: Stump Stomp — {1}{R/G} — Sorcery
//! Face: Burnwillow Clearing —  — Land
use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice(&[
    ManaColor::Red,
    ManaColor::Green,
])])];

static CREATURE_OR_PLANESWALKER_YOU_DONT_CONTROL: Filter = f!(not_yours CREATURE_OR_PLANESWALKER);

card!(
    index = index::STUMP_STOMP,
    oracle_id = "eb7b1284-0b2c-4b6a-a389-b2b932838083",
    scryfall_id = "49974246-0a3b-4ec9-b5ea-2a89df9bb0b5",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[
        face!(
            name = "Stump Stomp",
            mana_cost = mana!("{1}{R/G}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Burnwillow Clearing",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::DamageEqualToPower {
            dealer: TargetSlot::First,
            to: TargetSlot::Second,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(
            &CREATURE_OR_PLANESWALKER_YOU_DONT_CONTROL
        ))),
    )],
);
