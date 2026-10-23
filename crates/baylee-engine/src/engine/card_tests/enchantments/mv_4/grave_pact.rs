//! `cards/enchantments/mv_4/grave_pact.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grave_pact() -> CardIndex {
    card_index("6f4ac4a4-53ec-4bc9-8f5c-d4b801d867b2")
}

/// Grave Pact (`6f4ac4a4-53ec-4bc9-8f5c-d4b801d867b2`) — {1}{B}{B}{B} Enchantment (Implemented):
/// "Whenever a creature you control dies, each other player sacrifices a creature of their choice."
#[test]
fn grave_pact_forces_each_other_player_to_sacrifice_a_creature_when_yours_dies() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grave_pact(), index::GRIZZLY_BEARS, mountain()])
        .battlefield(1, &[index::LLANOWAR_ELVES])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, my_bear);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    {
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::LLANOWAR_ELVES).is_none());
    assert!(in_graveyard(&engine, P1, index::LLANOWAR_ELVES).is_some());
}

#[test]
fn opponent_creature_death_does_not_make_you_sacrifice() {
    let mut e = lk::game(
        SEED,
        &[grave_pact(), index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[lightning_bolt()],
        &[],
    );
    let enemy = lk::obj(&e, P1, index::GRIZZLY_BEARS);
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, P0, lightning_bolt(), &[enemy], &[]);
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P1, index::LLANOWAR_ELVES).is_some());
}
