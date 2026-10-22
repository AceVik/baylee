//! `cards/enchantments/mv_2/angelic_renewal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn angelic_renewal() -> CardIndex {
    card_index("f86bcdde-8b1f-4efa-a01d-39e09dc826d0")
}

/// `Angelic Renewal` ({1}{W}, Enchantment) may sacrifice itself when a creature
/// is put into your graveyard from the battlefield to return that card to the
/// battlefield.
#[test]
fn angelic_renewal_may_sacrifice_itself_to_return_dying_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), mountain(), GRIZZLY_BEARS])
        .hand(0, &[angelic_renewal(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, angelic_renewal());
    pass_until(&mut engine, stack_is_empty);
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, angelic_renewal()).is_some());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
}
