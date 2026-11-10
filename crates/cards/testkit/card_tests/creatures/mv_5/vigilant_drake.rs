//! `cards/creatures/mv_5/vigilant_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vigilant Drake — {4}{U}, a 3/3 Drake with flying and "{2}{U}: Untap this
/// creature."
///
/// A printed untap is worth nothing on a standing creature, so the scenario
/// makes the Drake attack first: the declaration is what turns it sideways
/// (CR 508.1f) and the 3 damage to p1 is the receipt that the body the card
/// prints and its flying are really on the battlefield. The end step is the
/// first priority round the active player is guaranteed to open after the
/// damage step (CR 510.2, CR 117.3a) and the Drake is still tapped there, so
/// the {2}{U} buys the untap its own turn would not give — read as exactly
/// three blue out of a pool the eight Islands actually filled.
#[test]
fn vigilant_drake_attacks_as_a_flier_and_pays_two_and_a_blue_to_untap_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 8])
        .hand(0, &[vigilant_drake()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // {4}{U} out of five of the eight Islands, claimed where the engine reads
    // it: with the mana already floating the Drake is a castable card.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Islands and no other permanent: eight blue"
    );
    cast_with_floating(&mut engine, p0, vigilant_drake());
    pass_until(&mut engine, stack_is_empty);

    let drake = on_battlefield(&engine, p0, vigilant_drake()).expect("the Drake resolved");
    assert_eq!(pt(&engine, drake), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "and the printed flying through the layers"
    );
    assert!(!is_tapped(&engine, drake), "a creature arrives standing");

    // The only creature on the board attacks, and the declaration is what
    // leaves it tapped. `pass_until` walks whatever turns stand in the way,
    // so the offer it stops on has to be the one that names the Drake.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { attackers, .. } if attackers.contains(&drake)),
    );
    let Pending::ChooseAttackers { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the declaration it asked for")
    };
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(drake, Defender::Player(p1))],
            },
        )
        .expect("the Drake came out of the list that offered it");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "3 flying damage got past a board with nothing on it to block"
    );
    assert!(
        is_tapped(&engine, drake),
        "an attacker is turned sideways and nothing has stood it back up"
    );

    // The eight Islands untapped in the meantime, so the printed price is
    // payable in full while the Drake is still down.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the Islands untapped and nothing was tapped on the way here"
    );
    activate(&mut engine, p0, vigilant_drake(), 0);
    assert!(
        is_tapped(&engine, drake),
        "untapping is an effect and no mana ability, so it waits on the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the {{2}}{{U}} came out of the pool as the price of the activation"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the ability is what is waiting there"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, drake),
        "\"{{2}}{{U}}: Untap this creature\" — the ability buys the untap the \
         turn itself would not give"
    );
}
