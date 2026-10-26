//! `cards/creatures/mv_5/ertai_the_corrupted.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Ertai, the Corrupted — {2}{W}{U}{B} Legendary Creature — Phyrexian Human Wizard 3/4
/// "{U}, {T}, Sacrifice a creature or enchantment: Counter target spell."
#[test]
fn ertai_the_corrupted_sacrifices_an_enchantment_to_counter_target_spell() {
    let ertai = card_index("36934bd0-b275-4222-926c-b5a74cf0967d");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[ertai, index::CRUSADE, island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));

    let ertai_id = on_battlefield(&engine, p1, ertai).expect("Ertai on board");
    let crusade_id = on_battlefield(&engine, p1, index::CRUSADE).expect("Crusade on board");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::GRIZZLY_BEARS);
    let spell_id = on_stack(&engine, index::GRIZZLY_BEARS).expect("Grizzly Bears on stack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && !stack_is_empty(e)
    });
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: ertai_id,
                ability_index: 0,
            },
        )
        .unwrap();
    // Choose target spell first, then sacrifice a creature or enchantment.
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![spell_id],
                },
            )
            .unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![crusade_id],
                },
            )
            .unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![spell_id],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::CRUSADE).is_none());
    assert!(in_graveyard(&engine, p1, index::CRUSADE).is_some());
    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_some());
}
