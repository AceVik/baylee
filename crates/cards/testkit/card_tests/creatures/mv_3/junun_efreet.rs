//! `cards/creatures/mv_3/junun_efreet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Junún Efreet — `{1}{B}{B}` 3/3 Efreet with flying: "At the beginning of
/// your upkeep, sacrifice this creature unless you pay {B}{B}."
///
/// Both answers are played against the card's own board. Paying: the
/// question names the coloured price (CR 118.12a), the window it opens is
/// for that price, the two Swamps make it while the window stands
/// (CR 605.3a) and the 3/3 flier is still there. Declining on the next
/// upkeep: the creature is sacrificed. The next upkeep is the second
/// question, which is why the first payment can only be read as "kept", not
/// as "the trigger is finished".
#[test]
fn junun_efreet_pays_bb_to_stay_and_is_sacrificed_when_the_payment_is_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[junun_efreet(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "\"at the beginning of *your* upkeep\"");
    assert_eq!(
        prompt,
        YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{B}{B}")
        },
        "the printed, coloured price"
    );
    assert_eq!(
        engine.payment_window(),
        None,
        "no window stands while the question does"
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            p0,
            baylee_core::mana::ManaPayment::Fixed(baylee_core::mana!("{B}{B}"))
        )),
        "the window is for the price the question named"
    );
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    let efreet = on_battlefield(&engine, p0, junun_efreet()).expect("paid, still standing");
    assert_eq!(pt(&engine, efreet), (3, 3), "the printed body");
    assert!(
        keywords(&engine, efreet).contains(KeywordSet::FLYING),
        "flying is the card's other printed line"
    );

    // Its controller's next turn: the same question, and this time the
    // answer is no.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        on_battlefield(&engine, p0, junun_efreet()).is_some(),
        "the upkeep it was paid for left it standing, so the same question is asked again"
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the sacrifice resolves and the seat is back at a quiet priority"
    );
    assert!(
        on_battlefield(&engine, p0, junun_efreet()).is_none(),
        "\"sacrifice this creature unless you pay\" — declined, it leaves"
    );
    assert!(
        in_graveyard(&engine, p0, junun_efreet()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}
