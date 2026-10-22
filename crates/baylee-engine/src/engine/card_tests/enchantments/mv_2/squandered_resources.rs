//! `cards/enchantments/mv_2/squandered_resources.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn squandered_resources() -> CardIndex {
    card_index("620d0572-6d22-43e2-83a9-000e605c8a2b")
}

/// `Squandered Resources` ({B}{G}, Enchantment, `Coverage::Partial`) sacrifices
/// a land to add one mana of any color.
#[test]
fn squandered_resources_sacrifices_a_land_to_add_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), forest()])
        .hand(0, &[squandered_resources()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, forest()).unwrap();
    cast_from_hand(&mut engine, p0, squandered_resources());
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, squandered_resources(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("sacrifice forest");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("choose black mana");

    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
}
