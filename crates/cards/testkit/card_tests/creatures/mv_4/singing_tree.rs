//! `cards/creatures/mv_4/singing_tree.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn singing_tree() -> CardIndex {
    card_index("23f9bee4-ac7e-4828-82c0-576fee0d29b7")
}

/// A board where p0 attacks with a 6/4 Craw Wurm while a Grizzly Bears stays
/// home, and p1 holds a Singing Tree and a Gray Ogre. Returns the engine at
/// the declare-attackers step with p1 holding priority, and the three
/// creatures that matter: `(wurm, bears, tree)`.
fn attack_into_the_tree(pump: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut duel = Duel::new(SEED, forest())
        .battlefield(0, &[craw_wurm(), grizzly_bears(), forest()])
        .battlefield(1, &[singing_tree(), gray_ogre()])
        .life(0, 20)
        .life(1, 20);
    if pump {
        duel = duel.hand(0, &[giant_growth()]);
    }
    let mut engine = duel.start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wurm = on_battlefield(&engine, p0, craw_wurm()).expect("the Wurm is out");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    let tree = on_battlefield(&engine, p1, singing_tree()).expect("the Tree is out");
    assert_eq!(pt(&engine, wurm), (6, 4), "printed 6/4");

    declare_band_attack(&mut engine, p0, p1, &[wurm]);
    // The active player passes first; then the Tree's controller has priority.
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "the defending player may respond to the attack");
    (engine, wurm, bears, tree)
}

/// Singing Tree: "{T}: Target attacking creature has base power 0 until end
/// of turn." An attacker that was declared is a legal target and nothing else
/// is (not a creature that stayed home, not the Tree); the 6/4 Wurm is
/// 0/4 and deals no combat damage.
#[test]
fn singing_tree_zeroes_an_attackers_damage_and_targets_only_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, wurm, bears, tree) = attack_into_the_tree(false);

    activate(&mut engine, p1, singing_tree(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&wurm), "the attacker is a legal target");
    assert!(
        !options.contains(&bears),
        "a creature that is not attacking is not: {options:?}"
    );
    assert!(!options.contains(&tree), "nor is the Tree: {options:?}");
    aim_at(&mut engine, p1, wurm);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wurm), (0, 4), "base power 0, toughness kept");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the unblocked Wurm dealt 0 combat damage"
    );
    assert_eq!(engine.state().players[0].life, 20);
    let _ = p0;
}

/// A pump after the base change adds on top (layer 7c after 7b): 0 + 3.
#[test]
fn singing_tree_leaves_a_pump_applied_afterwards_in_force() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, wurm, _bears, _tree) = attack_into_the_tree(true);

    activate(&mut engine, p1, singing_tree(), 0);
    aim_at(&mut engine, p1, wurm);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wurm), (0, 4));

    // The Wurm's controller has priority after the Tree's ability resolved.
    cast_from_hand(&mut engine, p0, giant_growth());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wurm), (3, 7), "0 + 3, 4 + 3");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "only the pump's three damage"
    );
}
