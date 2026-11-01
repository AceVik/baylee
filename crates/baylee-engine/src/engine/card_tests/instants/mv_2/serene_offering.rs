//! `cards/instants/mv_2/serene_offering.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CRUSADE, GRIZZLY_BEARS};

fn serene_offering() -> CardIndex {
    card_index("6cef0548-6010-4781-8e83-431d014dd1da")
}

/// `Serene Offering` ({1}{W}, Instant, `Coverage::Implemented`):
/// `Destroy target enchantment. You gain life equal to its mana value.`
#[test]
fn serene_offering_destroys_target_enchantment_and_gains_life_equal_to_its_mana_value() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[CRUSADE, GRIZZLY_BEARS])
        .hand(0, &[serene_offering()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crusade = on_battlefield(&engine, p1, CRUSADE).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, serene_offering());
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![]
                }
            )
            .is_err(),
        "only enchantments are targets"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![crusade],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, CRUSADE).is_none());
    assert!(in_graveyard(&engine, p1, CRUSADE).is_some());
    assert_eq!(engine.state().players[0].life, 22);
}
