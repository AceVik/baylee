//! `cards/creatures/mv_7/shaleskin_bruiser.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shaleskin Bruiser: the trample it keeps, and the pump it refuses by name.
///
/// `Coverage::Partial` here is about an `Amount` that multiplies a count, so
/// what a game can show is the 4/4 body with trample — and, on a board with
/// two other attacking Beasts, that it is still a 4/4 afterwards. The second
/// half is the honest half of a refusal: a card that quietly pumped by one
/// per Beast instead of three would read as "nearly right" and is the thing
/// the `NOT SUPPORTED` note says was refused rather than approximated.
#[test]
fn shaleskin_bruiser_tramples_and_does_not_grow_beside_other_beasts() {
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

    assert_eq!(
        pt(&engine, beasts[0]),
        (4, 4),
        "the attack trigger is off the card, so a 4/4 attacks as a 4/4 — and \
         an approximation of +1/+0 per Beast would show up here as a 5/4"
    );
}
