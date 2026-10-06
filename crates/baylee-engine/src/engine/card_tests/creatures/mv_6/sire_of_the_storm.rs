//! `cards/creatures/mv_6/sire_of_the_storm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sire of the Storm — {4}{U}{U} 3/3 Spirit with flying: "Whenever you cast a
/// Spirit or Arcane spell, you may draw a card."
///
/// Three copies and one Sol Ring sit in hand so that the trigger can be
/// questioned three times and answered both ways on one board. The first Sire
/// is a Spirit spell cast while nothing is watching — a card cannot watch its
/// own arrival, and its trigger is registered only once it is a permanent —
/// the Sol Ring is the type filter's control, and the third Sire is cast under
/// two Sires, so two triggers ask and both are declined. Three questions and
/// exactly one drawn card is the whole of "you may".
#[test]
#[allow(clippy::too_many_lines)]
fn sire_of_the_storm_draws_once_for_a_spirit_and_answers_its_own_may_either_way() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, island())
        .battlefield(0, &[island(); 19])
        .hand(
            0,
            &[
                sire_of_the_storm(),
                sire_of_the_storm(),
                sire_of_the_storm(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Nineteen Islands are the whole price of everything below — three Sires
    // at {4}{U}{U} and the rock at {1} — and the pool survives the whole test
    // because CR 500.5 empties it only when the step ends.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        19,
        "nineteen Islands, nineteen blue"
    );
    let library_before = library_size(&engine, p0);

    // (1) The first Sire is a Spirit spell with no Sire on the battlefield.
    cast_with_floating(&mut engine, p0, sire_of_the_storm());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let first = on_battlefield(&engine, p0, sire_of_the_storm()).expect("the Sire resolved");
    assert_eq!(pt(&engine, first), (3, 3), "the printed 3/3 body");
    assert!(
        keywords(&engine, first).contains(KeywordSet::FLYING),
        "and the printed flying reaches it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "nothing drew: a Spirit-spell trigger is registered only once its \
         source is on the battlefield, so it cannot watch its own arrival"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 13, "six paid");

    // (2) The control for the type filter: an artifact spell, watched by a
    // Sire that is very much on the battlefield.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the rock resolved"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "a Sol Ring is neither a Spirit nor an Arcane spell, so the Sire asked \
         nothing and nothing was drawn"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "the {{1}} came out of the pool"
    );

    // (3) The second Sire, cast with a Sire standing: the trigger asks before
    // it draws anything.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_with_floating(&mut engine, p0, sire_of_the_storm());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "\"you may\" is a question: the cast has left the hand and nothing has \
         been drawn while the question stands"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and the library is still the length it was"
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "answering yes draws exactly one card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the cast spent a card and the draw put one back"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        all_on_battlefield(&engine, p0, sire_of_the_storm()).len(),
        2,
        "both Sires are on the battlefield now"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 6, "six more");

    // (4) The third Sire is cast under two Sires, so the trigger fires once
    // per Sire and each copy asks for itself. Declining both is the other half
    // of the printed word.
    cast_with_floating(&mut engine, p0, sire_of_the_storm());
    for _ in 0..2 {
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::MayDo,
                    ..
                }
            )
        });
        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    }
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "three trigger questions and one card drawn in total: the two answers \
         of no drew nothing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the third cast spent a card and nothing came back for it"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, sire_of_the_storm()).len(),
        3,
        "and all three Sires resolved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three {{4}}{{U}}{{U}} and one {{1}} is nineteen mana, spent to the last"
    );
}
