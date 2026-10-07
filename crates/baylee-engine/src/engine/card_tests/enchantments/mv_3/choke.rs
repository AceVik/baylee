//! `cards/enchantments/mv_3/choke.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Choke — {2}{G} enchantment: "Islands don't untap during their controllers'
/// untap steps."
///
/// The enchantment is cast, and `cast_from_hand` taps every source whose whole
/// price is its own `{T}` — so the caster's own Island is already down beside
/// the three Forests that paid when the printed sentence is about to matter. The
/// Forests are the bound on the claim: they come back at that untap step and the
/// Island does not, which is the difference between a rule (CR 502.3) and a game
/// that simply never moved on. p1's Island is tapped in its own main phase and
/// read one turn later, because the sentence says "controllers'" — it names a
/// land type and no controller, and a filter quietly narrowed to the caster's
/// permanents would satisfy the first half of this test and none of the second.
#[test]
fn choke_keeps_every_island_down_while_the_forests_beside_them_untap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), forest(), island()])
        .battlefield(1, &[island(), forest()])
        .hand(0, &[choke()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, choke());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, choke()).is_some(),
        "the enchantment resolved onto the battlefield"
    );

    let mine = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let my_forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(my_forests.len(), 3, "three Forests paid for the Choke");
    assert!(
        my_forests.iter().all(|id| is_tapped(&engine, *id)) && is_tapped(&engine, mine),
        "the whole board was tapped for the {{2}}{{G}}, the Island included — \
         which is the state both untap steps below are read from"
    );

    // p1's two lands, tapped in their own main phase so that their controller's
    // untap step has something to leave behind.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    let their_island = on_battlefield(&engine, p1, island()).expect("their Island is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert!(
        is_tapped(&engine, their_island) && is_tapped(&engine, their_forest),
        "both of p1's lands tapped for mana in their own main phase"
    );

    // p0's next untap step: every Forest stands back up, the Island does not.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        my_forests.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: every Forest came back. Without this the Island \
         below would be satisfied by a game that never reached CR 502.3"
    );
    assert!(
        is_tapped(&engine, mine),
        "\"Islands don't untap during their controllers' untap steps\" — the \
         caster's own Island stayed down (CR 502.3), an effect and not a \
         characteristic anything projects"
    );

    // And the same rule across the table, where a filter narrowed to the
    // caster's permanents would show nothing at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !is_tapped(&engine, their_forest),
        "p1's untap step ran too, and its Forest came back"
    );
    assert!(
        is_tapped(&engine, their_island),
        "the printed sentence names a land type and no controller, so p1's \
         Island is down for as long as the Choke is on the battlefield"
    );
}
