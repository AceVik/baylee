//! `cards/creatures/mv_7/shaleskin_bruiser.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn shaleskin_bruiser_counts_other_attacking_beasts_once_on_resolution() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(393, forest())
        .battlefield(0, &[shaleskin_bruiser(), shaleskin_bruiser()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);

    let board: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    let beasts: Vec<ObjectId> = board
        .into_iter()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == shaleskin_bruiser()))
        })
        .collect();
    assert_eq!(
        beasts.len(),
        2,
        "two Beasts, so \"each other\" is one of them"
    );
    assert!(
        keywords(&engine, beasts[0]).contains(KeywordSet::TRAMPLE),
        "trample is printed and kept"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: beasts
                    .iter()
                    .map(|id| (*id, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("both Beasts attack");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, beasts[0]), (7, 4));
    assert_eq!(pt(&engine, beasts[1]), (7, 4));
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(pt(&engine, beasts[0]), (4, 4));
}

/// "for each other attacking Beast": non-Beast attackers are no Beasts and a
/// Beast kept home is not attacking, so each attacking Bruiser counts exactly
/// the one other attacking Bruiser. Two attacking Bears, not one, so that
/// counting them (or every attacker) cannot land on the same total.
#[test]
fn shaleskin_bruiser_ignores_non_beast_attackers_and_a_beast_kept_home() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(393, forest())
        .battlefield(
            0,
            &[
                shaleskin_bruiser(),
                shaleskin_bruiser(),
                shaleskin_bruiser(),
                grizzly_bears(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);

    let of = |engine: &Engine<RegistryLookup>, card: CardIndex| -> Vec<ObjectId> {
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
            })
            .collect()
    };
    let beasts = of(&engine, shaleskin_bruiser());
    let bears = of(&engine, grizzly_bears());
    assert_eq!(
        (beasts.len(), bears.len()),
        (3, 2),
        "three Beasts, two Bears"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: [beasts[0], beasts[1], bears[0], bears[1]]
                    .into_iter()
                    .map(|id| (id, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("two Beasts and both Bears attack, the third Beast stays home");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, beasts[0]),
        (7, 4),
        "+3/+0 for the one other attacking Beast; the attacking Bears are no Beasts and the idle Beast is not attacking"
    );
    assert_eq!(
        pt(&engine, beasts[1]),
        (7, 4),
        "the second attacking Beast counts the same single other attacking Beast"
    );
    assert_eq!(
        pt(&engine, beasts[2]),
        (4, 4),
        "a Beast that did not attack has no attack trigger"
    );
    for bear in bears {
        assert_eq!(pt(&engine, bear), (2, 2), "the Bears are untouched");
    }
}
