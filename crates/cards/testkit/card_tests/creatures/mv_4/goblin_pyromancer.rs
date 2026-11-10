//! `cards/creatures/mv_4/goblin_pyromancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Goblin Pyromancer (`696ee797-0c32-4bff-8c3d-02d05835b138`) — {3}{R} Creature — Goblin Wizard 2/2:
/// "When this creature enters, Goblin creatures get +3/+0 until end of turn.
/// At the beginning of the end step, destroy all Goblins."
#[test]
fn goblin_pyromancer_pumps_goblins_on_etb_and_destroys_all_goblins_at_end_step() {
    let pyromancer_card = card_index("696ee797-0c32-4bff-8c3d-02d05835b138");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[pyromancer_card])
        .battlefield(
            0,
            &[
                index::RAGING_GOBLIN,
                index::GRIZZLY_BEARS,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[index::MONS_S_GOBLIN_RAIDERS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let raging = on_battlefield(&engine, p0, index::RAGING_GOBLIN).unwrap();
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let opp_goblin = on_battlefield(&engine, p1, index::MONS_S_GOBLIN_RAIDERS).unwrap();
    assert_eq!(pt(&engine, raging), (1, 1));
    assert_eq!(pt(&engine, opp_goblin), (1, 1));

    cast_from_hand(&mut engine, p0, pyromancer_card);
    pass_until(&mut engine, stack_is_empty);

    let pyromancer = on_battlefield(&engine, p0, pyromancer_card).unwrap();
    // All Goblin creatures (including Pyromancer and opponent's Goblin) get +3/+0 until end of turn; non-Goblins do not.
    assert_eq!(pt(&engine, pyromancer), (5, 2));
    assert_eq!(pt(&engine, raging), (4, 1));
    assert_eq!(pt(&engine, opp_goblin), (4, 1));
    assert_eq!(pt(&engine, bears), (2, 2));

    // At the beginning of the end step, all Goblins are destroyed, while Grizzly Bears survives.
    reach_their_main_phase(&mut engine, p1);
    assert!(in_graveyard(&engine, p0, pyromancer_card).is_some());
    assert!(in_graveyard(&engine, p0, index::RAGING_GOBLIN).is_some());
    assert!(in_graveyard(&engine, p1, index::MONS_S_GOBLIN_RAIDERS).is_some());
    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_some());
}
