//! `cards/sorceries/mv_2/fanning_the_flames.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fanning_the_flames() -> CardIndex {
    card_index("1d5f0a1c-88e0-4603-97af-8ed47766812e")
}

/// Fanning the Flames (`1d5f0a1c-88e0-4603-97af-8ed47766812e`) — {X}{R}{R} Sorcery (Partial):
/// "This spell deals X damage to any target."
#[test]
fn fanning_the_flames_deals_x_damage_to_any_target() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[fanning_the_flames()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, fanning_the_flames());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, fanning_the_flames()).is_some(),
        "base spell does not get free buyback"
    );

    assert_eq!(engine.state().players[1].life, 17);
}
