//! `cards/creatures/mv_4/roc_of_kher_ridges.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "01f07ab9-3c3e-4fea-9cd6-543a5b7056b8"

/// Roc of Kher Ridges — {3}{R} — Creature — Bird: "Flying."
///
/// The whole card is a cost, a body and one keyword, and each is read off a
/// different place: four Mountains are spent down to an empty pool, so the
/// {3}{R} is a real payment rather than a label; the permanent that lands is a
/// 3/3 on a creature type line of its own; and `flying` is read through the
/// layer projection (`keywords`), which is the only reading that can see a
/// keyword rather than its printing. Walking one turn further then shows the
/// card is a creature *in the game* and not a name on a board: the untap step
/// stands it back up and the next declare-attackers question offers it, which
/// is exactly what summoning sickness (CR 302.6) withheld on the turn it
/// arrived.
#[test]
fn roc_of_kher_ridges_lands_as_a_flying_three_three_off_four_mountains() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[roc_of_kher_ridges()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, roc_of_kher_ridges());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, roc_of_kher_ridges()).is_some()
    });

    let roc = on_battlefield(&engine, p0, roc_of_kher_ridges()).expect("the Roc resolved");
    assert!(
        in_hand(&engine, p0, roc_of_kher_ridges()).is_none(),
        "the card arrived out of the hand rather than being believed in"
    );
    assert_eq!(pt(&engine, roc), (3, 3), "the body the card prints");
    assert!(
        types(&engine, roc).contains(TypeSet::CREATURE),
        "and it is the creature its type line says"
    );
    assert!(
        keywords(&engine, roc).contains(KeywordSet::FLYING),
        "\"Flying\" reaches the permanent through the layers"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Mountains paid {{3}}{{R}} to the last mana"
    );

    // A turn later the Roc is no longer summoning sick: the untap step stood
    // it up, and the declare-attackers question is the one place a 3/3 flier
    // is worth anything in the rules at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 takes another turn with the Roc still on the battlefield"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&roc),
        "an untapped, unsick creature is offered as an attacker: {attackers:?}"
    );
}
