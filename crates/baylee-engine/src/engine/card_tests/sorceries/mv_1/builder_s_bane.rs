//! `cards/sorceries/mv_1/builder_s_bane.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BUILDER_S_BANE, JAYEMDAE_TOME, SOL_RING};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Builder's Bane (814adb99-6100-4c36-96d9-763bc20dca5d) — {X}{X}{R} Sorcery:
/// "Destroy X target artifacts. Builder's Bane deals damage to each player equal
/// to the number of artifacts they controlled that were put into a graveyard
/// this way."
#[test]
fn builder_s_bane_destroys_x_artifacts_and_damages_each_controller_for_theirs() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                JAYEMDAE_TOME,
            ],
        )
        .battlefield(1, &[SOL_RING])
        .hand(0, &[BUILDER_S_BANE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p0_art = on_battlefield(&engine, P0, JAYEMDAE_TOME).unwrap();
    let p1_art = on_battlefield(&engine, P1, SOL_RING).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, BUILDER_S_BANE);
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_art, p1_art],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, JAYEMDAE_TOME).is_none());
    assert!(on_battlefield(&engine, P1, SOL_RING).is_none());
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
}
