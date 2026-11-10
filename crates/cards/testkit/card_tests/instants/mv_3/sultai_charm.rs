//! `cards/instants/mv_3/sultai_charm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sultai Charm (`Coverage::Implemented`): "Choose one — • Destroy target
/// monocolored creature. • Destroy target artifact or enchantment. • Draw two
/// cards, then discard a card."
///
/// The test casts Sultai Charm, selects the first mode ("Destroy target
/// monocolored creature"), targets an opponent's monocolored Llanowar Elves,
/// and verifies that the creature is destroyed upon resolution.
#[test]
fn sultai_charm_destroys_target_monocolored_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(47, forest())
        .battlefield(0, &[swamp(), forest(), island()])
        .hand(0, &[sultai_charm()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's Elf");

    cast_from_hand(&mut engine, p0, sultai_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .expect("mode 0 is offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(slot))
        .expect("chooses mode 0");

    let Pending::ChooseTargets {
        options: targets, ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        targets.contains(&elf),
        "target monocolored creature — the Elf qualifies"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted monocolored creature was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the destroyed creature is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, sultai_charm()).is_some(),
        "the resolved charm is in its caster's graveyard"
    );
}

/// The copy of a choose-one spell copies its one mode too (CR 700.2g).
/// Storm of Saruman copies a Sultai Charm cast for "Draw two cards, then
/// discard a card", and the copy draws and discards as well. The copy used
/// to carry no mode, and a spell whose every effect sits under a mode then
/// resolved to nothing.
#[test]
fn sultai_charm_copied_keeps_the_mode_chosen_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[swamp(), swamp(), forest(), island(), storm_of_saruman()],
        )
        .hand(0, &[dark_ritual(), sultai_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    let library = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, sultai_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected the modes, got {:?}", engine.pending())
    };
    let draw = options
        .iter()
        .position(|o| o.kind == CastModeKind::Mode(2))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(draw)).unwrap();
    let mut discards = 0;
    while !stack_is_empty(&engine) {
        if let Pending::ChooseCards {
            player,
            options,
            min,
            ..
        } = engine.pending().clone()
        {
            assert_eq!(player, p0);
            discards += 1;
            let objects = options.into_iter().take(usize::from(min)).collect();
            engine
                .apply(p0, PlayerAction::ChooseObjects { objects })
                .unwrap();
            continue;
        }
        let (player, action) = answer_one(&engine).unwrap();
        engine.apply(player, action).unwrap();
    }
    assert_eq!(library_size(&engine, p0), library - 4, "drew two, twice");
    assert_eq!(discards, 2, "and discarded once each time");
}
