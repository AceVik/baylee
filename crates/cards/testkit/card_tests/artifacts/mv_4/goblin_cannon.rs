//! `cards/artifacts/mv_4/goblin_cannon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Cannon — {4} artifact: "{2}: This artifact deals 1 damage to any
/// target. Sacrifice this artifact."
///
/// The printed line is two sentences inside one ability and the difference is
/// what this scenario reads: the {2} is the *cost* (CR 601.2h) while the
/// sacrifice is the effect's own last act, so the Cannon is still standing on
/// the battlefield with the mana still in the pool while the target question
/// is open. Six Forests pay the {4} and leave exactly the {2} the ability
/// charges, so "the pool is empty afterwards" is a statement about the printed
/// price and not about a board that never held the mana. "Any target"
/// (CR 115.4) is where the two option lists meet: the Elf across the table is
/// offered as an object, both seats as players, and the creature nobody named
/// is untouched afterwards.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn goblin_cannon_pays_two_mana_to_shoot_any_target_and_then_eats_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 6])
        .hand(0, &[goblin_cannon()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Six Forests into the pool: the {4} brings the artifact to the table and
    // the {2} the ability charges is what is left floating beside it — a pool
    // survives until the step ends (CR 500.5) and this whole scenario plays
    // inside this one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests tapped, six green"
    );
    cast_with_floating(&mut engine, p0, goblin_cannon());
    pass_until(&mut engine, stack_is_empty);
    let cannon = on_battlefield(&engine, p0, goblin_cannon()).expect("the Cannon resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(!is_tapped(&engine, cannon), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the cast's {{4}} is spent and exactly the {{2}} the ability charges is left"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool and not the untapped lands — which is why the claim about the
    // offer is made with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cannon, 0)),
        "with {{2}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, goblin_cannon(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&elf),
        "CR 115.4: \"any target\" counts creatures in the same choice: {options:?}"
    );
    assert!(
        !options.contains(&cannon),
        "the Cannon is an artifact and no creature, so it is no target for its \
         own ability: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and it counts players too, both of them: {player_options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards: the {2}
    // is a cost and is still floating, and the sacrifice is the effect's own
    // sentence, so it has not begun either.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the cost is the last step of the activation, so nothing is spent yet"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_cannon()).is_some(),
        "and the artifact is still on the battlefield: the sacrifice is not a cost"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent seat was one of the targets it enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_cannon()).is_some(),
        "the sacrifice is the effect's own sentence and happens when it resolves"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage\" to the seat that was named — one, and never a point \
         per mana spent"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the target, not to the seat that paid for it"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the creature nobody named never moved"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_cannon()).is_none(),
        "\"Sacrifice this artifact\" — it ate itself as the ability resolved"
    );
    assert!(
        in_graveyard(&engine, p0, goblin_cannon()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}
