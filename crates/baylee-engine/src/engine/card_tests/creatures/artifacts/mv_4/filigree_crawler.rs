//! `cards/creatures/artifacts/mv_4/filigree_crawler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn filigree_crawler() -> CardIndex {
    card_index("f74095aa-232f-432e-a899-86e9463ab5be")
}

/// Filigree Crawler — {4} 2/2: "When this creature dies, create a 1/1
/// colorless Thopter artifact creature token with flying." A Lightning Bolt
/// kills it, and its controller is left with a 1/1 artifact creature token
/// that has flying.
#[test]
fn filigree_crawler_leaves_a_flying_thopter_behind() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[filigree_crawler(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let crawler = on_battlefield(&engine, p0, filigree_crawler()).expect("seated");
    assert_eq!(pt(&engine, crawler), (2, 2));
    let creatures_before = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == p0 && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .count();
    assert_eq!(creatures_before, 1, "only the Crawler");

    bolt_to_death(&mut engine, p0, crawler);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, filigree_crawler()).is_some());
    let thopters: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                let c = o.characteristics();
                o.controller == p0
                    && c.types.contains(TypeSet::CREATURE)
                    && c.types.contains(TypeSet::ARTIFACT)
            })
        })
        .collect();
    assert_eq!(thopters.len(), 1, "exactly one token");
    let thopter = thopters[0];
    assert_eq!(pt(&engine, thopter), (1, 1));
    assert!(keywords(&engine, thopter).contains(KeywordSet::FLYING));
    assert!(
        engine
            .state()
            .object(thopter)
            .expect("object")
            .card
            .is_none(),
        "it is a token and not a card"
    );
}
