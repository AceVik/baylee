//! `cards/sorceries/mv_5/cave_in.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "7d937265-02f5-4f2c-9fef-c5930e13b3be"
fn cave_in() -> CardIndex {
    card_index("7d937265-02f5-4f2c-9fef-c5930e13b3be")
}

/// Cave-In ({3}{R}{R} Sorcery):
/// "You may exile a red card from your hand rather than pay this spell's mana cost."
/// "Cave-In deals 2 damage to each creature and each player."
#[test]
fn cave_in_may_exile_a_red_card_from_hand_and_deals_two_damage_to_each_creature_and_player() {
    let mut engine = lk::game(
        SEED,
        &[index::HILL_GIANT],
        &[index::GRIZZLY_BEARS],
        &[cave_in(), index::LIGHTNING_BOLT, index::FOREST],
        &[],
    );
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let spell = in_hand(&engine, P0, cave_in()).expect("Cave-In in hand");
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).expect("red card in hand");
    let green = in_hand(&engine, P0, index::FOREST).expect("non-red card in hand");

    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .expect("Cave-In is castable via its alternative cost with zero mana");

    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let alt_slot = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
            .expect("alternative cost offered");
        engine
            .apply(P0, PlayerAction::ChooseMode(alt_slot))
            .unwrap();
    }

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseCards for exile cost, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&bolt), "red card in hand is offered");
    assert!(!options.contains(&green), "non-red card is not offered");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();

    lk::settle(&mut engine);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&bolt),
        "pitched red card was exiled"
    );
    assert_eq!(lk::life(&engine, P0), 18, "P0 took 2 damage from Cave-In");
    assert_eq!(lk::life(&engine, P1), 18, "P1 took 2 damage from Cave-In");
    assert_eq!(
        lk::damage_on(&engine, giant),
        2,
        "3/3 Hill Giant has 2 damage marked on it"
    );
    assert!(
        in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "2/2 Grizzly Bears died to 2 damage from Cave-In"
    );
}
