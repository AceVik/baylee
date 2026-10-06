//! `cards/creatures/mv_5/kiki_jiki_mirror_breaker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "{T}: Create a token that's a copy of target nonlegendary creature you
/// control, except it has haste. Sacrifice it at the beginning of the next
/// end step." The menu holds only the Thragtusk: not Kiki-Jiki (legendary),
/// not the opponent's creature. The copy enters as a Thragtusk does (5
/// life), has haste, and leaves at this turn's end step, leaving a Beast.
#[test]
fn kiki_jiki_copies_a_nonlegendary_creature_with_haste_until_the_end_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kiki_jiki_mirror_breaker(), thragtusk()])
        .battlefield(1, &[steadfast_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let kiki = on_battlefield(&engine, p0, kiki_jiki_mirror_breaker()).unwrap();
    assert!(keywords(&engine, kiki).contains(KeywordSet::HASTE));
    let tusk = on_battlefield(&engine, p0, thragtusk()).unwrap();
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();

    activate(&mut engine, p0, kiki_jiki_mirror_breaker(), 0);
    let menu = aim_at(&mut engine, p0, tusk);
    assert_eq!(menu, vec![tusk], "not Kiki-Jiki, not {guard:?}");
    pass_until(&mut engine, stack_is_empty);

    let copies: Vec<ObjectId> = tokens_of(&engine, p0);
    assert_eq!(copies.len(), 1, "one token");
    let copy = copies[0];
    assert_eq!(pt(&engine, copy), (5, 3), "a copy of Thragtusk");
    assert!(
        keywords(&engine, copy).contains(KeywordSet::HASTE),
        "except it has haste"
    );
    assert!(
        !keywords(&engine, tusk).contains(KeywordSet::HASTE),
        "the original keeps its own"
    );
    assert_eq!(
        engine.state().players[0].life,
        25,
        "the copy's own enters trigger"
    );
    assert_eq!(
        engine.state().delayed.len(),
        1,
        "a sacrifice waits for the end step"
    );

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(
        engine
            .state()
            .object(copy)
            .is_none_or(|o| o.zone != crate::zone::Zone::Battlefield),
        "sacrificed at the beginning of the next end step"
    );
    let left = tokens_of(&engine, p0);
    assert_eq!(left.len(), 1, "the copy's leaves trigger made a Beast");
    assert_eq!(pt(&engine, left[0]), (3, 3));
    assert!(
        on_battlefield(&engine, p0, thragtusk()).is_some(),
        "the original stays"
    );
    assert!(engine.state().delayed.is_empty());
}

/// The delayed sacrifice names that one token: a copy that has already left
/// by the end step is not replaced by anything else there, and the original
/// is never what is sacrificed.
#[test]
fn kiki_jiki_sacrifices_nothing_else_when_the_copy_is_already_gone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kiki_jiki_mirror_breaker(), steadfast_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p0, steadfast_guard()).unwrap();
    activate(&mut engine, p0, kiki_jiki_mirror_breaker(), 0);
    aim_at(&mut engine, p0, guard);
    pass_until(&mut engine, stack_is_empty);
    let copy = tokens_of(&engine, p0)[0];
    kill(&mut engine, copy);
    assert!(tokens_of(&engine, p0).is_empty());
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(on_battlefield(&engine, p0, steadfast_guard()).is_some());
    assert!(on_battlefield(&engine, p0, kiki_jiki_mirror_breaker()).is_some());
    assert!(
        engine.state().delayed.is_empty(),
        "spent, not kept for a later turn"
    );
}
