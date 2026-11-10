//! `cards/creatures/mv_2/zephyr_falcon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zephyr Falcon — {1}{U} — 1/1 Bird with flying and vigilance.
///
/// Vigilance is not readable off the card file, only off a combat step: the
/// Falcon attacks and is still untapped afterwards, and a Llanowar Elves
/// attacks in the very same declaration beside it as the control — the Elf
/// prints no vigilance and is tapped, so an engine that simply never tapped
/// attackers could not pass both halves. The two damage that reached the
/// opponent are what say the declaration really happened rather than being
/// skipped, and the turn cycle before it is CR 302.6: a creature cast this
/// turn may not attack at all.
#[test]
fn zephyr_falcon_flies_and_attacks_without_tapping_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .hand(0, &[zephyr_falcon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{U} off the two Islands, with the Elf named as the printing kept
    // back: it is the control in the attack declaration below, and a mana
    // creature tapped for the cast could not attack afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands in the pool, and the Elf kept back"
    );
    cast_with_floating(&mut engine, p0, zephyr_falcon());
    pass_until(&mut engine, stack_is_empty);

    let falcon = on_battlefield(&engine, p0, zephyr_falcon()).expect("the Falcon resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the control is out");
    assert_eq!(pt(&engine, falcon), (1, 1), "the body the card prints");
    let kw = keywords(&engine, falcon);
    assert!(kw.contains(KeywordSet::FLYING), "flying");
    assert!(kw.contains(KeywordSet::VIGILANCE), "vigilance");

    // A creature summoned this turn cannot attack (CR 302.6), so the attack
    // belongs to the turn after it arrived: one whole cycle of the table.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, falcon),
        "the Falcon's own untap step left it standing, so it may be declared"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&falcon) && attackers.contains(&elves),
        "both are untapped, unsick creatures of mine: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (falcon, Defender::Player(p1)),
                    (elves, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: that is already true the moment attackers are
    // declared, so it would stop the walk before the combat damage step and
    // both life totals would still read 20. The end step is past damage.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending) && e.state().turn.active == p0
    });

    assert_eq!(
        engine.state().players[1].life,
        18,
        "two 1/1s got through for two damage, so the attack was real"
    );
    assert!(
        is_tapped(&engine, elves),
        "the Elf prints no vigilance, and the same declaration tapped it"
    );
    assert!(
        !is_tapped(&engine, falcon),
        "while the Falcon is still untapped after attacking — that is what \
         vigilance does, and a board where nothing taps could not show it"
    );
}
