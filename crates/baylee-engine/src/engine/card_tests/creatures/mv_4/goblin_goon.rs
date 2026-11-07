//! `cards/creatures/mv_4/goblin_goon.rs`, played.

use super::super::super::creature_count_combat as count_combat;
#[allow(clippy::wildcard_imports)]
use super::*;

fn goblin_goon() -> CardIndex {
    card_index("e91bcf01-113b-4532-9f78-2a155b17be4f")
}

#[test]
fn goblin_goon_attacks_only_a_player_it_outnumbers() {
    count_combat::attack_compares_each_defender(goblin_goon());
}

#[test]
fn goblin_goon_blocks_only_when_it_outnumbers_the_attackers_controller() {
    count_combat::block_compares_actual_attackers_controller(goblin_goon());
}
