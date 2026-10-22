//! `cards/sorceries/mv_5/living_death.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn living_death() -> CardIndex {
    card_index("9e6a3df4-67a3-452e-a6ef-f04dbadb21ef")
}

/// Living Death (`9e6a3df4-67a3-452e-a6ef-f04dbadb21ef`) — {3}{B}{B} Sorcery (Partial):
/// "Each player exiles all creature cards from their graveyard, then sacrifices all creatures they control, then puts all cards they exiled this way onto the battlefield."
#[test]
fn living_death_returns_creature_cards_from_each_players_graveyard_to_battlefield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[lightning_bolt(), living_death()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());

    cast_from_hand(&mut engine, P0, living_death());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
