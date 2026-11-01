//! `cards/creatures/mv_2/goblin_turncoat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Goblin Turncoat (`297c2ddf-6d2d-4ed4-ab37-716514d5e3b6`) — {1}{B} Creature — Goblin Mercenary 2/1:
/// "Sacrifice a Goblin: Regenerate this creature."
#[test]
fn goblin_turncoat_sacrifices_goblin_to_regenerate() {
    let turncoat_card = card_index("297c2ddf-6d2d-4ed4-ab37-716514d5e3b6");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[turncoat_card, index::RAGING_GOBLIN, mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let turncoat = on_battlefield(&engine, p0, turncoat_card).unwrap();
    let fodder = on_battlefield(&engine, p0, index::RAGING_GOBLIN).unwrap();

    activate(&mut engine, p0, turncoat_card, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, index::RAGING_GOBLIN).is_some());
    assert_eq!(
        engine
            .state()
            .object(turncoat)
            .unwrap()
            .regeneration_shields,
        1
    );

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![turncoat],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, turncoat_card).is_some());
    assert!(
        engine
            .state()
            .object(turncoat)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
