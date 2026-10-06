//! `cards/creatures/mv_5/ardent_militia.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "23625877-b6db-480c-8885-a62b7d0457df"

/// Ardent Militia — {4}{W} — Creature — Human Soldier, printed as a 2/5 with
/// "Vigilance". Vigilance is the whole card and it is exactly the kind of
/// characteristic that keeps reading correctly while nothing plays it — the
/// keyword can be projected, the body can be projected, and both would look
/// right on a creature that has never been in combat. So the scenario plays
/// the line: the Militia is cast for {4}{W}, the board walks a full round of
/// turns (a creature that arrived this turn cannot attack, CR 302.6), and it
/// is then declared as an attacker against the opponent. Two claims are read
/// off one combat — 2 damage reached the defending seat, and the Militia is
/// still untapped while the blocker step is open *and* after the damage, which
/// is the difference between a vigilant attacker and a tap symbol paid for.
#[test]
fn ardent_militia_attacks_without_tapping_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 5])
        .hand(0, &[ardent_militia()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ardent_militia());
    pass_until(&mut engine, stack_is_empty);
    let militia = on_battlefield(&engine, p0, ardent_militia()).expect("the Militia resolved");
    assert_eq!(pt(&engine, militia), (2, 5), "the body the card prints");
    assert!(
        types(&engine, militia).contains(TypeSet::CREATURE),
        "and what arrived is the creature the card prints"
    );
    assert!(
        keywords(&engine, militia).contains(KeywordSet::VIGILANCE),
        "the printed Vigilance reaches the permanent"
    );
    assert!(
        !is_tapped(&engine, militia),
        "and it enters untapped, so its first combat is a real decision"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{4}}{{W}} out of exactly five Plains leaves nothing floating"
    );

    // Summoning sickness (CR 302.6): the Militia arrived this turn, so it is
    // not offered as an attacker until it has begun a turn under this seat's
    // control. A full round of turns is what makes the declaration below legal
    // rather than refused.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, militia),
        "nothing tapped it on the way round, so a tapped attacker below would \
         be the attack's doing and not somebody else's"
    );

    attack_and_collect_blocks(&mut engine, militia, p1);
    assert!(
        !is_tapped(&engine, militia),
        "Vigilance: attacking does not tap the creature, so it is still standing \
         while the blocker step is open"
    );

    // p1 controls nothing, so there is nothing to block with and the two
    // damage land. `pass_until` declares the empty blocks itself; the walk
    // stops on the life total, which is the only way this combat can change it.
    pass_until(&mut engine, |e| e.state().players[1].life < 20);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the 2/5 dealt its power in combat damage (CR 510.2)"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the seat that was attacked, not to the attacker"
    );
    assert!(
        !is_tapped(&engine, militia),
        "after the damage too: the attack cost it no tap, where a paid {{T}} \
         would have left it down until this seat's next untap step"
    );
    assert!(
        on_battlefield(&engine, p0, ardent_militia()).is_some(),
        "and it is still on the battlefield, untapped and able to block"
    );
}
