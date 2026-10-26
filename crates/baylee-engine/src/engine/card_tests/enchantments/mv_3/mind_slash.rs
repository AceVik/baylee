//! `cards/enchantments/mv_3/mind_slash.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn mind_slash_sacrifices_creature_to_reveal_opponent_hand_and_discard_chosen_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                card_index("47e79913-322e-4ba6-9bf8-3b4c1c82a6c4"),
                swamp(),
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(1, &[index::HILL_GIANT, index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    let opp_giant = in_hand(&engine, p1, index::HILL_GIANT).expect("Hill Giant in p1 hand");
    tap_all_mana(&mut engine, p0);
    activate(
        &mut engine,
        p0,
        card_index("47e79913-322e-4ba6-9bf8-3b4c1c82a6c4"),
        0,
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("target opponent");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .expect("sacrifice Grizzly Bears");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_giant],
            },
        )
        .expect("choose Hill Giant from revealed hand");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS), None);
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
}
