//! `cards/creatures/mv_4/lava_hounds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lava Hounds prints two sentences that pull in opposite directions: a 4/4
/// body with haste, and "When this creature enters, it deals 4 damage to you".
/// So one cast over four Mountains reads both — the life total has to fall by
/// exactly four on the *casting* seat and stay at twenty across the table
/// (`PlayerRel::You` is the controller, not the table), and the Hounds then has
/// to be offered as an attacker in the very combat step of the turn it arrived,
/// which is the printed haste and nothing else. Four damage of any other number
/// would leave the life total at a value this board cannot reach, and an
/// untapped 4/4 that is *not* in the offer would be a creature without haste.
#[test]
fn lava_hounds_burns_its_own_controller_for_four_and_still_attacks_at_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[lava_hounds()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{R}{R} off the four Mountains, and the entry trigger is answered by
    // whatever it asks on the way — it is a damage event, not a mana ability.
    cast_from_hand(&mut engine, p0, lava_hounds());
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the Hounds resolve and their enters-trigger resolves behind them"
    );

    let hounds = on_battlefield(&engine, p0, lava_hounds()).expect("the Hounds resolved");
    assert_eq!(pt(&engine, hounds), (4, 4), "the body the card prints");
    assert!(
        keywords(&engine, hounds).contains(KeywordSet::HASTE),
        "the printed haste reaches the permanent"
    );

    assert_eq!(
        engine.state().players[0].life,
        16,
        "\"When this creature enters, it deals 4 damage to you\" — four, off \
         the life of the seat that cast it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing of it reaches across the table: `You` is the controller"
    );

    // Haste played rather than read: the Hounds entered this turn, nobody
    // tapped it, and it is on the attack declaration of that same turn.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&hounds),
        "a creature with haste may attack the turn it entered, summoning \
         sickness or not (CR 302.6): {attackers:?}"
    );
}
