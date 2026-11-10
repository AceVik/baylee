//! Monsoon — {2}{R}{G} — Enchantment
//! Oracle: At the beginning of each player's end step, tap all untapped Islands that player controls and this enchantment deals X damage to the player, where X is the number of Islands tapped this way.
//! Set: ICE #298 — Ice Age | Scryfall ID: 254fcc50-79a5-40cd-b028-e78dde3f8480 | Oracle ID: a86e7de8-8448-406c-b162-22c62bdc3813
// IMPLEMENTED — end step trigger tapping all untapped Islands the active player controls and dealing that much damage to that player.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ACTIVE_PLAYER_UNTAPPED_ISLAND: Filter = Filter::And(&[
    Filter::ControlledByActivePlayer,
    Filter::HasSubtype(subtypes::land::ISLAND),
    Filter::Untapped,
]);

card!(
    index = index::MONSOON,
    oracle_id = "a86e7de8-8448-406c-b162-22c62bdc3813",
    scryfall_id = "254fcc50-79a5-40cd-b028-e78dde3f8480",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Monsoon",
        mana_cost = mana!("{2}{R}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::EachPlayer,
        },
        &[
            Effect::DealDamage {
                amount: Amount::CountOf {
                    zone: ZoneSel::Battlefield,
                    filter: &ACTIVE_PLAYER_UNTAPPED_ISLAND,
                },
                target: TargetSpec::Player(PlayerRel::ActivePlayer),
            },
            Effect::TapAll {
                filter: &ACTIVE_PLAYER_UNTAPPED_ISLAND,
            },
        ],
    )],
);
