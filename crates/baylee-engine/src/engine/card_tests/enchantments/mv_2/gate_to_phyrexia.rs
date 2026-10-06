//! `cards/enchantments/mv_2/gate_to_phyrexia.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gate to Phyrexia — {B}{B} — "Sacrifice a creature: Destroy target
/// artifact. Activate only during your upkeep and only once each turn."
///
/// Three restrictions on one line, each read in its own window. The main
/// phase and the opponent's upkeep refuse it (CR 602.5d — both "your" and
/// "upkeep" are the card's words); the first activation names its target
/// before its sacrifice (CR 601.2c then 601.2h) and spares the creature it
/// did not name; the same-turn second is refused with a creature and an
/// artifact still standing, so the refusal is `PerTurn(1)` and not an empty
/// menu; and a later upkeep offers it again.
#[allow(clippy::too_many_lines)] // four windows on one printed sentence
#[test]
fn gate_to_phyrexia_activates_only_in_its_controllers_upkeep_and_once_a_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[gate_to_phyrexia(), llanowar_elves(), rib_cage_spider()],
        )
        .battlefield(1, &[quiet_artifact(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gate = on_battlefield(&engine, p0, gate_to_phyrexia()).expect("the Gate is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    let spider = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider is seated");
    let artifacts = all_on_battlefield(&engine, p1, quiet_artifact());
    assert_eq!(artifacts.len(), 2, "two artifacts are seated");

    // Main phase: the card says upkeep, and the engine refuses it by address.
    assert!(
        !priority_offer(&engine).abilities.contains(&(gate, 0)),
        "\"Activate only during your upkeep\": not in the main phase"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: gate,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the window is the engine's rule, not only the offer's"
    );

    // The next upkeep is p0's own.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        priority_offer(&engine).abilities.contains(&(gate, 0)),
        "offered in its controller's own upkeep"
    );

    activate(&mut engine, p0, gate_to_phyrexia(), 0);
    let menu = aim_at(&mut engine, p0, artifacts[0]);
    assert!(
        menu.contains(&artifacts[0]) && menu.contains(&artifacts[1]),
        "\"target artifact\" names no controller: both are on the menu"
    );
    assert!(
        !menu.contains(&gate),
        "the Gate is an enchantment and no artifact: {menu:?}"
    );

    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice cost asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&elf) && options.contains(&spider),
        "your own creatures are the menu: {options:?}"
    );
    assert_eq!(options.len(), 2, "and nothing across the table");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered pays the cost");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the sacrifice cost was paid"
    );
    assert!(
        on_battlefield(&engine, p0, rib_cage_spider()).is_some(),
        "the creature the cost did not name stays"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the named artifact was destroyed"
    );
    assert_eq!(
        all_on_battlefield(&engine, p1, quiet_artifact()).len(),
        1,
        "and only the named one"
    );

    // Once each turn: a creature and an artifact are both still there, so
    // the empty offer is `PerTurn(1)` and not an unpayable cost.
    assert!(
        on_battlefield(&engine, p0, rib_cage_spider()).is_some()
            && all_on_battlefield(&engine, p1, quiet_artifact()).len() == 1,
        "the second activation has a creature to spend and an artifact to aim at"
    );
    assert!(
        !priority_offer(&engine).abilities.contains(&(gate, 0)),
        "\"only once each turn\": refused in the same upkeep"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: gate,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the limit is the engine's rule, not only the offer's"
    );

    // "Your upkeep" is the controller's: the opponent's is not a window.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        !priority_offer(&engine).abilities.contains(&(gate, 0)),
        "not offered in the opponent's upkeep"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: gate,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the window is the engine's rule, not only the offer's"
    );

    // A new turn resets `PerTurn(1)`: the ability works again.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        priority_offer(&engine).abilities.contains(&(gate, 0)),
        "a later turn offers it again"
    );
    activate(&mut engine, p0, gate_to_phyrexia(), 0);
    aim_at(&mut engine, p0, artifacts[1]);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the sacrifice cost asks again, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&spider),
        "the Spider is the only creature left to spend: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spider],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(in_graveyard(&engine, p0, rib_cage_spider()).is_some());
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the last artifact is gone"
    );
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());
}
