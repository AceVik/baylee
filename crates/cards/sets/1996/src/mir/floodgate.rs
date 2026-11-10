//! Floodgate — {3}{U} — Creature — Wall
//! Oracle: Defender
//! Oracle: When this creature has flying, sacrifice it.
//! Oracle: When this creature leaves the battlefield, it deals damage to each nonblue creature without flying equal to half the number of Islands you control, rounded down.
//! Set: DMR #49 — Dominaria Remastered | Scryfall ID: 600b315b-58f8-4b1b-b210-f09e3e5e3b27 | Oracle ID: f5a85674-fd09-42d8-bba7-5656be25fcea
// IMPLEMENTED — defender, sacrifices itself when it has flying, and on LTB deals damage to each nonblue creature without flying equal to half the number of Islands you control, rounded down.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static HAS_FLYING: Condition = Condition::SourceMatches(&Filter::HasKeyword(KeywordSet::FLYING));

static NONBLUE_WITHOUT_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Blue]))),
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

static ISLANDS_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::ISLAND),
    Filter::ControlledByYou,
]);

static HALF_ISLANDS_ROUNDED_DOWN: Amount = Amount::Half {
    base: &Amount::CountOf {
        filter: &ISLANDS_YOU_CONTROL,
        zone: ZoneSel::Battlefield,
    },
    round_up: false,
};

card!(
    index = index::FLOODGATE,
    oracle_id = "f5a85674-fd09-42d8-bba7-5656be25fcea",
    scryfall_id = "600b315b-58f8-4b1b-b210-f09e3e5e3b27",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Floodgate",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(5),
        keywords = KeywordSet::DEFENDER,
    ),],
    abilities = &[
        triggered!(Trigger::State(&HAS_FLYING), &[Effect::SacrificeSelf]),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::DealDamageEach {
                amount: HALF_ISLANDS_ROUNDED_DOWN,
                filter: &NONBLUE_WITHOUT_FLYING,
            }],
        ),
    ],
);
