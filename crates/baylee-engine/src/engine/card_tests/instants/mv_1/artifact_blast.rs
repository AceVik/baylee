//! `cards/instants/mv_1/artifact_blast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Artifact Blast costs {R} and prints a single line: „Counter target
/// artifact spell." The scenario first casts a Sol Ring — an artifact
/// that comes onto the stack as a *spell* and never touches the battlefield —
/// and responds to it while the caster has priority (CR 601.2i);
/// „target artifact spell" does not ask who controls the spell. The
/// counterproof lies in the target list: the Sol Ring that is already on
/// the opponent's side has the same card name and still does not appear
/// on it, because the target is a spell and not a permanent. After
/// resolution the artifact card is in its owner's graveyard instead of on
/// the battlefield, and the Sol Ring already on the battlefield is untapped
/// and untouched — the counter hit the stack and not the battlefield.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn artifact_blast_counters_a_sol_ring_on_the_stack_and_never_touches_the_permanent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[quiet_artifact(), artifact_blast()])
        // The twin on the board: the same card name, but a permanent.
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 erreicht seinen Main");
    let standing =
        on_battlefield(&engine, p1, quiet_artifact()).expect("bei p1 liegt schon ein Sol Ring");

    // Mana before the assertion: `LegalActions` reads the pool and not the
    // two untapped Mountains.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, two red mana"
    );

    // The spell that is to be countered: {1} paid, and the card lies
    // afterwards on the stack — in none of the three zones in which a test
    // otherwise looks for it.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    let spell = on_stack(&engine, quiet_artifact()).expect("Sol Ring is a spell on the stack");
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "an artifact spell is on the stack until resolution"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the {{1}} is paid, the {{R}} for the counterspell is still floating"
    );

    // CR 601.2i: the caster gets priority back, so they can respond to
    // their own spell.
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("after casting, p0 holds priority: {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    cast_with_floating(&mut engine, p0, artifact_blast());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "„target artifact spell\" is a target choice: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat chooses the target");
    assert_eq!((min, max), (1, 1), "exactly one spell");
    assert!(
        options.contains(&spell),
        "the artifact spell on the stack is the target: {options:?}"
    );
    assert!(
        !options.contains(&standing),
        "ein Artefakt auf dem Schlachtfeld ist kein Spruch, auch wenn es \
         derselbe Kartenname ist: {options:?}"
    );
    assert_eq!(options.len(), 1, "und sonst liegt nichts auf dem Stapel");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the target that the question itself offered");
    // CR 601.2h: the costs are the last step of the announcement, so
    // the target question is still there while the mana is floating.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "the {{R}} is paid"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the counterspell is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "and never onto the battlefield"
    );
    assert_eq!(
        on_battlefield(&engine, p1, quiet_artifact()),
        Some(standing),
        "der Sol Ring, der schon lag, ist derselbe wie vorher: keiner ist \
         aufgetaucht, keiner verschwunden"
    );
    assert!(
        !is_tapped(&engine, standing),
        "and he was tapped for nothing — the counterspell never touched the board"
    );
    assert!(
        in_graveyard(&engine, p0, artifact_blast()).is_some(),
        "the counterspell itself is in the graveyard after its resolution"
    );
}
