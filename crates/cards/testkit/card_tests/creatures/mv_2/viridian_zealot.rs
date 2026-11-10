//! `cards/creatures/mv_2/viridian_zealot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Viridian Zealot — {G}{G} 2/1 Elf Warrior: "{1}{G}, Sacrifice this
/// creature: Destroy target artifact or enchantment."
///
/// Four Forests, all tapped before anything is claimed: the {G}{G} brings the
/// Zealot to the table and the two mana left floating are exactly the
/// `{1}{G}` its ability charges, which is read off the pool and not off the
/// untapped lands. The Sol Ring across the table is the artifact the printed
/// word names, and the Elf beside it is the control — a creature is neither
/// an artifact nor an enchantment, so an offer holding exactly the Ring is
/// what reads `Filter::ARTIFACT_OR_ENCHANTMENT` rather than `Filter::Any`.
/// Both halves of the price are read where CR 601.2h puts them: the Zealot is
/// still standing and both green are still floating while the target question
/// is open, and the sacrifice and the mana are gone the moment it is
/// answered.
#[test]
fn viridian_zealot_sacrifices_itself_to_destroy_an_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[viridian_zealot()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, viridian_zealot());
    pass_until(&mut engine, stack_is_empty);
    let zealot = on_battlefield(&engine, p0, viridian_zealot()).expect("the Zealot resolved");
    let their_ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, zealot), (2, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "four Forests less the {{G}}{{G}} the Zealot cost is exactly its {{1}}{{G}}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(zealot, 0)),
        "with the {{1}}{{G}} already floating the whole price is payable, so \
         the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, viridian_zealot(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact or enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&their_ring),
        "the artifact across the table is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&their_elves),
        "and the Elf beside it is a creature, which is neither an artifact \
         nor an enchantment: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, viridian_zealot()).is_some(),
        "targets are chosen before costs are paid (CR 601.2c, then \
         CR 601.2h), so the Zealot has not sacrificed itself yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{1}}{{G}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_ring],
            },
        )
        .expect("the Ring was one of the options it enumerated");

    assert!(
        on_battlefield(&engine, p0, viridian_zealot()).is_none(),
        "\"Sacrifice this creature\" is the last step of the activation"
    );
    assert!(
        in_graveyard(&engine, p0, viridian_zealot()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{G}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact that was named left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its owner's graveyard, not under the seat that aimed it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the permanent the ability did not name never moved"
    );
}
