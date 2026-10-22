//! `cards/instants/mv_1/misinformation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn misinformation() -> CardIndex {
    card_index("03dc8820-ec10-42c1-8200-d323cfcbf2b0")
}

/// Misinformation ({B} Instant, Partial):
/// "Put up to three target cards from an opponent's graveyard on top of their library in any order."
#[test]
fn misinformation_puts_target_card_from_opponent_graveyard_on_top_of_their_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[misinformation(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let own_gy = in_hand(&engine, p0, forest()).unwrap();
    let opp_gy = in_hand(&engine, p1, grizzly_bears()).unwrap();
    {
        let state = engine.dev_state_mut(p0).unwrap();
        state
            .move_object(
                own_gy,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                opp_gy,
                ZoneLocation::Graveyard(p1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, misinformation());

    // Own graveyard card cannot be targeted.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![own_gy],
                    players: vec![],
                },
            )
            .is_err()
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![opp_gy],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .last()
            .copied(),
        Some(opp_gy)
    );
}
