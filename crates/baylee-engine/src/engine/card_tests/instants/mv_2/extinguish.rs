//! `cards/instants/mv_2/extinguish.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Extinguish — {1}{U} instant: "Counter target sorcery spell."
///
/// Both words of the restriction are the engine's answer rather than the
/// card's, so a sorcery has to actually be on the stack: p0 casts Vindicate
/// at p1's Sol Ring, and the counter is then read off what becomes of both
/// cards. The target question is the first half — the spell p0 just cast is
/// on the menu, and the permanent that spell is aimed at is not — and the
/// resolution is the second: the sorcery lies in its owner's graveyard
/// without ever destroying anything, while the Sol Ring is still standing
/// and the stack is empty.
#[test]
fn extinguish_counters_the_sorcery_on_the_stack_and_leaves_its_target_standing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), swamp()])
        .hand(0, &[vindicate()])
        .battlefield(1, &[island(), island(), quiet_artifact()])
        .hand(1, &[extinguish()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A sorcery belongs in its caster's own main phase (CR 307.1), so this is
    // where the card under test gets something legal to counter.
    let victim = on_battlefield(&engine, p1, quiet_artifact()).expect("the Sol Ring stands");
    cast_from_hand(&mut engine, p0, vindicate());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Vindicate targets a permanent, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster names its own target");
    assert!(
        options.contains(&victim),
        "the Sol Ring is a permanent and so a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the permanent the cast offered was named");
    let spell = on_stack(&engine, vindicate()).expect("the sorcery is on the stack, unresolved");

    // The active player holds priority first after casting (CR 117.3c), so p1
    // has to be handed it before it can answer.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );

    // Two Islands and nothing else: the Sol Ring is the spell's target and is
    // left untapped, so the {1}{U} is paid entirely out of the lands.
    tap_all_mana_but(&mut engine, p1, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "two Islands, exactly the {{1}}{{U}} Extinguish charges"
    );
    cast_with_floating(&mut engine, p1, extinguish());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"counter target sorcery spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it chooses");
    assert!(
        options.contains(&spell),
        "the sorcery p0 just cast is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&victim),
        "the Sol Ring is a permanent and no spell on the stack: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the spell on the stack was one of the options");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_stack(&engine, vindicate()).is_none(),
        "the countered spell left the stack rather than resolving"
    );
    assert!(
        in_graveyard(&engine, p0, vindicate()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent Vindicate aimed at is untouched: the counter stopped \
         the effect and not merely the card"
    );
    assert!(
        in_graveyard(&engine, p1, extinguish()).is_some(),
        "and the instant that did it resolved into its caster's graveyard"
    );
}
