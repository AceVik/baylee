#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn volrath_s_dungeon() -> CardIndex {
    card_index("a8b968ea-76f3-4ee7-9d53-d251d8a9faf6")
}

#[test]
fn volrath_s_dungeon_can_be_destroyed_by_opponent_paying_five_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[volrath_s_dungeon()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, volrath_s_dungeon());
    pass_until(&mut engine, stack_is_empty);
    let dungeon = on_battlefield(&engine, p0, volrath_s_dungeon()).expect("seated");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        kit::offered(&engine, dungeon).is_empty(),
        "P1 cannot activate during P0's turn"
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::ActivateAbility {
                    source: dungeon,
                    ability_index: 0
                }
            )
            .is_err()
    );
    reach_their_main_phase(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: dungeon,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 15);
    assert!(on_battlefield(&engine, p0, volrath_s_dungeon()).is_none());
    assert!(in_graveyard(&engine, p0, volrath_s_dungeon()).is_some());
}
