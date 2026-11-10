//! `cards/artifacts/mv_4/fodder_cannon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fodder Cannon — {4} artifact: "{4}, {T}, Sacrifice a creature: This
/// artifact deals 4 damage to target creature." The board reads both filters
/// the sentence turns on: "a creature" is a sacrifice menu holding the Elf I
/// control and not the same Llanowar Elves standing across the table
/// (CR 701.21a), while "target creature" is a menu holding either, because
/// `Filter::CREATURE` names no side of the battlefield. Each price lands
/// where a zone can show it — the {4} out of a pool only the eight tapped
/// Forests filled, the {T} on the artifact itself — and four damage on a
/// printed 1/1 is lethal (CR 704.5g), so the aimed Elf dies while the Cannon
/// it was aimed with stays standing.
#[test]
#[allow(clippy::too_many_lines)]
fn fodder_cannon_sacrifices_a_creature_for_four_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fodder_cannon()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, victim),
        (1, 1),
        "a printed 1/1 for four damage to kill"
    );

    // Eight Forests are the {4} the cast costs and the {4} the ability then
    // charges. The Elf is named as the printing kept back because it is the
    // creature this ability is about to sacrifice, and a mana creature tapped
    // for the cost would be a different board.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight tapped Forests and eight green, with the Elf still standing"
    );
    cast_with_floating(&mut engine, p0, fodder_cannon());
    pass_until(&mut engine, stack_is_empty);
    let cannon = on_battlefield(&engine, p0, fodder_cannon()).expect("the Cannon resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the cast's {{4}} is spent and exactly the {{4}} the ability charges is left"
    );
    assert!(!is_tapped(&engine, cannon), "an artifact enters untapped");
    assert!(
        !types(&engine, cannon).contains(TypeSet::CREATURE),
        "the artifact is no creature, so its own filter can never name it"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool, so the claim about the offer is made with the four mana
    // already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the seat holds a quiet main phase, got {:?}",
            engine.pending()
        );
    };
    assert!(
        legal.abilities.contains(&(cannon, 0)),
        "with {{4}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, fodder_cannon(), 0);

    // The two questions one activation asks: which creature is aimed at
    // (CR 601.2c) and which creature is being given up (CR 601.2h). Answered
    // in whichever order they arrive, and each menu is read on the spot.
    let mut target_menu: Vec<ObjectId> = Vec::new();
    let mut sacrifice_menu: Vec<ObjectId> = Vec::new();
    for _ in 0..8 {
        if !target_menu.is_empty() && !sacrifice_menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "one creature, and the ability asks once"
                );
                target_menu = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![victim],
                        },
                    )
                    .expect("the Elf across the table was one of the options");
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
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
                sacrifice_menu = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the creature the question offered pays the cost");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Cannon's activation resolves: {other:?}"),
        }
    }
    assert!(
        target_menu.contains(&fodder) && target_menu.contains(&victim),
        "\"target creature\" is any creature, on either side of the table: {target_menu:?}"
    );
    assert!(
        !target_menu.contains(&cannon),
        "the Cannon is an artifact and no creature: {target_menu:?}"
    );
    assert_eq!(
        sacrifice_menu,
        vec![fodder],
        "the one creature this seat controls is the whole menu — the Elf across \
         the table is the same card and is not mine to give up (CR 701.21a)"
    );

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and has left the battlefield, which is the sacrifice itself"
    );
    assert!(
        is_tapped(&engine, cannon),
        "{{T}} is the other half of the price, paid by the Cannon itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{4}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the target is still standing while the ability waits to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "four damage on a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and the Elf the ability named left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, fodder_cannon()).is_some(),
        "the Cannon is neither sacrificed nor destroyed: its price was the \
         creature, not the artifact"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage went to the creature that was named, not the seat that \
         activated the ability"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and never to the seat whose board it stood on"
    );
}
