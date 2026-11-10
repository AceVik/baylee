//! Oracle checklist: Lancer itself gains double strike while its controller has a Dinosaur; an opponent's Dinosaur does not satisfy the condition; losing own Dinosaur removes double strike.
#[allow(clippy::wildcard_imports)]
use super::*;
fn imperial_lancer() -> CardIndex {
    card_index("c0018eb9-6aae-4188-8f36-ab5fc69f5903")
}
fn dinosaur() -> CardIndex {
    baylee_core::generated::index::QUAKESTRIDER_CERATOPS
}
#[test]
fn only_a_controlled_dinosaur_grants_double_strike() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[imperial_lancer()])
        .battlefield(1, &[dinosaur()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let lancer = on_battlefield(&e, p0, imperial_lancer()).unwrap();
    assert!(
        !keywords(&e, lancer).contains(KeywordSet::DOUBLE_STRIKE),
        "opponent's Dinosaur does not satisfy your condition"
    );
    let mut own = Duel::new(SEED, forest())
        .battlefield(0, &[imperial_lancer(), dinosaur()])
        .battlefield(1, &[dinosaur(), plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut own);
    reach_main_phase(&mut own, p0);
    let lancer = on_battlefield(&own, p0, imperial_lancer()).unwrap();
    assert!(keywords(&own, lancer).contains(KeywordSet::DOUBLE_STRIKE));
    let controlled_dinosaur = on_battlefield(&own, p0, dinosaur()).unwrap();
    let other_dinosaur = on_battlefield(&own, p1, dinosaur()).unwrap();
    assert!(!keywords(&own, other_dinosaur).contains(KeywordSet::DOUBLE_STRIKE));
    reach_their_main_phase(&mut own, p1);
    cast_from_hand(&mut own, p1, swords_to_plowshares());
    aim_at(&mut own, p1, controlled_dinosaur);
    pass_until(&mut own, stack_is_empty);
    assert!(on_battlefield(&own, p0, dinosaur()).is_none());
    assert!(on_battlefield(&own, p1, dinosaur()).is_some());
    assert!(!keywords(&own, lancer).contains(KeywordSet::DOUBLE_STRIKE));
}
