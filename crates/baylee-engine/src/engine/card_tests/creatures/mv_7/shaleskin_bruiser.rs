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
