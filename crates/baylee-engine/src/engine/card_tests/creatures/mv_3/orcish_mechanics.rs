//! `cards/creatures/mv_3/orcish_mechanics.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Orcish Mechanics — {2}{R}, a 1/1 Orc printing one line: "{T}, Sacrifice an
/// artifact: This creature deals 2 damage to any target."
///
/// The cost names no artifact in particular, so the engine has to ask which
/// one, and that question is half the card: the Sol Ring under the same seat is
/// the whole of its menu — the Mechanics is a creature and no artifact, the Elf
/// beside it is a creature too, and the Sol Ring across the table is not this
/// seat's to give up (CR 701.21a). The other half is that "any target"
/// (CR 115.4) is one choice carrying both the object and the player lists, so an
/// activation aimed at the Elf has to leave the opponent at twenty: the two
/// damage went where the answer pointed and nowhere else. The whole price is a
/// tap and an artifact and no mana at all, so the board holds an empty pool and
/// nothing on it is tapped by hand.
#[allow(clippy::too_many_lines)] // one activation, and every gate it passes asserted
#[test]
fn orcish_mechanics_eats_an_artifact_of_yours_for_two_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[orcish_mechanics(), quiet_artifact(), llanowar_elves()])
        // An Elf to aim at, and an artifact across the table that "sacrifice
        // an artifact" is not an invitation to eat.
        .battlefield(1, &[llanowar_elves(), quiet_artifact()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mechanics = on_battlefield(&engine, p0, orcish_mechanics()).expect("the Mechanics is out");
    let fodder = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let ours = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let prey = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    assert_eq!(pt(&engine, mechanics), (1, 1), "the body the card prints");
    let kinds = types(&engine, mechanics);
    assert!(
        kinds.contains(TypeSet::CREATURE) && !kinds.contains(TypeSet::ARTIFACT),
        "and it is a creature and no artifact, which is why its own cost can \
         never eat it: {kinds:?}"
    );
    assert_eq!(
        pt(&engine, prey),
        (1, 1),
        "a printed 1/1 for two damage to kill"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the price is a tap and an artifact: there is no mana anywhere on this board"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mechanics, 0)),
        "an untapped Orc with an artifact to give up is offered its one line \
         with nothing floating: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, orcish_mechanics(), 0);

    // Two questions, and the order they arrive in is threaded rather than
    // assumed: the target is chosen at CR 601.2c and the sacrifice is paid at
    // CR 601.2h, and both answers are taken out of what the question itself
    // enumerated.
    let mut menu: Vec<ObjectId> = Vec::new();
    let mut aimed = false;
    for _ in 0..12 {
        if aimed && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat aims it");
                assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
                assert!(
                    options.contains(&prey),
                    "the creature across the table is one of the object options: {options:?}"
                );
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
                );
                if menu.is_empty() {
                    // Nothing has been given up yet: the target is named first
                    // and the costs are the last step of the activation
                    // (CR 601.2c, then CR 601.2h).
                    assert!(
                        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
                        "the sacrifice is a cost and is paid after the target"
                    );
                    assert!(!is_tapped(&engine, mechanics), "and so is the tap symbol");
                }
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![prey],
                        },
                    )
                    .expect("the creature was one of the options it enumerated");
                aimed = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat answers its own cost");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
                menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the artifact the question offered pays the cost");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected while the Mechanics' activation resolves: {other:?}"),
        }
    }
    assert!(aimed, "\"any target\" asks where the damage goes");
    assert!(
        !menu.contains(&theirs),
        "`CR 701.21a`: an opponent's artifact is not yours to sacrifice: {menu:?}"
    );
    assert!(
        !menu.contains(&ours) && !menu.contains(&mechanics),
        "and neither creature is an artifact: {menu:?}"
    );
    assert_eq!(
        menu,
        vec![fodder],
        "so the one artifact this seat controls is the whole menu"
    );

    assert!(
        is_tapped(&engine, mechanics),
        "{{T}} paid the other half of the cost"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature the answer pointed at and not to the \
         player whose board it stood on"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the sacrificed artifact is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, orcish_mechanics()).is_some(),
        "the Mechanics ate the Sol Ring and not itself: its own source is no artifact"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature nobody named never moved"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the permanent across the table was never on the menu"
    );
}
