//! `cards/creatures/mv_5/aclazotz_deepest_betrayal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Aclazotz, Deepest Betrayal` // `Temple of the Dead` (`Coverage::Partial`):
/// "Flying, lifelink. Whenever `Aclazotz` attacks, each opponent discards a card. For each opponent
/// who can't, you draw a card. Whenever an opponent discards a land card, create a 1/1 black Bat
/// creature token with flying. When `Aclazotz` dies, return it to the battlefield tapped and transformed
/// under its owner's control. // `{{T}}`: Add `{{B}}`. `{{2}}{{B}}`, `{{T}}`: Transform this land."
///
/// Under `Coverage::Partial`, the draw rider, bat token creation on land discard, and dies-return are omitted,
/// while `KeywordSet::FLYING`, `KeywordSet::LIFELINK`, and the attack-trigger `Effect::DiscardForPlayers` are implemented.
/// The test declares `Aclazotz` as an attacker against an opponent holding a card, answers the resulting
/// `Pending::ChooseCards` discard prompt, and confirms that combat damage deals 4 damage and gains 4 life via lifelink.
#[test]
fn aclazotz_attacks_to_cause_discard_and_gains_life_from_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(310, forest())
        .battlefield(0, &[aclazotz_deepest_betrayal()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bat =
        on_battlefield(&engine, p0, aclazotz_deepest_betrayal()).expect("Aclazotz on battlefield");
    let kw = keywords(&engine, bat);
    assert!(kw.contains(KeywordSet::FLYING), "Aclazotz has flying");
    assert!(kw.contains(KeywordSet::LIFELINK), "Aclazotz has lifelink");
    assert_eq!(pt(&engine, bat), (4, 4), "Aclazotz is a 4/4");

    // Advance to combat and declare Aclazotz as an attacker against p1.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bat, Defender::Player(p1))],
            },
        )
        .unwrap();

    // The attack trigger resolves: opponent must discard a card.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on ChooseCards");
    };
    assert_eq!(player, p1, "p1 is prompted to discard");
    assert_eq!((min, max), (1, 1), "p1 must discard one card");

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: the stack is already empty the moment attackers
    // are declared, so that predicate stops the walk *before* the combat
    // damage step and every life total still reads 20. The end step is past
    // damage (CR 510.2) and is what the assertion below needs.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    // Opponent discarded their card.
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "p1's card was discarded to their graveyard"
    );
    // Unblocked combat damage of 4 was dealt to p1.
    assert_eq!(
        engine.state().players[1].life,
        16,
        "p1 took 4 combat damage"
    );
    // Lifelink triggered and gained 4 life for p0.
    assert_eq!(
        engine.state().players[0].life,
        24,
        "p0 gained 4 life via lifelink"
    );
}
