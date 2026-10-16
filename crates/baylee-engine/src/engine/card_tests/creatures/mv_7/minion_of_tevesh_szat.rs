//! `cards/creatures/mv_7/minion_of_tevesh_szat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn minion_of_tevesh_szat() -> CardIndex {
    card_index("93fbf4a7-4283-4eec-afde-6780149b56df")
}

/// Minion of Tevesh Szat — a 4/4 for {4}{B}{B}{B}.
#[test]
fn minion_of_tevesh_szat_costs_seven_and_is_a_four_four() {
    let mut engine = table7(&lots(swamp(), 7), &[minion_of_tevesh_szat()], &[]);
    // Cast in the main phase of the first turn: the upkeep it would trigger
    // in is already behind us.
    let minion = cast_and_resolve(&mut engine, minion_of_tevesh_szat());
    assert_body(&engine, minion, (4, 4), KeywordSet::default());
}

/// A game in which seat 0 has `board` and the Minion's upkeep trigger of the
/// first turn is the pending question: the payment of {B}{B}.
#[track_caller]
fn at_the_upkeep_payment(board: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut all = vec![minion_of_tevesh_szat()];
    all.extend_from_slice(board);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &all)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayMana { .. },
                ..
            }
        ),
        "the upkeep asks for the payment, got {:?}",
        engine.pending()
    );
    engine
}

/// "At the beginning of your upkeep, this creature deals 2 damage to you
/// unless you pay {B}{B}." Declined, I take 2.
#[test]
fn minion_of_tevesh_szat_hurts_me_when_i_decline_to_pay() {
    let p0 = PlayerId::new(0);
    let mut engine = at_the_upkeep_payment(&[swamp(), swamp()], &[]);
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declined");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 0), 18);
}

/// Paid with two Swamps, nothing is dealt.
#[test]
fn minion_of_tevesh_szat_spares_me_when_i_pay() {
    let p0 = PlayerId::new(0);
    let mut engine = at_the_upkeep_payment(&[swamp(), swamp()], &[]);
    engine.apply(p0, PlayerAction::YesNo(true)).expect("pay");
    assert_eq!(tap_all_mana(&mut engine, p0), 2, "two Swamps, {{B}}{{B}}");
    engine.apply(p0, PlayerAction::PassPriority).expect("pay");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 0), 20, "paid: no damage");
}

/// "{T}: Target creature gets +3/-2 until end of turn": a 4/5 Elemental
/// becomes 7/3.
#[test]
fn minion_of_tevesh_szat_gives_plus_three_minus_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = at_the_upkeep_payment(&[], &[earth_elemental()]);
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declined");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && stack_is_empty(e)
    });
    let elemental = on_battlefield(&engine, p1, earth_elemental()).expect("out");
    let minion = on_battlefield(&engine, p0, minion_of_tevesh_szat()).expect("out");
    activate(&mut engine, p0, minion_of_tevesh_szat(), 1);
    let _ = aim_at(&mut engine, p0, elemental);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elemental), (7, 3));
    assert!(is_tapped(&engine, minion), "{{T}} is the cost");
}
