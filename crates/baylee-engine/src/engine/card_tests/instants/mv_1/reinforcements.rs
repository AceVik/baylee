//! `cards/instants/mv_1/reinforcements.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn reinforcements() -> CardIndex {
    card_index("ff27ff96-afd2-45df-b799-3046179e27c6")
}

/// Reinforcements ({W} Instant, Partial):
/// "Put up to three target creature cards from your graveyard on top of your library."
#[test]
fn reinforcements_puts_target_creature_card_from_your_graveyard_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .hand(0, &[reinforcements(), grizzly_bears(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gy_bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let gy_plains = in_hand(&engine, p0, plains()).unwrap();
    {
        let state = engine.dev_state_mut(p0).unwrap();
        for card in [gy_bears, gy_plains] {
            state
                .move_object(
                    card,
                    ZoneLocation::Graveyard(p0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
    }
    engine.refresh_offer();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, reinforcements());

    // Noncreature card in graveyard cannot be targeted.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![gy_plains],
                    players: vec![],
                },
            )
            .is_err()
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gy_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(gy_bears)
    );
}
