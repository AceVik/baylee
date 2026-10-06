//! `cards/creatures/mv_1/plagued_rusalka.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plagued Rusalka — {B} 1/1 Spirit: "{B}, Sacrifice a creature: Target
/// creature gets -1/-1 until end of turn."
///
/// The price is two things at once and each needs its own reading: the {B}
/// out of a pool the Swamp alone filled, and a *creature* the question
/// offers — which CR 701.21a limits to what the sacrificing seat controls, so
/// the Elf across the table is the counter-half of that menu. The target is
/// any creature at all, and the reading that proves the pump happened is a
/// 1/1 dying: -1/-1 on a printed 1/1 is a 0/0, which CR 704.5f puts into its
/// owner's graveyard, while the Rusalka standing beside it is still a 1/1 —
/// so the effect reached its target rather than the table.
#[test]
#[allow(clippy::too_many_lines)] // one activation, both halves of the price and the result read off it
fn plagued_rusalka_sacrifices_a_creature_and_a_black_to_shrink_a_one_one_to_death() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(611, swamp())
        .battlefield(0, &[swamp(), plagued_rusalka(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // `walk_to_own_main` rather than `reach_main_phase`: the seed decides who
    // is on the play, and the other seat's combat step is not a question
    // `reach_main_phase` answers.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let rusalka = on_battlefield(&engine, p0, plagued_rusalka()).expect("the Rusalka is out");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let prey = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, prey), (1, 1), "the prey is a printed 1/1");

    // The Swamp alone, with the Elves named as the thing kept back: the pool
    // is what `can_afford` reads, and a second untapped source would make
    // "exactly one black" unreadable rather than untrue.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one Swamp tapped, and nothing else on this board makes black"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(rusalka, 0)),
        "{{B}} is in the pool and a creature stands beside it, so the line \
         is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, plagued_rusalka(), 0);

    // CR 601.2c before CR 601.2h: the target is named while the price is
    // still unpaid — the Elf is on the battlefield, the black is in the pool
    // and the Rusalka is untapped. Both questions are answered in the order
    // they arrive rather than in the order they are expected.
    let mut aimed = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if aimed && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat chooses");
                assert!(
                    player_options.is_empty(),
                    "a creature is an object and no player: {player_options:?}"
                );
                assert!(
                    options.contains(&prey),
                    "\"target creature\" reaches across the table: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![prey],
                        },
                    )
                    .unwrap();
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
                assert_eq!(player, p0, "the activating seat pays");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to \
                     tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
                assert!(
                    options.contains(&fodder),
                    "the creature this seat controls is on the menu: {options:?}"
                );
                menu = options.clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Rusalka's ability resolves: {other:?}"),
        }
    }
    assert!(aimed, "\"target creature\" is a target choice");
    assert_eq!(
        menu.len(),
        2,
        "the two creatures this seat controls: {menu:?}"
    );
    assert!(
        menu.contains(&rusalka),
        "the Rusalka is a creature and may pay its own price: {menu:?}"
    );
    assert!(
        !menu.contains(&prey),
        "CR 701.21a: a seat sacrifices only what it controls: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none()
            && in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the sacrificed creature went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none()
            && in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "-1/-1 on a printed 1/1 is a 0/0, which CR 704.5f puts into the \
         graveyard of the seat that owned it"
    );
    assert!(
        on_battlefield(&engine, p0, plagued_rusalka()).is_some(),
        "the Rusalka is the source and not a cost it paid"
    );
    assert_eq!(
        pt(&engine, rusalka),
        (1, 1),
        "the pump reached its target and no other creature on the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{B}} was spent paying for it"
    );
}
