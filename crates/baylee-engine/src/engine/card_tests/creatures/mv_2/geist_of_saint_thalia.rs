//! `cards/creatures/mv_2/geist_of_saint_thalia.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Geist of Saint Thalia — {1}{U} — Legendary Creature — Spirit Cleric, a
/// 1/2 with flying. `Coverage::Partial`: only the keyword is implemented and
/// the cost-reduction sentence is the documented gap, so playing the card is
/// the only way to prove the flying arrives through the layer projection
/// rather than merely being printed on the definition. Two Islands are
/// exactly {U}{U}, so the cast also shows the printed {1}{U} is what was
/// paid and nothing else was needed.
#[test]
fn geist_of_saint_thalia_lands_as_a_flying_body_on_its_printed_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[geist_of_saint_thalia()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, geist_of_saint_thalia());
    pass_until(&mut engine, stack_is_empty);

    let geist = on_battlefield(&engine, p0, geist_of_saint_thalia()).expect("the Geist resolved");
    assert!(
        types(&engine, geist).contains(TypeSet::CREATURE),
        "it landed as a creature",
    );
    assert_eq!(pt(&engine, geist), (1, 2), "the printed 1/2");
    assert!(
        keywords(&engine, geist).contains(KeywordSet::FLYING),
        "flying — the clause of the card that exists",
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}} came out of the two Islands and nothing was left over",
    );
}
