//! Vodalian Hypnotist — {1}{U} — Creature — Merfolk Wizard
//! Oracle: {2}{B}, {T}: Target player discards a card. Activate only as a sorcery.

use super::*;
use baylee_core::generated::index;

#[test]
fn vodalian_hypnotist_makes_target_player_discard_a_card_at_sorcery_speed_only() {
    let hypnotist = card_index("586d64c7-6aa9-42a0-9d8a-9dece52bc5d6");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[hypnotist, island(), island(), swamp()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let hyp_id = on_battlefield(&engine, p0, hypnotist).expect("Vodalian Hypnotist on board");
    let bears_in_hand = in_hand(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, hyp_id), (1, 1));

    // Cannot activate during P1's turn even with mana floating.
    assert!(walk_to_own_main(&mut engine, p1));
    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: hyp_id,
                    ability_index: 0,
                },
            )
            .is_err()
    );

    // On P0's main phase with empty stack, activates cleanly.
    assert!(walk_to_own_main(&mut engine, p0));
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: hyp_id,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1)
            || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![bears_in_hand],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
}
