//! `cards/instants/mv_2/flash.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{FLASH, SERRA_ANGEL};

const P0: PlayerId = PlayerId::new(0);

/// Flash (3e057471-e415-4105-9064-8236a0a5047d) — {1}{U} Instant:
/// "You may put a creature card from your hand onto the battlefield. If you do,
/// sacrifice it unless you pay its mana cost reduced by up to {2}."
#[test]
fn flash_puts_creature_card_from_hand_onto_battlefield() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[FLASH, SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, FLASH);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == P0),
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards")
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, SERRA_ANGEL).is_some());
}
