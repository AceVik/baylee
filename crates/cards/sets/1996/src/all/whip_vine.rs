//! Whip Vine — {2}{G} — Creature — Plant Wall
//! Oracle: Defender; reach (This creature can block creatures with flying.)
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: Tap target creature with flying blocked by this creature. That creature doesn't untap during its controller's untap step for as long as this creature remains tapped.
//! Set: ALL #103a — Alliances | Scryfall ID: 4b66b9fe-47f1-4786-96d5-981d62012663 | Oracle ID: 4e90ddce-1765-44e4-9b3c-e576f3a1bcd4
// IMPLEMENTED — Defender, reach, may choose not to untap, and {T} to tap and lock a flyer blocked by this creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FLYING_BLOCKED_BY_SOURCE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasKeyword(KeywordSet::FLYING),
    Filter::BlockedBySource,
]);

card!(
    index = index::WHIP_VINE,
    oracle_id = "4e90ddce-1765-44e4-9b3c-e576f3a1bcd4",
    scryfall_id = "4b66b9fe-47f1-4786-96d5-981d62012663",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::DEFENDER.union(KeywordSet::REACH),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Whip Vine",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PLANT, subtypes::creature::WALL],
        power = Some(1),
        toughness = Some(4),
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
            target = Some(TargetSpec::Object(&FLYING_BLOCKED_BY_SOURCE)),
        ),
    ],
);
