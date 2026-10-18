//! `cards/creatures/mv_4/gemrazer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gemrazer ({3}{G}) cast normally arrives as a 4/4 Beast with reach and trample.
#[test]
fn gemrazer_arrives_as_a_four_four_beast_with_reach_and_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[gemrazer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        in_hand(&engine, p0, gemrazer()).is_some(),
        "the card is in hand before anything is cast"
    );
    cast_from_hand(&mut engine, p0, gemrazer());
    pass_until(&mut engine, stack_is_empty);

    let beast = on_battlefield(&engine, p0, gemrazer()).expect("Gemrazer resolved");
    assert!(
        in_hand(&engine, p0, gemrazer()).is_none(),
        "and it left the hand to get there"
    );
    assert!(
        types(&engine, beast).contains(TypeSet::CREATURE),
        "it is a creature permanent: {:?}",
        types(&engine, beast)
    );
    assert_eq!(pt(&engine, beast), (4, 4), "the body the card prints");
    let kw = keywords(&engine, beast);
    assert!(kw.contains(KeywordSet::REACH), "Gemrazer has reach");
    assert!(kw.contains(KeywordSet::TRAMPLE), "Gemrazer has trample");
}

#[test]
fn gemrazer_mutates_onto_non_human_creature_and_destroys_opponent_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(84, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[sol_ring()])
        .hand(0, &[gemrazer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("Sol Ring on opponent battlefield");
    let card = in_hand(&engine, p0, gemrazer()).expect("Gemrazer in hand");

    // Tap the 3 Forests (leaving Llanowar Elves untapped) so {G}{G}{G} is in the pool:
    // only Mutate ({1}{G}{G}) is affordable, not the normal {3}{G} cost.
    tap_mana_except(&mut engine, p0, elf);
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("begin casting Gemrazer via mutate");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected mutate target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&elf),
        "non-Human creature owned by p0 is a legal mutate target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("choose Llanowar Elves as mutate target");

    // Resolve the mutating spell, which triggers "Whenever this creature mutates,
    // destroy target artifact or enchantment an opponent controls."
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected mutate trigger target choice, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&ring),
        "opponent's Sol Ring is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("target opponent's Sol Ring");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, sol_ring()).is_none(),
        "opponent's Sol Ring was destroyed by the mutate trigger"
    );
    assert_eq!(
        pt(&engine, elf),
        (4, 4),
        "merged creature has Gemrazer's 4/4 characteristics"
    );
    let kw = keywords(&engine, elf);
    assert!(kw.contains(KeywordSet::REACH));
    assert!(kw.contains(KeywordSet::TRAMPLE));
}
