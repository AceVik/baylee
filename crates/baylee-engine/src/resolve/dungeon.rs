//! Dungeon progression and room effects.

#[allow(clippy::wildcard_imports)] // the parent interpreter vocabulary
use super::*;

static DUNGEON_TREASURE: baylee_cards_dsl::TokenDef = baylee_cards_dsl::TokenDef {
    name: "Treasure",
    types: baylee_core::types::TypeSet::ARTIFACT,
    subtypes: &[baylee_core::generated::subtypes::artifact::TREASURE],
    abilities: &[baylee_cards_dsl::mana_ability!(
        baylee_cards_dsl::cost!(TapSelf, SacrificeSelf),
        &[Effect::mana_choice(&[
            baylee_core::mana::ManaColor::White,
            baylee_core::mana::ManaColor::Blue,
            baylee_core::mana::ManaColor::Black,
            baylee_core::mana::ManaColor::Red,
            baylee_core::mana::ManaColor::Green,
        ])]
    )],
    scryfall_id: "0559f9f3-eff0-465d-93c1-e875a8afe87f",
    ..baylee_cards_dsl::TokenDef::DEFAULT
};

static CAVE_ENTRANCE: &[Effect] = &[Effect::scry(1)];
static MINE_TUNNELS: &[Effect] = &[Effect::CreateToken {
    token: &DUNGEON_TREASURE,
}];
static DARK_POOL: &[Effect] = &[
    Effect::LoseLife {
        amount: baylee_cards_dsl::Amount::Fixed(1),
        target: PlayerRel::EachOpponent,
    },
    Effect::gain_life(1),
];
static TEMPLE_OF_DUMATHOIN: &[Effect] = &[Effect::draw(1)];

pub(super) fn venture_into_the_dungeon(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let seat = usize::from(you.get());
    let room = state.players[seat].dungeon_room;
    let room_effects: &'static [Effect] = match room {
        0 => {
            state.players[seat].dungeon_room = 1;
            CAVE_ENTRANCE
        }
        1 => {
            state.players[seat].dungeon_room = 2;
            MINE_TUNNELS
        }
        2 => {
            state.players[seat].dungeon_room = 3;
            DARK_POOL
        }
        _ => {
            state.players[seat].dungeon_room = 0;
            state.players[seat].dungeons_completed =
                state.players[seat].dungeons_completed.saturating_add(1);
            TEMPLE_OF_DUMATHOIN
        }
    };
    run_nested(state, res, room_effects)
}
