//! `cards/instants/mv_2/chain_of_plasma.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn chain_of_plasma() -> CardIndex {
    card_index("029d08cb-da4d-4f4e-b667-62712af63260")
}

/// Chain of Plasma deals 3 damage to a target, then lets the targeted player
/// discard a card to copy it with a new target; declining stops the chain.
#[test]
fn chain_of_plasma_deals_three_and_allows_discard_to_copy() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), index::GRIZZLY_BEARS])
        .hand(0, &[chain_of_plasma()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p0_bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let p1_forest = in_hand(&engine, p1, forest()).unwrap();

    cast_from_hand(&mut engine, p0, chain_of_plasma());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    loop {
        pass_until(&mut engine, |e| {
            !matches!(e.pending(), Pending::Priority { .. }) || stack_is_empty(e)
        });
        match engine.pending().clone() {
            Pending::ChooseCards { player, .. } => {
                assert_eq!(player, p1);
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![p1_forest],
                        },
                    )
                    .unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseTargets { player, .. } => {
                assert_eq!(player, p1);
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![p0_bears],
                            players: vec![],
                        },
                    )
                    .unwrap();
                break;
            }
            _ => break,
        }
    }
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(lk::life(&engine, p1), 17);
    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert!(
        on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none(),
        "P1's copy dealt 3 damage to P0's Grizzly Bears"
    );
}
