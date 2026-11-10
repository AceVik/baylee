//! `cards/creatures/mv_2/nether_shadow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nether Shadow — {B}{B} 1/1 Spirit with haste. Haste (CR 702.10b) is the one permission a
/// summoning-sick creature (CR 302.6) otherwise lacks: cast this turn and
/// walked straight to combat, it may still attack, while an ordinary
/// Llanowar Elves cast beside it off the same mana may not.
#[test]
fn nether_shadow_attacks_the_turn_it_is_cast_while_the_elf_beside_it_may_not() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), forest()])
        .hand(0, &[nether_shadow(), llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, nether_shadow());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let shadow = on_battlefield(&engine, p0, nether_shadow()).expect("resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("resolved");
    assert_eq!(pt(&engine, shadow), (1, 1), "the body the card prints");
    assert!(
        keywords(&engine, shadow).contains(KeywordSet::HASTE),
        "\"Haste\" is the printed line"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::HASTE),
        "and the Elf beside it, cast the same turn, has none"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the attack declaration")
    };
    assert!(
        attackers.contains(&shadow),
        "haste: still offered the very turn it arrived: {attackers:?}"
    );
    assert!(
        !attackers.contains(&elf),
        "the Elf arrived the same turn and has no such permission: {attackers:?}"
    );

    let defender = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shadow, defender)],
            },
        )
        .expect("haste lets it attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocker was offered");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the attack connected: haste was real permission, not just an offer"
    );
}
