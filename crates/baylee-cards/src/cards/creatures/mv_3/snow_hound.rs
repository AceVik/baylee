//! Snow Hound — {2}{W} — Creature — Dog
//! Oracle: {1}, {T}: Return this creature and target green or blue creature you control to their owner's hand.
//! Set: ICE #53 — Ice Age | Scryfall ID: 084437ba-26d4-4af6-ab00-dcb145dd2cd0 | Oracle ID: 2fe8512f-826c-400d-ba6d-1de4f315b617
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GREEN_OR_BLUE_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::YOUR_CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green, Color::Blue])),
]);

card!(
    index = index::SNOW_HOUND,
    oracle_id = "2fe8512f-826c-400d-ba6d-1de4f315b617",
    scryfall_id = "084437ba-26d4-4af6-ab00-dcb145dd2cd0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Snow Hound",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DOG],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf),
        &[
            Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            },
            Effect::ReturnToHand {
                target: TargetSpec::Object(&GREEN_OR_BLUE_CREATURE_YOU_CONTROL),
            },
        ],
        target = Some(TargetSpec::Object(&GREEN_OR_BLUE_CREATURE_YOU_CONTROL)),
    ),],
);
