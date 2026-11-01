//! Mole Worms — {2}{B} — Creature — Worm
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: Tap target land. It doesn't untap during its controller's untap step for as long as this creature remains tapped.
//! Set: 5ED #179 — Fifth Edition | Scryfall ID: 700f43c9-1e71-4e5f-a6ce-eca94993558f | Oracle ID: b86ccfc1-a26b-4942-9d77-fef7676c77be
// IMPLEMENTED — optional untap suppression and {T} ability tapping target land and suppressing its untap while Mole Worms remains tapped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOLE_WORMS,
    oracle_id = "b86ccfc1-a26b-4942-9d77-fef7676c77be",
    scryfall_id = "700f43c9-1e71-4e5f-a6ce-eca94993558f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mole Worms",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WORM],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            Cost::TAP,
            &[
                Effect::TapTarget,
                Effect::ModifyTargets {
                    modifier: Modifier::DoesNotUntap,
                    duration: Duration::WhileSourceTapped {
                        control: false,
                        power_limited: false,
                    },
                },
            ],
            target = Some(TargetSpec::Object(&Filter::LAND)),
        ),
    ],
);
