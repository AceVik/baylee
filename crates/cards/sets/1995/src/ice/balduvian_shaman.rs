//! Balduvian Shaman — {U} — Creature — Human Cleric Shaman
//! Oracle: {T}: Change the text of target white enchantment you control that doesn't have cumulative upkeep by replacing all instances of one color word with another. (For example, you may change "black creatures can't attack" to "blue creatures can't attack.") That enchantment gains "Cumulative upkeep {1}." (At the beginning of its controller's upkeep, that player puts an age counter on it, then sacrifices it unless they pay its upkeep cost for each age counter on it.)
//! Set: ICE #59 — Ice Age | Scryfall ID: 74859723-8ddf-4ee6-a0a7-87192c84e8ad | Oracle ID: 3354c3d3-5713-4139-beb2-4fa17f6b47d9

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TARGET_ENCHANTMENT: Filter = Filter::And(&[
    Filter::ENCHANTMENT,
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::Not(&Filter::HasCumulativeUpkeep),
]);

card!(
    index = index::BALDUVIAN_SHAMAN,
    oracle_id = "3354c3d3-5713-4139-beb2-4fa17f6b47d9",
    scryfall_id = "74859723-8ddf-4ee6-a0a7-87192c84e8ad",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Balduvian Shaman",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::CLERIC,
            subtypes::creature::SHAMAN
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[
            Effect::ChangeTextWord {
                kind: TextWordKind::Color,
            },
            Effect::continuous(
                &Filter::Reference,
                Modifier::GrantTriggered {
                    trigger: Trigger::StepBegin {
                        step: StepKind::Upkeep,
                        whose: PlayerRel::You,
                    },
                    effects: &[
                        Effect::AddCounter {
                            kind: counters::AGE,
                            amount: Amount::Fixed(1),
                        },
                        Effect::PlayerMayPayOr {
                            player: PlayerRel::You,
                            mana: Amount::CountersOnSource(counters::AGE),
                            effect: &Effect::SacrificeSelf,
                        },
                    ],
                    target: None,
                },
                Duration::Indefinitely,
            ),
        ],
        target = Some(TargetSpec::Object(&TARGET_ENCHANTMENT))
    )],
);
