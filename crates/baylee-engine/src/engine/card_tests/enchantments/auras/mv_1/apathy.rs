//! `cards/enchantments/auras/mv_1/apathy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn apathy() -> CardIndex {
    card_index("09ac5adf-f722-417e-b0fc-7ed0d8c9abd6")
}

/// `Apathy` ({U}, Enchantment — Aura, `Coverage::Partial`) enchants a creature
/// and prevents it from untapping during its controller's untap step.
#[test]
fn apathy_prevents_enchanted_creature_from_untapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), GRIZZLY_BEARS])
        .hand(0, &[apathy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(bears)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    cast_from_hand(&mut engine, p0, apathy());
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

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
