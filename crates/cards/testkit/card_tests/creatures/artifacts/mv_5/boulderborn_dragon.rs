//! `cards/creatures/artifacts/mv_5/boulderborn_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn boulderborn_dragon() -> CardIndex {
    card_index("1654cab1-0697-4547-b1fe-2d0a61385199")
}

/// Boulderborn Dragon — flying, vigilance, and "Whenever this creature
/// attacks, surveil 1": the attack leaves it untapped and asks the surveil,
/// and sending the card to the graveyard moves it out of the library.
#[test]
fn boulderborn_dragon_surveils_one_when_it_attacks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[boulderborn_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dragon = on_battlefield(&engine, p0, boulderborn_dragon()).expect("seated");
    assert_eq!(pt(&engine, dragon), (3, 3));
    let kw = keywords(&engine, dragon);
    assert!(kw.contains(KeywordSet::FLYING) && kw.contains(KeywordSet::VIGILANCE));

    let library = library_size(&engine, p0);
    let graveyard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    attack_alone(&mut engine, p0, dragon, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, piles, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(cards.len(), 1, "surveil 1 looks at one card");
    assert_eq!(piles, surveil_piles(1));
    engine
        .apply(p0, look_answer(&cards, &cards))
        .expect("the card goes to the graveyard");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard + 1
    );
    assert!(
        !is_tapped(&engine, dragon),
        "vigilance: attacking did not tap it"
    );
}
