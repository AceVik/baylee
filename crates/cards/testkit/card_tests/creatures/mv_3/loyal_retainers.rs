//! `cards/creatures/mv_3/loyal_retainers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::zone::ZonePosition;

fn loyal_retainers() -> CardIndex {
    card_index("55b88d43-ec45-4435-a316-1c8aee249ff0")
}

/// Loyal Retainers ({2}{W} 1/1 Human Advisor):
/// "Sacrifice this creature: Return target legendary creature card from your
/// graveyard to the battlefield. Activate only during your turn, before
/// attackers are declared."
#[test]
fn loyal_retainers_sacrifices_before_attackers_to_reanimate_legendary_creature_from_your_graveyard()
{
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), pearled_unicorn()])
        .hand(
            0,
            &[
                loyal_retainers(),
                index::HUA_TUO_HONORED_PHYSICIAN,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(1, &[index::XUN_YU_WEI_ADVISOR])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, lk::P0);

    let my_legend = in_hand(&e, lk::P0, index::HUA_TUO_HONORED_PHYSICIAN).unwrap();
    let my_bear = in_hand(&e, lk::P0, index::GRIZZLY_BEARS).unwrap();
    let their_legend = in_hand(&e, lk::P1, index::XUN_YU_WEI_ADVISOR).unwrap();
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    {
        let state = e.dev_state_mut(lk::P0).unwrap();
        for (id, seat) in [
            (my_legend, lk::P0),
            (my_bear, lk::P0),
            (their_legend, lk::P1),
        ] {
            state
                .move_object(
                    id,
                    ZoneLocation::Graveyard(seat),
                    ZonePosition::Top,
                    baylee_engine::event::Cause::Setup,
                )
                .unwrap();
        }
    }
    e.refresh_offer();

    cast_from_hand(&mut e, lk::P0, loyal_retainers());
    lk::settle(&mut e);

    // Post-attackers and opponent's turn are illegal.
    let _ = lk::attack(&mut e, &[unicorn]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, loyal_retainers(), lk::P0, 0));
    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, loyal_retainers(), lk::P0, 0));

    // Back in P0's FirstMain: offers only your legendary creature card.
    reach_their_main_phase(&mut e, lk::P0);
    activate(&mut e, lk::P0, loyal_retainers(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options, vec![my_legend]);
    lk::aim(&mut e, &[my_legend], &[]);
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P0, loyal_retainers()).is_some());
    assert!(on_battlefield(&e, lk::P0, index::HUA_TUO_HONORED_PHYSICIAN).is_some());
}
