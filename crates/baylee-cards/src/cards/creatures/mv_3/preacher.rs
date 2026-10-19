//! Preacher — {1}{W}{W} — Creature — Human Cleric
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: For as long as this creature remains tapped, gain control of target creature of an opponent's choice they control.
//! Set: ME1 #24 — Masters Edition | Scryfall ID: d8e6f74c-0c69-4b83-ba4e-4470f50c1fb4 | Oracle ID: d5ea9176-30c1-4152-9086-322f39245f46

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PREACHER,
    oracle_id = "d5ea9176-30c1-4152-9086-322f39245f46",
    scryfall_id = "d8e6f74c-0c69-4b83-ba4e-4470f50c1fb4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Preacher",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            Cost::TAP,
            &[Effect::ModifyTargets {
                modifier: Modifier::GainControl,
                duration: Duration::WhileSourceTapped {
                    control: false,
                    power_limited: false,
                },
            }],
            targets = Some(
                TargetReq::one(TargetSpec::Object(&Filter::OPPONENT_CREATURE)).chosen_by_opponent()
            ),
        ),
    ],
);
