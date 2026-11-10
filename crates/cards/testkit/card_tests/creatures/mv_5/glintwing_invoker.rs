//! `cards/creatures/mv_5/glintwing_invoker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Glintwing Invoker prints one line — "{7}{U}: This creature gets +3/+3 and
/// gains flying until end of turn" — on a {4}{U} 3/3, and neither half of that
/// sentence is readable from the card file: `legal.abilities` is filtered
/// through `can_afford`, which reads the mana pool rather than the untapped
/// lands, so the offer is only worth claiming while the eight mana is really
/// floating. The ability's whole price is a mana cost and no tap, which is why
/// a 3/3 that arrived this turn may already pay it (CR 302.6 constrains `{T}`),
/// and `(6, 6)` plus flying is the only projected state that applies both the
/// pump and the keyword — a `(6, 6)` without flying would mean the granted
/// `KeywordSet` rode along unread. Walking a whole turn afterwards is the
/// control for the printed duration: the same creature is still standing, at
/// the 3/3 it prints and no longer flying.
#[test]
fn glintwing_invoker_pays_seven_and_a_blue_for_three_three_and_flying_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 13])
        .hand(0, &[glintwing_invoker()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Thirteen Islands into the pool first: the {4}{U} the creature costs and
    // the {7}{U} its ability charges are one payment, and CR 500.5 keeps what
    // is left in the pool because the whole scenario stays inside this one
    // main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        13,
        "thirteen Islands tapped, thirteen blue"
    );
    cast_with_floating(&mut engine, p0, glintwing_invoker());
    pass_until(&mut engine, stack_is_empty);
    let invoker = on_battlefield(&engine, p0, glintwing_invoker()).expect("the Invoker resolved");
    assert_eq!(pt(&engine, invoker), (3, 3), "the body the card prints");
    assert!(
        !keywords(&engine, invoker).contains(KeywordSet::FLYING),
        "nothing has granted it a keyword yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cast's {{4}}{{U}} came out of the pool and exactly the \
         {{7}}{{U}} the ability charges is left"
    );

    // The whole price is mana and no tap, so the offer is read off the pool
    // and not off the untapped lands — and a creature that entered this turn
    // is not stopped by summoning sickness (CR 302.6 names `{T}` only).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(invoker, 0)),
        "with eight mana in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, glintwing_invoker(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{7}}{{U}} came out of the pool"
    );
    assert!(
        !at_rest(&engine, p0),
        "the activation is not already over: a pump is no mana ability \
         (CR 605.1), so it is either still asking for its target or sitting on \
         the stack, got {:?}",
        engine.pending()
    );
    let rest = drive_to_rest(&mut engine, p0);
    assert!(
        matches!(rest, Rest::Reached),
        "the Invoker's own pump asks nothing that cannot be answered: {rest:?}"
    );

    assert_eq!(
        pt(&engine, invoker),
        (6, 6),
        "\"gets +3/+3\" on the 3/3 the card prints"
    );
    assert!(
        keywords(&engine, invoker).contains(KeywordSet::FLYING),
        "and \"gains flying\" is the second half of the same ability: a \
         KeywordSet that was never merged into the projection would leave the \
         body above unchanged and still pass a pump-only reading"
    );

    // "until end of turn": a turn later the creature is still standing and is
    // the printed 3/3 again, so both halves of the grant were a duration.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, invoker),
        (3, 3),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        !keywords(&engine, invoker).contains(KeywordSet::FLYING),
        "and the keyword left with it"
    );
    assert!(
        on_battlefield(&engine, p0, glintwing_invoker()).is_some(),
        "the creature is still on the battlefield, so the pump left rather \
         than the creature"
    );
}
