//! Dream Tides — {2}{U}{U} — Enchantment
//! Oracle: Creatures don't untap during their controllers' untap steps.
//! Oracle: At the beginning of each player's upkeep, that player may choose any number of tapped nongreen creatures they control and pay {2} for each creature chosen this way. If the player does, untap those creatures.
//! Set: VIS #31 — Visions | Scryfall ID: 3bd292a0-ec08-4250-8d75-0802e985d6e6 | Oracle ID: d9583aab-3eff-4d88-84d5-7219ab982636
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static NONGREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Green]))),
]);

card!(
    index = index::DREAM_TIDES,
    oracle_id = "d9583aab-3eff-4d88-84d5-7219ab982636",
    scryfall_id = "3bd292a0-ec08-4250-8d75-0802e985d6e6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dream Tides",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(Filter::CREATURE, Modifier::DoesNotUntap),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::UntapChosenPayingEach {
                who: PlayerRel::ActivePlayer,
                filter: &NONGREEN_CREATURE,
                each: mana!("{2}"),
            }],
        ),
    ],
);
