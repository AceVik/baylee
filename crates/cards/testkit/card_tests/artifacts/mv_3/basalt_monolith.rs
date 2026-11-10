//! `cards/artifacts/mv_3/basalt_monolith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Basalt Monolith ({3}): "This artifact doesn't untap during your untap
/// step. {T}: Add {C}{C}{C}. {3}: Untap this artifact."
///
/// **The three Forests are the test, not the scenery.** A permanent that is
/// still tapped after an untap step proves nothing on its own — an untap
/// step that never ran leaves everything tapped and passes. The Forests are
/// tapped in the same turn as the Monolith and have to come back in the same
/// step it does not, which is what tells a rule from a missing turn.
///
/// The rule is CR 502.3: the active player *determines* which of their
/// permanents untap, and "effects can keep one or more of a player's
/// permanents from untapping". What kind of effect that is, is CR 613.11 —
/// one that modifies a game rule rather than an object — so nothing about
/// the Monolith's characteristics changes and the untap step reads the
/// effect table instead.
///
/// The card's own way out is played too: `{3}` untaps it at instant speed,
/// and the artifact then taps for `{C}{C}{C}` again, which is the whole
/// printed card in one scenario.
#[test]
fn basalt_monolith_stays_tapped_while_the_lands_beside_it_untap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7731, forest())
        .battlefield(0, &[basalt_monolith(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let monolith =
        on_battlefield(&engine, p0, basalt_monolith()).expect("the Monolith is on the table");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(forests.len(), 3, "three Forests were dealt");

    // The three Forests, and the Monolith kept back: its {T} is the ability
    // this test presses by index, and `tap_all_mana` would have spent it.
    tap_all_mana_but(&mut engine, p0, Some(basalt_monolith()));
    // Index 1: the static ability is index 0 and takes no activation.
    activate(&mut engine, p0, basalt_monolith(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "{{C}}{{C}}{{C}} off the Monolith, and no stack: a mana ability \
         resolves as it is activated (CR 605.3b)"
    );
    assert!(is_tapped(&engine, monolith), "which tapped it");
    assert!(
        forests.iter().all(|id| is_tapped(&engine, *id)),
        "and the Forests are tapped in the same turn"
    );

    // Through the opponent's turn and back, because `walk_to_own_main`
    // answers "you are already there" from the main phase this started in.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Monolith's controller takes another turn"
    );
    assert!(
        forests.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: every Forest is back. Without this the \
         assertion below is satisfied by a game that never reached CR 502.3"
    );
    assert!(
        is_tapped(&engine, monolith),
        "and the Monolith alone stayed down — the printed sentence is an \
         effect that keeps a permanent from untapping (CR 502.3), not a \
         characteristic anything projects (CR 613.11)"
    );

    // The card's own way out, at index 2.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, basalt_monolith(), 2);
    assert!(
        !stack_is_empty(&engine),
        "untapping is no mana ability, so it uses the stack"
    );
    assert!(is_tapped(&engine, monolith), "and has not happened yet");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, monolith),
        "{{3}} buys the untap the untap step would not give"
    );

    activate(&mut engine, p0, basalt_monolith(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "and it taps for three again, which is what the card is for"
    );
}
