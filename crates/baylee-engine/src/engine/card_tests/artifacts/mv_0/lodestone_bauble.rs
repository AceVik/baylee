//! `cards/artifacts/mv_0/lodestone_bauble.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn lodestone_bauble() -> CardIndex {
    card_index("6048b9ac-7d5e-4486-a89a-30adb45aab27")
}

/// Lodestone Bauble ({0} Artifact, Partial):
/// "{1}, {T}, Sacrifice this artifact: Put up to four target basic land cards from a player's
/// graveyard on top of their library in any order. That player draws a card at the beginning of
/// the next turn's upkeep."
#[test]
fn lodestone_bauble_puts_target_basic_land_on_top_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[lodestone_bauble(), plains()])
        .hand(0, &[forest(), GRIZZLY_BEARS])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bauble = on_battlefield(&engine, p0, lodestone_bauble()).unwrap();
    let gy_forest = in_hand(&engine, p0, forest()).unwrap();
    let gy_bears = in_hand(&engine, p0, GRIZZLY_BEARS).unwrap();
    let opp_forest = in_hand(&engine, p1, forest()).unwrap();
    {
        let state = engine.dev_state_mut(p0).unwrap();
        for (card, owner) in [(gy_forest, p0), (gy_bears, p0), (opp_forest, p1)] {
            state
                .move_object(
                    card,
                    ZoneLocation::Graveyard(owner),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
    }
    engine.refresh_offer();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bauble,
                ability_index: 0,
            },
        )
        .unwrap();

    // A basic land in the opponent's graveyard is not offered: "that player
    // draws" cannot name them, so the card takes only your own graveyard.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![opp_forest],
                    players: vec![],
                },
            )
            .is_err()
    );
    // Nonbasic/nonland cards in graveyard cannot be targeted.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![gy_bears],
                    players: vec![],
                },
            )
            .is_err()
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gy_forest],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, lodestone_bauble()).is_some());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(gy_forest)
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let opp_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        opp_hand_before,
        "the opponent draws nothing"
    );
    assert_eq!(
        engine.state().object(opp_forest).unwrap().zone,
        Zone::Graveyard,
        "their land stays where it was"
    );
}
