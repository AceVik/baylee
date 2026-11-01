//! `cards/enchantments/mv_4/pillar_tombs_of_aku.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn pillar_tombs_of_aku() -> CardIndex {
    card_index("b8cdf79f-b247-4346-8122-9a2d7d23b3c3")
}

/// Pillar Tombs of Aku on P0's battlefield, cast on P0's first turn; the engine is
/// left in that main phase.
fn pillar_out(p1_board: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, p1_board)
        .hand(0, &[pillar_tombs_of_aku()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, pillar_tombs_of_aku());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, pillar_tombs_of_aku()).is_some());
    engine
}

/// "…that player may sacrifice a creature of their choice. If that player doesn't, they lose 5
/// life and you sacrifice this enchantment." P1 has no creature to sacrifice, so it loses 5 life
/// at its upkeep and the enchantment goes to its controller's graveyard.
#[test]
fn pillar_tombs_of_aku_without_a_creature_loses_5_life_and_the_enchantment_is_sacrificed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = pillar_out(&[]);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 15);
    assert!(
        in_graveyard(&engine, p0, pillar_tombs_of_aku()).is_some(),
        "the enchantment's controller sacrifices it when the player does not"
    );
    assert!(on_battlefield(&engine, p0, pillar_tombs_of_aku()).is_none());
}

/// Sacrificing a creature pays the price: no life is lost and the enchantment stays. It reads
/// each player's upkeep, so it asks the controller at their own upkeep too.
#[test]
fn pillar_tombs_of_aku_a_sacrificed_creature_keeps_life_and_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = pillar_out(&[GRIZZLY_BEARS]);
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[1].life, 20);
    assert!(
        on_battlefield(&engine, p0, pillar_tombs_of_aku()).is_some(),
        "paid with a creature, the enchantment stays"
    );

    // P0's own upkeep: P0 controls no creature, so it loses 5 life and sacrifices the enchantment.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().players[0].life, 15);
    assert!(in_graveyard(&engine, p0, pillar_tombs_of_aku()).is_some());
}

/// "May": a player who has a creature and declines it loses 5 life, keeps the creature, and the
/// enchantment is sacrificed.
#[test]
fn pillar_tombs_of_aku_declining_the_sacrifice_costs_5_life_and_the_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = pillar_out(&[GRIZZLY_BEARS]);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 15);
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, pillar_tombs_of_aku()).is_some());
}
