//! `cards/instants/mv_2/jwari_disruption.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jwari Disruption // Jwari Ruins (`Coverage::Implemented`): "Counter target
/// spell unless its controller pays {1}." The back face enters tapped and
/// taps for {U}.
///
/// p1 casts a Dark Ritual; p0 holds an Island and answers with Jwari
/// Disruption. The tax question goes to p1 (the targeted spell's controller),
/// not to p0. When p1 declines, the Ritual is countered and ends up in p1's
/// graveyard. The mana pool is checked to confirm that declining means nothing
/// was spent — the Ritual itself never resolved either.
#[test]
fn jwari_disruption_counters_unless_its_controller_pays_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[jwari_disruption()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // p1 casts Dark Ritual. The pool is read here and again at the end,
    // because `cast_from_hand` taps *every* land: p1 has two Swamps and the
    // Ritual costs one, so a bare `== 0` at the end measures the harness'
    // leftover change rather than the spell. What the card is about is that
    // the pool does not *grow* by three.
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("Ritual is in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());
    let pool_before = engine.state().players[1].mana_pool.total();

    // p0 responds with Jwari Disruption.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p0, jwari_disruption());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Disruption asks for a target spell, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "their spell, their target choice");
    assert!(
        options.contains(&ritual),
        "the Dark Ritual on the stack is a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the Ritual is a legal target");

    // Both pass; Disruption resolves and the tax question goes to p1.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        unreachable!("pass_until stopped on the tax question")
    };
    assert_eq!(*mana, 1, "Jwari Disruption prints a {{1}} tax");
    assert_eq!(
        *player, p1,
        "\"unless its controller pays\" — the tax goes to the targeted spell's controller"
    );

    // p1 declines; the Ritual is countered.
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        in_graveyard(&engine, p0, jwari_disruption()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    // The tax went unpaid and the Ritual was countered, so it added no
    // {B}{B}{B} on the way through.
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        pool_before,
        "a countered Dark Ritual adds nothing to its controller's pool"
    );
}
