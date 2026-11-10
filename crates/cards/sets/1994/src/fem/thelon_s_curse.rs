//! Thelon's Curse — {G}{G} — Enchantment
//! Oracle: Blue creatures don't untap during their controllers' untap steps.
//! Oracle: At the beginning of each player's upkeep, that player may choose any number of tapped blue creatures they control and pay {U} for each creature chosen this way. If the player does, untap those creatures.
//! Set: FEM #77 — Fallen Empires | Scryfall ID: 9b868846-cc3c-4756-a5dd-2335bb380567 | Oracle ID: f6880f71-a05a-4752-ba33-ffbc450606bf
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

card!(
    index = index::THELON_S_CURSE,
    oracle_id = "f6880f71-a05a-4752-ba33-ffbc450606bf",
    scryfall_id = "9b868846-cc3c-4756-a5dd-2335bb380567",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thelon's Curse",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(BLUE_CREATURE, Modifier::DoesNotUntap),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::UntapChosenPayingEach {
                who: PlayerRel::ActivePlayer,
                filter: &BLUE_CREATURE,
                each: mana!("{U}"),
            }],
        ),
    ],
);
