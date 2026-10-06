//! `cards/instants/mv_4/fact_or_fiction.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fact or Fiction: "Reveal the top five cards of your library. An opponent
/// separates those cards into two piles. Put one pile into your hand and the
/// other into your graveyard."
///
/// The opponent puts Counterspell alone. The caster is asked which pile, by
/// position, is refused a pile that is not there, and takes the four: they
/// go to the hand, Counterspell to the graveyard beside the spell.
#[test]
fn fact_or_fiction_takes_the_pile_its_caster_chooses() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, revealed) = fact_or_fiction_revealed(41);
    let alone = revealed[1];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![alone],
            },
        )
        .unwrap();
    let Pending::ChoosePile { player, piles } = engine.pending().clone() else {
        panic!(
            "expected the caster's pile choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "\"put one pile into your hand\": the caster picks"
    );
    let rest: Vec<ObjectId> = revealed.iter().copied().filter(|c| *c != alone).collect();
    assert_eq!(piles, vec![vec![alone], rest.clone()]);
    assert!(
        engine.apply(p0, PlayerAction::ChooseMode(2)).is_err(),
        "two piles, so there is no third to take"
    );

    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    for card in &rest {
        assert_eq!(
            engine.state().object(*card).map(|o| o.zone),
            Some(crate::zone::Zone::Hand),
            "the chosen pile went to the hand"
        );
    }
    assert_eq!(
        engine.state().object(alone).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard),
        "the other pile went to the graveyard"
    );
    assert!(in_graveyard(&engine, p0, fact_or_fiction()).is_some());
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 4);
}
