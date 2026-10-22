//! `cards/enchantments/mv_4/tooth_and_claw.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn tooth_and_claw() -> CardIndex {
    card_index("587368eb-068c-44a3-ba8c-5ad0f59f880f")
}

/// `Tooth and Claw` ({3}{R}, Enchantment, `Coverage::Implemented`):
/// `Sacrifice two creatures: Create a 3/1 red Beast creature token named
/// Carnivore.`
#[test]
fn tooth_and_claw_sacrifices_two_creatures_to_create_a_three_one_carnivore_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[tooth_and_claw(), GRIZZLY_BEARS, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == GRIZZLY_BEARS)
        })
        .collect();
    assert_eq!(bears.len(), 2);

    activate(&mut engine, p0, tooth_and_claw(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears[0]],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_none());
    let token = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| engine.state().object(id).unwrap().token.is_some())
        .unwrap();
    let ch = engine.state().object(token).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(1)));
}
