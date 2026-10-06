//! `cards/creatures/mv_4/gemrazer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gemrazer ({3}{G}) is `Coverage::Partial`: only two of its three printed
/// lines exist in the DSL, so what can be played is the body — a 4/4 Beast
/// with reach and trample — while mutate and the mutate trigger are the gap.
/// The scenario casts it off four Forests and then reads the *projected*
/// characteristics, which is the only reading that can see a keyword at all,
/// so it says the two keywords on the card are the two the creature on the
/// battlefield actually has. The 4/4 body is asserted beside them because
/// reach and trample on a permanent that arrived as the wrong creature would
/// be the same passing test — and the two Forests beyond the cost are there
/// so the {3} is not quietly paid by a land the test never counted.
#[test]
fn gemrazer_arrives_as_a_four_four_beast_with_reach_and_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[gemrazer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        in_hand(&engine, p0, gemrazer()).is_some(),
        "the card is in hand before anything is cast"
    );
    cast_from_hand(&mut engine, p0, gemrazer());
    pass_until(&mut engine, stack_is_empty);

    let beast = on_battlefield(&engine, p0, gemrazer()).expect("Gemrazer resolved");
    assert!(
        in_hand(&engine, p0, gemrazer()).is_none(),
        "and it left the hand to get there"
    );
    assert!(
        types(&engine, beast).contains(TypeSet::CREATURE),
        "it is a creature permanent: {:?}",
        types(&engine, beast)
    );
    assert_eq!(pt(&engine, beast), (4, 4), "the body the card prints");
    let kw = keywords(&engine, beast);
    assert!(kw.contains(KeywordSet::REACH), "Gemrazer has reach");
    assert!(kw.contains(KeywordSet::TRAMPLE), "Gemrazer has trample");
}
